use std::cell::OnceCell;

use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::{
    gio,
    glib::{self, Properties},
    prelude::*,
};

use musicus_library::{db::models::*, format_translated};

use crate::{
    editor::work::WorkEditor,
    facet_tile::FacetTile,
    library::{Facet, Library, LibraryQuery},
    player::Player,
    program::Program,
    recording_tile::RecordingTile,
    util,
};

mod imp {
    use super::*;

    #[derive(Properties, Debug, Default, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::WorkPage)]
    #[template(file = "data/ui/work_page.blp")]
    pub struct WorkPage {
        #[property(get, construct_only)]
        pub toast_overlay: OnceCell<adw::ToastOverlay>,

        #[property(get, construct_only)]
        pub navigation: OnceCell<adw::NavigationView>,

        #[property(get, construct_only)]
        pub library: OnceCell<Library>,

        #[property(get, construct_only)]
        pub player: OnceCell<Player>,

        pub query: OnceCell<LibraryQuery>,
        pub work: OnceCell<Work>,

        #[template_child]
        pub title_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub subtitle_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub parent_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub parent_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub structure_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub related_works_flow_box: TemplateChild<gtk::FlowBox>,
        #[template_child]
        pub recordings_flow_box: TemplateChild<gtk::FlowBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for WorkPage {
        const NAME: &'static str = "MusicusWorkPage";
        type Type = super::WorkPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for WorkPage {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj().to_owned();
            let add_to_playlist_action = gio::ActionEntry::builder("add-to-playlist")
                .activate(move |_, _, _| {
                    let program = Program::from_query(obj.imp().query.get().unwrap().clone());
                    obj.player().set_program(program);
                })
                .build();

            let obj = self.obj().to_owned();
            let edit_action = gio::ActionEntry::builder("edit")
                .activate(move |_, _, _| {
                    obj.navigation().push(&WorkEditor::new(
                        &obj.navigation(),
                        &obj.library(),
                        Some(obj.imp().work.get().unwrap()),
                        false,
                    ));
                })
                .build();

            let obj = self.obj().to_owned();
            let delete_action = gio::ActionEntry::builder("delete")
                .activate(move |_, _, _| {
                    if let Err(err) = obj
                        .library()
                        .delete_work(&obj.imp().work.get().unwrap().work_id)
                    {
                        util::error_toast("Failed to delete work", err, &obj.toast_overlay());
                    }
                })
                .build();

            let actions = gio::SimpleActionGroup::new();
            actions.add_action_entries([add_to_playlist_action, edit_action, delete_action]);
            self.obj().insert_action_group("work", Some(&actions));
        }
    }

    impl WidgetImpl for WorkPage {}
    impl NavigationPageImpl for WorkPage {}
}

glib::wrapper! {
    pub struct WorkPage(ObjectSubclass<imp::WorkPage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

#[gtk::template_callbacks]
impl WorkPage {
    /// `query.work` must be `Some`; the rest of `query` (e.g. an active tag filter)
    /// carries over from wherever this page was reached from, scoping the shown
    /// recordings the same way the search page did before work got its own page.
    pub fn new(
        toast_overlay: &adw::ToastOverlay,
        navigation: &adw::NavigationView,
        library: &Library,
        player: &Player,
        query: LibraryQuery,
    ) -> Self {
        let obj: Self = glib::Object::builder()
            .property("toast-overlay", toast_overlay)
            .property("navigation", navigation)
            .property("library", library)
            .property("player", player)
            .build();

        let work = query.work.clone().expect("WorkPage requires query.work");

        obj.imp().title_label.set_label(work.name.get());

        if let Some(label) = work.composers_string() {
            obj.imp().subtitle_label.set_label(&label);
            obj.imp().subtitle_label.set_visible(true);
        }

        let results = match library.search(&query, "") {
            Ok(results) => results,
            Err(err) => {
                util::error_toast("Search failed", err, toast_overlay);
                Default::default()
            }
        };

        if let Some(parent) = &results.parent_work {
            let label = match parent.composers_string() {
                Some(composers) => {
                    format_translated!(gettext("Part of {} ({})"), parent.name.get(), composers,)
                }
                None => format_translated!(gettext("Part of {}"), parent.name.get()),
            };

            let this = obj.clone();
            let parent = parent.clone();
            obj.imp()
                .parent_button
                .connect_clicked(move |_| this.open_work(parent.clone()));

            obj.imp().parent_label.set_label(&label);
            obj.imp().parent_button.set_visible(true);
        }

        obj.imp().structure_box.set_visible(!work.parts.is_empty());
        append_structure(&obj, &obj.imp().structure_box, &work.parts, 0);

        obj.imp()
            .related_works_flow_box
            .set_visible(!results.works.is_empty());
        for related in &results.works {
            obj.imp()
                .related_works_flow_box
                .append(&FacetTile::new(Facet::Work(related.clone())));
        }

        for recording in &results.recordings {
            obj.imp().recordings_flow_box.append(&RecordingTile::new(
                toast_overlay,
                navigation,
                library,
                player,
                recording,
            ));
        }

        obj.imp().work.set(work).unwrap();
        obj.imp().query.set(query).unwrap();

        obj
    }

    fn open_work(&self, work: Work) {
        let mut new_query = self.imp().query.get().unwrap().clone();
        new_query.work = Some(work);

        self.navigation().push(&WorkPage::new(
            &self.toast_overlay(),
            &self.navigation(),
            &self.library(),
            &self.player(),
            new_query,
        ));
    }

    #[template_callback]
    fn play_button_clicked(&self) {
        let program = Program::from_query(self.imp().query.get().unwrap().clone());
        self.player().set_program(program);
        self.player().play_from_program();
    }

    #[template_callback]
    fn work_selected(&self, tile: &gtk::FlowBoxChild) {
        let Facet::Work(work) = tile.downcast_ref::<FacetTile>().unwrap().facet().clone() else {
            return;
        };

        self.open_work(work);
    }

    #[template_callback]
    fn recording_selected(&self, tile: &gtk::FlowBoxChild) {
        let playlist = self
            .player()
            .recording_to_playlist(tile.downcast_ref::<RecordingTile>().unwrap().recording());
        self.player().append_and_play(playlist);
    }
}

/// Add one flat-button row per part in `parts` to `container`, depth-first, each
/// clicking through to that part's own page.
fn append_structure(page: &WorkPage, container: &gtk::Box, parts: &[Work], depth: usize) {
    for part in parts {
        let button = structure_button(part.name.get(), depth, "work-part");

        let click_page = page.clone();
        let click_part = part.clone();
        button.connect_clicked(move |_| click_page.open_work(click_part.clone()));

        container.append(&button);

        append_structure(page, container, &part.parts, depth + 1);
    }
}

fn structure_button(label: &str, depth: usize, style: &str) -> gtk::Button {
    let label = gtk::Label::builder()
        .label(label)
        .xalign(0.0)
        .wrap(true)
        .margin_start((depth * 20) as i32)
        .css_classes([style])
        .build();

    gtk::Button::builder()
        .child(&label)
        .halign(gtk::Align::Fill)
        .css_classes(["flat"])
        .build()
}
