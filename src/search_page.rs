use std::cell::{OnceCell, RefCell};

use adw::subclass::{navigation_page::NavigationPageImpl, prelude::*};
use gtk::{
    gio,
    glib::{self, Properties},
    prelude::*,
};

use musicus_library::db::models::*;

use crate::{
    album_page::AlbumPage,
    album_tile::AlbumTile,
    editor::{
        ensemble::EnsembleEditor, simple_entity::SimpleEntityEditor, tag::TagEditor,
        work::WorkEditor,
    },
    facet_tile::FacetTile,
    library::{Facet, Library, LibraryQuery},
    player::Player,
    program::Program,
    program_tile::ProgramTile,
    recording_tile::RecordingTile,
    util,
    work_page::WorkPage,
};

mod imp {
    use super::*;

    #[derive(Properties, Debug, Default, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::SearchPage)]
    #[template(file = "data/ui/search_page.blp")]
    pub struct SearchPage {
        #[property(get, construct_only)]
        pub toast_overlay: OnceCell<adw::ToastOverlay>,

        #[property(get, construct_only)]
        pub navigation: OnceCell<adw::NavigationView>,

        #[property(get, construct_only)]
        pub library: OnceCell<Library>,

        #[property(get, construct_only)]
        pub player: OnceCell<Player>,

        pub query: OnceCell<LibraryQuery>,
        pub highlight: RefCell<Option<Facet>>,

        pub program_tiles: RefCell<Vec<ProgramTile>>,
        pub composers: RefCell<Vec<Person>>,
        pub performers: RefCell<Vec<Person>>,
        pub ensembles: RefCell<Vec<Ensemble>>,
        pub instruments: RefCell<Vec<Instrument>>,
        pub tags: RefCell<Vec<TagValue>>,
        pub works: RefCell<Vec<Work>>,
        pub recordings: RefCell<Vec<Recording>>,
        pub albums: RefCell<Vec<Album>>,

        #[template_child]
        pub scrolled_window: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub header_bar: TemplateChild<adw::HeaderBar>,
        #[template_child]
        pub search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub header_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub title_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub subtitle_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub programs_flow_box: TemplateChild<gtk::FlowBox>,
        #[template_child]
        pub composers_flow_box: TemplateChild<gtk::FlowBox>,
        #[template_child]
        pub performers_flow_box: TemplateChild<gtk::FlowBox>,
        #[template_child]
        pub ensembles_flow_box: TemplateChild<gtk::FlowBox>,
        #[template_child]
        pub instruments_flow_box: TemplateChild<gtk::FlowBox>,
        #[template_child]
        pub tags_flow_box: TemplateChild<gtk::FlowBox>,
        #[template_child]
        pub works_flow_box: TemplateChild<gtk::FlowBox>,
        #[template_child]
        pub recordings_flow_box: TemplateChild<gtk::FlowBox>,
        #[template_child]
        pub albums_flow_box: TemplateChild<gtk::FlowBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SearchPage {
        const NAME: &'static str = "MusicusSearchPage";
        type Type = super::SearchPage;
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
    impl ObjectImpl for SearchPage {
        fn constructed(&self) {
            self.parent_constructed();

            self.search_entry.set_key_capture_widget(Some(&*self.obj()));

            let obj = self.obj().to_owned();
            self.search_entry.connect_search_changed(move |entry| {
                obj.imp().scrolled_window.vadjustment().set_value(0.0);
                obj.search(&entry.text());
            });

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
                    obj.edit();
                })
                .build();

            let obj = self.obj().to_owned();
            let delete_action = gio::ActionEntry::builder("delete")
                .activate(move |_, _, _| {
                    obj.delete();
                })
                .build();

            let actions = gio::SimpleActionGroup::new();
            actions.add_action_entries([add_to_playlist_action, edit_action, delete_action]);
            self.obj().insert_action_group("search", Some(&actions));
        }
    }

    impl WidgetImpl for SearchPage {
        fn map(&self) {
            self.parent_map();
            self.search_entry.grab_focus();
        }
    }

    impl NavigationPageImpl for SearchPage {}
}

glib::wrapper! {
    pub struct SearchPage(ObjectSubclass<imp::SearchPage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

#[gtk::template_callbacks]
impl SearchPage {
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

        if query.is_empty() {
            for key in &["program1", "program2", "program3"] {
                obj.imp()
                    .programs_flow_box
                    .append(&ProgramTile::new_for_setting(navigation, key));
            }
        }

        obj.imp().query.set(query).unwrap();
        obj.search("");

        obj
    }

    fn edit(&self) {
        if let Some(highlight) = &*self.imp().highlight.borrow() {
            match highlight {
                Facet::Composer(person) | Facet::Performer(person) => {
                    self.navigation().push(&SimpleEntityEditor::person(
                        &self.navigation(),
                        &self.library(),
                        Some(person),
                    ));
                }
                Facet::Ensemble(ensemble) => {
                    self.navigation().push(&EnsembleEditor::new(
                        &self.navigation(),
                        &self.library(),
                        Some(ensemble),
                    ));
                }
                Facet::Instrument(instrument) => {
                    self.navigation().push(&SimpleEntityEditor::instrument(
                        &self.navigation(),
                        &self.library(),
                        Some(instrument),
                    ))
                }
                Facet::Work(work) => self.navigation().push(&WorkEditor::new(
                    &self.navigation(),
                    &self.library(),
                    Some(work),
                    false,
                )),
                Facet::Tag(tag_value) => self.navigation().push(&TagEditor::new(
                    &self.navigation(),
                    &self.library(),
                    Some(&tag_value.tag),
                )),
            }
        }
    }

    fn delete(&self) {
        if let Some(highlight) = &*self.imp().highlight.borrow() {
            match highlight {
                Facet::Composer(person) | Facet::Performer(person) => {
                    if let Err(err) = self.library().delete_person(&person.person_id) {
                        util::error_toast("Failed to delete person", err, &self.toast_overlay());
                    }
                }
                Facet::Ensemble(ensemble) => {
                    if let Err(err) = self.library().delete_ensemble(&ensemble.ensemble_id) {
                        util::error_toast("Failed to delete ensemble", err, &self.toast_overlay());
                    }
                }
                Facet::Instrument(instrument) => {
                    if let Err(err) = self.library().delete_instrument(&instrument.instrument_id) {
                        util::error_toast(
                            "Failed to delete instrument",
                            err,
                            &self.toast_overlay(),
                        );
                    }
                }
                Facet::Work(work) => {
                    if let Err(err) = self.library().delete_work(&work.work_id) {
                        util::error_toast("Failed to delete work", err, &self.toast_overlay());
                    }
                }
                Facet::Tag(tag_value) => {
                    if let Err(err) = self.library().delete_tag(&tag_value.tag.tag_id) {
                        util::error_toast("Failed to delete tag", err, &self.toast_overlay());
                    }
                }
            }
        }
    }

    #[template_callback]
    fn play_button_clicked(&self) {
        let program = Program::from_query(self.imp().query.get().unwrap().clone());
        self.player().set_program(program);
        self.player().play_from_program();
    }

    #[template_callback]
    fn select(&self) {
        let imp = self.imp();

        if imp.programs_flow_box.is_visible() {
            if let Some(widget) = imp.programs_flow_box.first_child() {
                if let Ok(program_tile) = widget.downcast::<ProgramTile>() {
                    self.player().set_program(program_tile.program());
                    self.player().play_from_program();
                }
            }
        } else {
            let mut new_query = self.imp().query.get().unwrap().clone();
            let mut work_selected = None;

            let query_changed = if let Some(person) = imp.composers.borrow().first().cloned() {
                new_query.composer = Some(person);
                true
            } else if let Some(person) = imp.performers.borrow().first().cloned() {
                new_query.performer = Some(person);
                true
            } else if let Some(ensemble) = imp.ensembles.borrow().first().cloned() {
                new_query.ensemble = Some(ensemble);
                true
            } else if let Some(instrument) = imp.instruments.borrow().first().cloned() {
                new_query.instrument = Some(instrument);
                true
            } else if let Some(tag_value) = imp.tags.borrow().first().cloned() {
                new_query.tag = Some(tag_value);
                true
            } else if let Some(work) = imp.works.borrow().first().cloned() {
                work_selected = Some(work);
                true
            } else if let Some(recording) = imp.recordings.borrow().first().cloned() {
                let playlist = self.player().recording_to_playlist(&recording);
                self.player().append_and_play(playlist);
                false
            } else if let Some(album) = imp.albums.borrow().first().cloned() {
                self.show_album(&album);
                false
            } else {
                false
            };

            if let Some(work) = work_selected {
                new_query.work = Some(work);
                self.navigation().push(&WorkPage::new(
                    &self.toast_overlay(),
                    &self.navigation(),
                    &self.library(),
                    &self.player(),
                    new_query,
                ));
            } else if query_changed {
                self.navigation().push(&SearchPage::new(
                    &self.toast_overlay(),
                    &self.navigation(),
                    &self.library(),
                    &self.player(),
                    new_query,
                ));
            }
        }
    }

    #[template_callback]
    fn program_selected(&self, tile: &gtk::FlowBoxChild) {
        self.player()
            .set_program(tile.downcast_ref::<ProgramTile>().unwrap().program());
        self.player().play_from_program();
    }

    #[template_callback]
    fn tile_selected(&self, tile: &gtk::FlowBoxChild) {
        let mut new_query = self.imp().query.get().unwrap().clone();
        match tile.downcast_ref::<FacetTile>().unwrap().facet().clone() {
            Facet::Composer(person) => new_query.composer = Some(person),
            Facet::Performer(person) => new_query.performer = Some(person),
            Facet::Ensemble(ensemble) => new_query.ensemble = Some(ensemble),
            Facet::Instrument(instrument) => new_query.instrument = Some(instrument),
            Facet::Work(work) => {
                new_query.work = Some(work);
                self.navigation().push(&WorkPage::new(
                    &self.toast_overlay(),
                    &self.navigation(),
                    &self.library(),
                    &self.player(),
                    new_query,
                ));
                return;
            }
            Facet::Tag(tag) => new_query.tag = Some(tag),
        }

        self.navigation().push(&SearchPage::new(
            &self.toast_overlay(),
            &self.navigation(),
            &self.library(),
            &self.player(),
            new_query,
        ));
    }

    #[template_callback]
    fn recording_selected(&self, tile: &gtk::FlowBoxChild) {
        let playlist = self
            .player()
            .recording_to_playlist(tile.downcast_ref::<RecordingTile>().unwrap().recording());
        self.player().append_and_play(playlist);
    }

    #[template_callback]
    fn album_selected(&self, tile: &gtk::FlowBoxChild) {
        self.show_album(tile.downcast_ref::<AlbumTile>().unwrap().album());
    }

    fn show_album(&self, album: &Album) {
        self.navigation().push(&AlbumPage::new(
            &self.toast_overlay(),
            &self.navigation(),
            &self.library(),
            &self.player(),
            album.to_owned(),
        ));
    }

    fn search(&self, search: &str) {
        let query = self.imp().query.get().unwrap();

        let imp = self.imp();

        let results = match self.library().search(query, search) {
            Ok(results) => results,
            Err(err) => {
                util::error_toast("Search failed", err, &self.toast_overlay());
                return;
            }
        };

        for flowbox in [
            &imp.composers_flow_box,
            &imp.performers_flow_box,
            &imp.ensembles_flow_box,
            &imp.instruments_flow_box,
            &imp.tags_flow_box,
            &imp.works_flow_box,
            &imp.recordings_flow_box,
            &imp.albums_flow_box,
        ] {
            while let Some(widget) = flowbox.first_child() {
                flowbox.remove(&widget);
            }
        }

        // Only show programs initially.
        imp.programs_flow_box
            .set_visible(query.is_empty() && search.is_empty());

        imp.header_bar.set_show_title(query.is_empty());
        imp.header_box.set_visible(!query.is_empty());

        if let Some(title) = query.title() {
            imp.title_label.set_text(&title);
        }

        match query.description() {
            Some(description) => {
                imp.subtitle_label.set_text(&description);
                imp.subtitle_label.set_visible(true);
            }
            None => imp.subtitle_label.set_visible(false),
        }

        imp.highlight.replace(query.highlight());

        if results.is_empty() {
            imp.stack.set_visible_child_name("empty");
        } else {
            imp.stack.set_visible_child_name("results");

            imp.composers_flow_box
                .set_visible(!results.composers.is_empty());
            imp.performers_flow_box
                .set_visible(!results.performers.is_empty());
            imp.ensembles_flow_box
                .set_visible(!results.ensembles.is_empty());
            imp.instruments_flow_box
                .set_visible(!results.instruments.is_empty());
            imp.tags_flow_box.set_visible(!results.tags.is_empty());
            imp.works_flow_box.set_visible(!results.works.is_empty());
            imp.recordings_flow_box
                .set_visible(!results.recordings.is_empty());
            imp.albums_flow_box.set_visible(!results.albums.is_empty());

            for composer in &results.composers {
                imp.composers_flow_box
                    .append(&FacetTile::new(Facet::Composer(composer.clone())));
            }

            for performer in &results.performers {
                imp.performers_flow_box
                    .append(&FacetTile::new(Facet::Performer(performer.clone())));
            }

            for ensemble in &results.ensembles {
                imp.ensembles_flow_box
                    .append(&FacetTile::new(Facet::Ensemble(ensemble.clone())));
            }

            for instrument in &results.instruments {
                imp.instruments_flow_box
                    .append(&FacetTile::new(Facet::Instrument(instrument.clone())));
            }

            for tag_value in &results.tags {
                imp.tags_flow_box
                    .append(&FacetTile::new(Facet::Tag(tag_value.clone())));
            }

            for work in &results.works {
                imp.works_flow_box
                    .append(&FacetTile::new(Facet::Work(work.clone())));
            }

            for recording in &results.recordings {
                imp.recordings_flow_box.append(&RecordingTile::new(
                    &self.toast_overlay(),
                    &self.navigation(),
                    &self.library(),
                    &self.player(),
                    recording,
                ));
            }

            for album in &results.albums {
                imp.albums_flow_box.append(&AlbumTile::new(album));
            }

            imp.composers.replace(results.composers);
            imp.performers.replace(results.performers);
            imp.ensembles.replace(results.ensembles);
            imp.instruments.replace(results.instruments);
            imp.tags.replace(results.tags);
            imp.works.replace(results.works);
            imp.recordings.replace(results.recordings);
            imp.albums.replace(results.albums);
        }
    }
}
