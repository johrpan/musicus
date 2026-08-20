use std::cell::{Cell, OnceCell, RefCell};

use adw::{prelude::*, subclass::prelude::*};
use anyhow::Result;
use chrono::{Local, NaiveDateTime, TimeZone, Utc};
use gettextrs::gettext;
use gtk::{
    gio,
    glib::{self, clone, Properties},
};
use musicus_library::{
    db::{tables::Source, TranslatedString},
    format_translated, LibraryError,
};

use crate::{
    editor::{
        album::AlbumEditor, ensemble::EnsembleEditor, recording::RecordingEditor,
        simple_entity::SimpleEntityEditor, tag::TagEditor, work::WorkEditor,
    },
    library::Library,
    util,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrowserKind {
    Persons,
    Ensembles,
    Works,
    Recordings,
    Albums,
    Instruments,
    Roles,
    Tags,
}

impl BrowserKind {
    const ALL: [BrowserKind; 8] = [
        BrowserKind::Persons,
        BrowserKind::Ensembles,
        BrowserKind::Works,
        BrowserKind::Recordings,
        BrowserKind::Albums,
        BrowserKind::Instruments,
        BrowserKind::Roles,
        BrowserKind::Tags,
    ];

    fn title(self) -> String {
        match self {
            BrowserKind::Persons => gettext("Persons"),
            BrowserKind::Ensembles => gettext("Ensembles"),
            BrowserKind::Works => gettext("Works"),
            BrowserKind::Recordings => gettext("Recordings"),
            BrowserKind::Albums => gettext("Albums"),
            BrowserKind::Instruments => gettext("Instruments"),
            BrowserKind::Roles => gettext("Roles"),
            BrowserKind::Tags => gettext("Tags"),
        }
    }

    fn name_title(self) -> String {
        match self {
            BrowserKind::Recordings => gettext("Work"),
            _ => gettext("Name"),
        }
    }

    fn details_title(self) -> Option<(String, bool)> {
        match self {
            BrowserKind::Works => Some((gettext("Composers"), true)),
            BrowserKind::Recordings => Some((gettext("Performers"), true)),
            BrowserKind::Ensembles => Some((gettext("Members"), true)),
            BrowserKind::Tags => Some((gettext("Kind"), false)),
            _ => None,
        }
    }

    fn load(self, library: &Library) -> Result<Vec<EntityObject>> {
        Ok(match self {
            BrowserKind::Persons => library
                .list_persons()?
                .iter()
                .map(|person| {
                    EntityObject::new(
                        &person.person_id,
                        person.name.get(),
                        person.source,
                        person.last_used_at,
                    )
                })
                .collect(),
            BrowserKind::Ensembles => library
                .list_ensembles()?
                .iter()
                .map(|item| {
                    let object = EntityObject::new(
                        &item.ensemble.ensemble_id,
                        item.ensemble.name.get(),
                        item.ensemble.source,
                        item.ensemble.last_used_at,
                    );
                    object.set_details(join_names(&item.members));
                    object
                })
                .collect(),
            BrowserKind::Works => library
                .list_works()?
                .iter()
                .map(|item| {
                    let object = EntityObject::new(
                        &item.work.work_id,
                        item.work.name.get(),
                        item.work.source,
                        item.work.last_used_at,
                    );
                    object.set_details(join_names(&item.composers));
                    object
                })
                .collect(),
            BrowserKind::Recordings => library
                .list_recordings()?
                .iter()
                .map(|item| {
                    let object = EntityObject::new(
                        &item.recording.recording_id,
                        &match join_names(&item.composers) {
                            composers if composers.is_empty() => item.work_name.get().to_owned(),
                            composers => {
                                format!("{composers}: {}", item.work_name.get())
                            }
                        },
                        item.recording.source,
                        item.recording.last_used_at,
                    );
                    object.set_details(join_names(&item.performers));
                    object.set_n_tracks(item.n_tracks.max(0) as u32);
                    object
                })
                .collect(),
            BrowserKind::Albums => library
                .list_albums()?
                .iter()
                .map(|album| {
                    EntityObject::new(
                        &album.album_id,
                        album.name.get(),
                        album.source,
                        album.last_used_at,
                    )
                })
                .collect(),
            BrowserKind::Instruments => library
                .list_instruments()?
                .iter()
                .map(|instrument| {
                    EntityObject::new(
                        &instrument.instrument_id,
                        instrument.name.get(),
                        instrument.source,
                        instrument.last_used_at,
                    )
                })
                .collect(),
            BrowserKind::Roles => library
                .list_roles()?
                .iter()
                .map(|role| {
                    EntityObject::new(
                        &role.role_id,
                        role.name.get(),
                        role.source,
                        role.last_used_at,
                    )
                })
                .collect(),
            BrowserKind::Tags => library
                .list_tags()?
                .iter()
                .map(|tag| {
                    let object = EntityObject::new(
                        &tag.tag_id,
                        tag.name.get(),
                        tag.source,
                        tag.last_used_at,
                    );

                    let mut details = if tag.takes_value {
                        gettext("Takes a value")
                    } else {
                        gettext("Label")
                    };

                    if tag.private {
                        details.push_str(&format!(" · {}", gettext("Private")));
                    }

                    object.set_details(details);
                    object
                })
                .collect(),
        })
    }

    fn edit(
        self,
        navigation: &adw::NavigationView,
        library: &Library,
        id: &str,
    ) -> Result<adw::NavigationPage> {
        Ok(match self {
            BrowserKind::Persons => {
                SimpleEntityEditor::person(navigation, library, Some(&library.load_person(id)?))
                    .upcast()
            }
            BrowserKind::Instruments => SimpleEntityEditor::instrument(
                navigation,
                library,
                Some(&library.load_instrument(id)?),
            )
            .upcast(),
            BrowserKind::Roles => {
                SimpleEntityEditor::role(navigation, library, Some(&library.load_role(id)?))
                    .upcast()
            }
            BrowserKind::Tags => {
                TagEditor::new(navigation, library, Some(&library.load_tag(id)?)).upcast()
            }
            BrowserKind::Ensembles => {
                EnsembleEditor::new(navigation, library, Some(&library.load_ensemble(id)?)).upcast()
            }
            BrowserKind::Works => {
                WorkEditor::new(navigation, library, Some(&library.load_work(id)?), false).upcast()
            }
            BrowserKind::Recordings => {
                RecordingEditor::new(navigation, library, Some(&library.load_recording(id)?))
                    .upcast()
            }
            BrowserKind::Albums => {
                AlbumEditor::new(navigation, library, Some(&library.load_album(id)?)).upcast()
            }
        })
    }

    fn delete(self, library: &Library, id: &str) -> Result<()> {
        match self {
            BrowserKind::Persons => library.delete_person(id)?,
            BrowserKind::Instruments => library.delete_instrument(id)?,
            BrowserKind::Roles => library.delete_role(id)?,
            BrowserKind::Tags => library.delete_tag(id)?,
            BrowserKind::Ensembles => library.delete_ensemble(id)?,
            BrowserKind::Works => library.delete_work(id)?,
            BrowserKind::Recordings => library.delete_recording_and_tracks(id)?,
            BrowserKind::Albums => library.delete_album(id)?,
        }

        Ok(())
    }

    fn create(self, navigation: &adw::NavigationView, library: &Library) -> adw::NavigationPage {
        match self {
            BrowserKind::Persons => SimpleEntityEditor::person(navigation, library, None).upcast(),
            BrowserKind::Instruments => {
                SimpleEntityEditor::instrument(navigation, library, None).upcast()
            }
            BrowserKind::Roles => SimpleEntityEditor::role(navigation, library, None).upcast(),
            BrowserKind::Tags => TagEditor::new(navigation, library, None).upcast(),
            BrowserKind::Ensembles => EnsembleEditor::new(navigation, library, None).upcast(),
            BrowserKind::Works => WorkEditor::new(navigation, library, None, false).upcast(),
            BrowserKind::Albums => AlbumEditor::new(navigation, library, None).upcast(),
            BrowserKind::Recordings => RecordingEditor::new(navigation, library, None).upcast(),
        }
    }
}

fn source_label(source: Source) -> String {
    match source {
        Source::Metadata => gettext("Musicus"),
        Source::User => gettext("Created here"),
        Source::Import => gettext("Imported"),
    }
}

fn date_label(timestamp: NaiveDateTime) -> String {
    Utc.from_utc_datetime(&timestamp)
        .with_timezone(&Local)
        .format("%Y-%m-%d")
        .to_string()
}

mod entity_object_imp {
    use super::*;

    #[derive(Properties, Default, Debug)]
    #[properties(wrapper_type = super::EntityObject)]
    pub struct EntityObject {
        #[property(get, set)]
        pub id: RefCell<String>,
        #[property(get, set)]
        pub name: RefCell<String>,
        #[property(get, set)]
        pub n_tracks: Cell<u32>,
        #[property(get, set)]
        pub details: RefCell<String>,
        #[property(get, set)]
        pub source: RefCell<String>,
        #[property(get, set)]
        pub last_used: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EntityObject {
        const NAME: &'static str = "MusicusEntityObject";
        type Type = super::EntityObject;
    }

    #[glib::derived_properties]
    impl ObjectImpl for EntityObject {}
}

glib::wrapper! {
    pub struct EntityObject(ObjectSubclass<entity_object_imp::EntityObject>);
}

impl EntityObject {
    fn new(id: &str, name: &str, source: Source, last_used: NaiveDateTime) -> Self {
        glib::Object::builder()
            .property("id", id)
            .property("name", name)
            .property("source", source_label(source))
            .property("last-used", date_label(last_used))
            .build()
    }
}

fn join_names(names: &[TranslatedString]) -> String {
    names
        .iter()
        .map(|name| name.get())
        .collect::<Vec<_>>()
        .join(", ")
}

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/entity_browser.blp")]
    pub struct EntityBrowser {
        pub navigation: OnceCell<adw::NavigationView>,
        pub library: OnceCell<Library>,
        pub kind: Cell<usize>,
        pub items: OnceCell<gio::ListStore>,
        pub filter: OnceCell<gtk::CustomFilter>,
        pub selection: OnceCell<gtk::MultiSelection>,

        #[template_child]
        pub kind_drop_down: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub column_view: TemplateChild<gtk::ColumnView>,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub status_page: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub count_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub select_all_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub clear_selection_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub delete_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub new_button: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EntityBrowser {
        const NAME: &'static str = "MusicusEntityBrowser";
        type Type = super::EntityBrowser;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for EntityBrowser {}
    impl WidgetImpl for EntityBrowser {}
    impl NavigationPageImpl for EntityBrowser {}
}

glib::wrapper! {
    pub struct EntityBrowser(ObjectSubclass<imp::EntityBrowser>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

#[gtk::template_callbacks]
impl EntityBrowser {
    pub fn new(navigation: &adw::NavigationView, library: &Library) -> Self {
        let obj: Self = glib::Object::new();
        let imp = obj.imp();

        let _ = imp.navigation.set(navigation.to_owned());
        let _ = imp.library.set(library.to_owned());

        let items = gio::ListStore::new::<EntityObject>();
        let _ = imp.items.set(items.clone());

        let search_entry = imp.search_entry.get();
        let filter = gtk::CustomFilter::new(move |object| {
            let needle = search_entry.text().to_lowercase();

            if needle.is_empty() {
                return true;
            }

            let Some(item) = object.downcast_ref::<EntityObject>() else {
                return false;
            };

            item.name().to_lowercase().contains(&needle)
                || item.details().to_lowercase().contains(&needle)
        });
        let _ = imp.filter.set(filter.clone());

        let selection = gtk::MultiSelection::new(Some(gtk::SortListModel::new(
            Some(gtk::FilterListModel::new(Some(items), Some(filter))),
            imp.column_view.sorter(),
        )));

        imp.column_view.set_model(Some(&selection));
        let _ = imp.selection.set(selection.clone());

        selection.connect_selection_changed(clone!(
            #[weak]
            obj,
            move |_, _, _| obj.update_selection()
        ));

        selection.connect_items_changed(clone!(
            #[weak]
            obj,
            move |_, _, _, _| {
                obj.update_selection();
                obj.update_view_state();
            }
        ));

        imp.column_view.connect_activate(clone!(
            #[weak]
            obj,
            move |_, position| {
                if let Some(item) = obj
                    .selection()
                    .item(position)
                    .and_downcast::<EntityObject>()
                {
                    obj.edit(&item.id());
                }
            }
        ));

        let kinds = BrowserKind::ALL
            .iter()
            .map(|kind| kind.title())
            .collect::<Vec<_>>();
        imp.kind_drop_down.set_model(Some(&gtk::StringList::new(
            &kinds.iter().map(String::as_str).collect::<Vec<_>>(),
        )));

        imp.kind_drop_down.connect_selected_notify(clone!(
            #[weak]
            obj,
            move |drop_down| {
                obj.imp().kind.set(drop_down.selected() as usize);
                obj.kind_changed();
            }
        ));

        // The library manager's page survives a library change (Window::reset_view
        // only rebuilds search and album pages), so it has to refresh itself.
        library.connect_changed(clone!(
            #[weak]
            obj,
            move |_| obj.reload()
        ));

        obj.rebuild_columns();
        obj.reload();

        obj
    }

    fn navigation(&self) -> adw::NavigationView {
        self.imp()
            .navigation
            .get()
            .expect("the browser has a navigation view")
            .to_owned()
    }

    fn library(&self) -> Library {
        self.imp()
            .library
            .get()
            .expect("the browser has a library")
            .to_owned()
    }

    fn items(&self) -> gio::ListStore {
        self.imp()
            .items
            .get()
            .expect("the browser has a list store")
            .to_owned()
    }

    fn selection(&self) -> gtk::MultiSelection {
        self.imp()
            .selection
            .get()
            .expect("the browser has a selection model")
            .to_owned()
    }

    fn kind(&self) -> BrowserKind {
        BrowserKind::ALL[self.imp().kind.get()]
    }

    fn kind_changed(&self) {
        self.rebuild_columns();
        self.reload();
    }

    #[template_callback]
    fn search_changed(&self) {
        self.imp()
            .filter
            .get()
            .expect("the browser has a filter")
            .changed(gtk::FilterChange::Different);
    }

    #[template_callback]
    fn select_all(&self) {
        self.selection().select_all();
    }

    #[template_callback]
    fn clear_selection(&self) {
        self.selection().unselect_all();
    }

    /// Delete every selected item, after confirming once for the whole batch.
    #[template_callback]
    async fn delete_selected(&self) {
        let ids = self.selected_ids();

        if ids.is_empty() {
            return;
        }

        let dialog = adw::AlertDialog::builder()
            .heading(format_translated!(
                gettext("Delete {}?"),
                ids.len().to_string()
            ))
            .body(gettext(
                "This cannot be undone. Items still used elsewhere in the library are kept.",
            ))
            .build();

        dialog.add_responses(&[
            ("cancel", &gettext("Cancel")),
            ("delete", &gettext("Delete")),
        ]);
        dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
        dialog.set_close_response("cancel");
        dialog.set_default_response(Some("cancel"));

        if dialog.choose_future(Some(self)).await != "delete" {
            return;
        }

        let kind = self.kind();
        let library = self.library();
        let mut deleted = 0usize;
        let mut skipped = 0usize;

        for id in &ids {
            match kind.delete(&library, id) {
                Ok(()) => deleted += 1,
                Err(err) => match err.downcast_ref::<LibraryError>() {
                    Some(LibraryError::StillReferenced(_)) => skipped += 1,
                    _ => {
                        self.report("Failed to delete item", err);
                        break;
                    }
                },
            }
        }

        self.reload();

        if let Some(toast_overlay) = util::find_toast_overlay(self) {
            let message = if skipped == 0 {
                format_translated!(gettext("Deleted {}"), deleted.to_string())
            } else {
                format_translated!(
                    gettext("Deleted {}; kept {}"),
                    deleted.to_string(),
                    skipped.to_string()
                )
            };

            toast_overlay.add_toast(adw::Toast::new(&message));
        }
    }

    #[template_callback]
    fn create_entity(&self) {
        let page = self.kind().create(&self.navigation(), &self.library());
        self.navigation().push(&page);
    }

    fn edit(&self, id: &str) {
        match self.kind().edit(&self.navigation(), &self.library(), id) {
            Ok(page) => self.navigation().push(&page),
            Err(err) => self.report("Failed to open the editor", err),
        }
    }

    fn report(&self, message: &str, err: anyhow::Error) {
        match util::find_toast_overlay(self) {
            Some(toast_overlay) => util::error_toast(message, err, &toast_overlay),
            None => log::error!("{message}: {err:?}"),
        }
    }

    fn reload(&self) {
        let kind = self.kind();

        let items = match kind.load(&self.library()) {
            Ok(items) => items,
            Err(err) => {
                self.report("Failed to list the library", err);
                return;
            }
        };

        let store = self.items();
        store.remove_all();
        store.extend_from_slice(&items);

        self.set_title(&kind.title());

        self.update_view_state();
    }

    fn update_view_state(&self) {
        let imp = self.imp();
        let kind = self.kind();
        let search_is_empty = imp.search_entry.text().is_empty();
        let n_visible = self.selection().n_items();

        imp.stack
            .set_visible_child_name(if n_visible == 0 { "empty" } else { "list" });

        imp.status_page.set_title(&if search_is_empty {
            format_translated!(gettext("No {} yet"), kind.title().to_lowercase())
        } else {
            gettext("No matches")
        });

        imp.status_page.set_description(Some(&if search_is_empty {
            gettext("Items you add to your music library show up here.")
        } else {
            gettext("Try a different filter.")
        }));

        imp.count_label.set_label(&format_translated!(
            gettext("{} items"),
            n_visible.to_string()
        ));
    }

    fn update_selection(&self) {
        let imp = self.imp();
        let n_selected = self.selected_ids().len();

        imp.clear_selection_button.set_sensitive(n_selected > 0);
        imp.clear_selection_button.set_label(&if n_selected > 0 {
            format_translated!(gettext("Clear {} selected"), n_selected.to_string())
        } else {
            gettext("Clear selection")
        });

        imp.delete_button.set_sensitive(n_selected > 0);
    }

    pub fn selected_ids(&self) -> Vec<String> {
        let selection = self.selection();

        (0..selection.n_items())
            .filter(|position| selection.is_selected(*position))
            .filter_map(|position| {
                selection
                    .item(position)
                    .and_downcast::<EntityObject>()
                    .map(|item| item.id())
            })
            .collect()
    }

    fn rebuild_columns(&self) {
        let imp = self.imp();
        let column_view = imp.column_view.get();

        for column in column_view
            .columns()
            .iter::<gtk::ColumnViewColumn>()
            .flatten()
            .collect::<Vec<_>>()
        {
            column_view.remove_column(&column);
        }

        let kind = self.kind();

        let name_column = self.text_column(&kind.name_title(), "name", true);
        column_view.append_column(&name_column);

        if let Some((title, expand)) = kind.details_title() {
            column_view.append_column(&self.text_column(&title, "details", expand));
        }

        if kind == BrowserKind::Recordings {
            column_view.append_column(&self.number_column(&gettext("Tracks"), "n-tracks"));
        }

        column_view.append_column(&self.text_column(&gettext("Source"), "source", false));
        column_view.append_column(&self.text_column(&gettext("Last used"), "last-used", false));

        // Start sorted by name; without this the table shows database order.
        column_view.sort_by_column(Some(&name_column), gtk::SortType::Ascending);
    }

    fn string_sorter(property: &str) -> gtk::StringSorter {
        let sorter = gtk::StringSorter::new(Some(gtk::PropertyExpression::new(
            EntityObject::static_type(),
            None::<gtk::Expression>,
            property,
        )));

        sorter.set_ignore_case(true);
        sorter
    }

    fn text_column(&self, title: &str, property: &str, expand: bool) -> gtk::ColumnViewColumn {
        let factory = gtk::SignalListItemFactory::new();

        factory.connect_setup(|_, list_item| {
            let Some(list_item) = list_item.downcast_ref::<gtk::ListItem>() else {
                return;
            };

            let label = gtk::Label::new(None);
            label.set_xalign(0.0);
            label.set_ellipsize(gtk::pango::EllipsizeMode::End);
            list_item.set_child(Some(&label));
        });

        let bound_property = property.to_owned();
        factory.connect_bind(move |_, list_item| {
            let Some(list_item) = list_item.downcast_ref::<gtk::ListItem>() else {
                return;
            };

            let (Some(item), Some(label)) = (
                list_item.item().and_downcast::<EntityObject>(),
                list_item.child().and_downcast::<gtk::Label>(),
            ) else {
                return;
            };

            label.set_label(&item.property::<String>(&bound_property));
        });

        let column = gtk::ColumnViewColumn::new(Some(title), Some(factory));
        column.set_expand(expand);
        column.set_resizable(true);
        column.set_sorter(Some(&Self::string_sorter(property)));
        column
    }

    fn number_column(&self, title: &str, property: &str) -> gtk::ColumnViewColumn {
        let factory = gtk::SignalListItemFactory::new();

        factory.connect_setup(|_, list_item| {
            let Some(list_item) = list_item.downcast_ref::<gtk::ListItem>() else {
                return;
            };

            let label = gtk::Label::new(None);
            label.set_xalign(1.0);
            list_item.set_child(Some(&label));
        });

        let bound_property = property.to_owned();
        factory.connect_bind(move |_, list_item| {
            let Some(list_item) = list_item.downcast_ref::<gtk::ListItem>() else {
                return;
            };

            let (Some(item), Some(label)) = (
                list_item.item().and_downcast::<EntityObject>(),
                list_item.child().and_downcast::<gtk::Label>(),
            ) else {
                return;
            };

            label.set_label(&item.property::<u32>(&bound_property).to_string());
        });

        let column = gtk::ColumnViewColumn::new(Some(title), Some(factory));
        column.set_resizable(true);
        column.set_sorter(Some(&gtk::NumericSorter::new(Some(
            gtk::PropertyExpression::new(
                EntityObject::static_type(),
                None::<gtk::Expression>,
                property,
            ),
        ))));

        column
    }
}
