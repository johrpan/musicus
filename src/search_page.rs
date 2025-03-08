use std::cell::{OnceCell, RefCell};

use adw::subclass::{navigation_page::NavigationPageImpl, prelude::*};
use formatx::formatx;
use gettextrs::gettext;
use gtk::{
    gio,
    glib::{self, Properties},
    prelude::*,
};

use crate::{
    album_page::AlbumPage,
    album_tile::AlbumTile,
    config,
    db::models::*,
    editor::{
        ensemble::EnsembleEditor, instrument::InstrumentEditor, person::PersonEditor,
        work::WorkEditor,
    },
    library::{Library, LibraryQuery},
    player::Player,
    program::Program,
    program_tile::ProgramTile,
    recording_tile::RecordingTile,
    search_tag::Tag,
    tag_tile::TagTile,
};

mod imp {
    use super::*;

    #[derive(Properties, Debug, Default, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::SearchPage)]
    #[template(file = "data/ui/search_page.blp")]
    pub struct SearchPage {
        #[property(get, construct_only)]
        pub navigation: OnceCell<adw::NavigationView>,

        #[property(get, construct_only)]
        pub library: OnceCell<Library>,

        #[property(get, construct_only)]
        pub player: OnceCell<Player>,

        pub query: OnceCell<LibraryQuery>,
        pub highlight: RefCell<Option<Tag>>,

        pub programs: RefCell<Vec<Program>>,
        pub composers: RefCell<Vec<Person>>,
        pub performers: RefCell<Vec<Person>>,
        pub ensembles: RefCell<Vec<Ensemble>>,
        pub instruments: RefCell<Vec<Instrument>>,
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
        @extends gtk::Widget, adw::NavigationPage;
}

#[gtk::template_callbacks]
impl SearchPage {
    pub fn new(
        navigation: &adw::NavigationView,
        library: &Library,
        player: &Player,
        query: LibraryQuery,
    ) -> Self {
        let obj: Self = glib::Object::builder()
            .property("navigation", navigation)
            .property("library", library)
            .property("player", player)
            .build();

        if query.is_empty() {
            let settings = gio::Settings::new(&config::APP_ID);

            let programs = vec![
                Program::deserialize(&settings.string("program1")).unwrap(),
                Program::deserialize(&settings.string("program2")).unwrap(),
                Program::deserialize(&settings.string("program3")).unwrap(),
            ];

            for program in &programs {
                obj.imp()
                    .programs_flow_box
                    .append(&ProgramTile::new(program.to_owned()));
            }

            obj.imp().programs.replace(programs);
        }

        obj.imp().query.set(query).unwrap();
        obj.search("");

        obj
    }

    fn edit(&self) {
        if let Some(highlight) = &*self.imp().highlight.borrow() {
            match highlight {
                Tag::Composer(person) | Tag::Performer(person) => {
                    self.navigation().push(&PersonEditor::new(
                        &self.navigation(),
                        &self.library(),
                        Some(person),
                    ));
                }
                Tag::Ensemble(ensemble) => {
                    self.navigation().push(&EnsembleEditor::new(
                        &self.navigation(),
                        &self.library(),
                        Some(ensemble),
                    ));
                }
                Tag::Instrument(instrument) => self.navigation().push(&InstrumentEditor::new(
                    &self.navigation(),
                    &self.library(),
                    Some(instrument),
                )),
                Tag::Work(work) => self.navigation().push(&WorkEditor::new(
                    &self.navigation(),
                    &self.library(),
                    Some(work),
                    false,
                )),
            }
        }
    }

    fn delete(&self) {
        log::warn!("Deletion not implemented");

        // if let Some(highlight) = &*self.imp().highlight.borrow() {
        //     match highlight {
        //         Tag::Composer(person) | Tag::Performer(person) => {
        //             // TODO
        //         }
        //         Tag::Ensemble(ensemble) => {
        //             // TODO
        //         }
        //         Tag::Instrument(instrument) => {
        //             // TODO
        //         }
        //         Tag::Work(work) => {
        //             // TODO
        //         }
        //     }
        // }
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
            if let Some(program) = imp.programs.borrow().first().cloned() {
                self.player().set_program(program);
                self.player().play_from_program();
            }
        } else {
            let mut new_query = self.imp().query.get().unwrap().clone();

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
            } else if let Some(work) = imp.works.borrow().first().cloned() {
                new_query.work = Some(work);
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

            if query_changed {
                self.navigation().push(&SearchPage::new(
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
        match tile.downcast_ref::<TagTile>().unwrap().tag().clone() {
            Tag::Composer(person) => new_query.composer = Some(person),
            Tag::Performer(person) => new_query.performer = Some(person),
            Tag::Ensemble(ensemble) => new_query.ensemble = Some(ensemble),
            Tag::Instrument(instrument) => new_query.instrument = Some(instrument),
            Tag::Work(work) => new_query.work = Some(work),
        }

        self.navigation().push(&SearchPage::new(
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
            &self.navigation(),
            &self.library(),
            &self.player(),
            album.to_owned(),
        ));
    }

    fn search(&self, search: &str) {
        let query = self.imp().query.get().unwrap();

        let imp = self.imp();
        let results = self.library().search(query, search).unwrap();

        for flowbox in [
            &imp.composers_flow_box,
            &imp.performers_flow_box,
            &imp.ensembles_flow_box,
            &imp.instruments_flow_box,
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

        let highlight = if let Some(work) = &query.work {
            imp.title_label.set_text(&work.name.get());
            if let Some(composers) = work.composers_string() {
                imp.subtitle_label.set_text(&composers);
                imp.subtitle_label.set_visible(true);
            } else {
                imp.subtitle_label.set_visible(false);
            }
            Some(Tag::Work(work.to_owned()))
        } else if let Some(person) = &query.composer {
            imp.title_label.set_text(&person.name.get());
            imp.subtitle_label.set_visible(false);
            Some(Tag::Composer(person.to_owned()))
        } else if let Some(person) = &query.performer {
            imp.title_label.set_text(&person.name.get());
            imp.subtitle_label.set_visible(false);
            Some(Tag::Performer(person.to_owned()))
        } else if let Some(ensemble) = &query.ensemble {
            imp.title_label.set_text(&ensemble.name.get());
            imp.subtitle_label.set_visible(false);
            Some(Tag::Ensemble(ensemble.to_owned()))
        } else if let Some(instrument) = &query.instrument {
            imp.title_label
                .set_text(&formatx!(gettext("Music for {}"), &instrument.name.get()).unwrap());
            imp.subtitle_label.set_visible(false);
            Some(Tag::Instrument(instrument.to_owned()))
        } else {
            None
        };

        if let Some(highlight) = &highlight {
            if !matches!(highlight, Tag::Work(_)) {
                let mut details = Vec::new();

                match highlight {
                    Tag::Composer(_) => {
                        if let Some(instrument) = &query.instrument {
                            details.push(formatx!(gettext("Works with {}"), instrument).unwrap());
                        }

                        if let (Some(person), Some(ensemble)) = (&query.performer, &query.ensemble)
                        {
                            details.push(
                                formatx!(gettext("Performed by {} and {}"), person, ensemble)
                                    .unwrap(),
                            );
                        } else if let Some(person) = &query.performer {
                            details.push(formatx!(gettext("Performed by {}"), person).unwrap());
                        } else if let Some(ensemble) = &query.ensemble {
                            details.push(formatx!(gettext("Performed by {}"), ensemble).unwrap());
                        }
                    }
                    Tag::Performer(_) => {
                        if let Some(instrument) = &query.instrument {
                            details.push(formatx!(gettext("Works with {}"), instrument).unwrap());
                        }

                        if let Some(ensemble) = &query.ensemble {
                            details.push(formatx!(gettext("Performed with {}"), ensemble).unwrap());
                        }
                    }
                    Tag::Ensemble(_) => {
                        if let Some(instrument) = &query.instrument {
                            details.push(formatx!(gettext("Works with {}"), instrument).unwrap());
                        }
                    }
                    Tag::Instrument(_) => (),
                    // Already covered.
                    Tag::Work(_) => unreachable!(),
                }

                imp.subtitle_label.set_visible(!details.is_empty());
                imp.subtitle_label.set_text(&details.join(", "));
            }
        }

        imp.highlight.replace(highlight);

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
            imp.works_flow_box.set_visible(!results.works.is_empty());
            imp.recordings_flow_box
                .set_visible(!results.recordings.is_empty());
            imp.albums_flow_box.set_visible(!results.albums.is_empty());

            for composer in &results.composers {
                imp.composers_flow_box
                    .append(&TagTile::new(Tag::Composer(composer.clone())));
            }

            for performer in &results.performers {
                imp.performers_flow_box
                    .append(&TagTile::new(Tag::Performer(performer.clone())));
            }

            for ensemble in &results.ensembles {
                imp.ensembles_flow_box
                    .append(&TagTile::new(Tag::Ensemble(ensemble.clone())));
            }

            for instrument in &results.instruments {
                imp.instruments_flow_box
                    .append(&TagTile::new(Tag::Instrument(instrument.clone())));
            }

            for work in &results.works {
                imp.works_flow_box
                    .append(&TagTile::new(Tag::Work(work.clone())));
            }

            for recording in &results.recordings {
                imp.recordings_flow_box.append(&RecordingTile::new(
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
            imp.works.replace(results.works);
            imp.recordings.replace(results.recordings);
            imp.albums.replace(results.albums);
        }
    }
}
