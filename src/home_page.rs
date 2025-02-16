use std::cell::{OnceCell, RefCell};

use adw::subclass::{navigation_page::NavigationPageImpl, prelude::*};
use gtk::{
    gio,
    glib::{self, Properties},
    prelude::*,
};

use crate::{
    album_tile::MusicusAlbumTile,
    config,
    db::models::*,
    editor::{
        ensemble_editor::MusicusEnsembleEditor, person_editor::MusicusPersonEditor,
        work_editor::MusicusWorkEditor,
    },
    library::{LibraryQuery, MusicusLibrary},
    player::MusicusPlayer,
    program::Program,
    program_tile::MusicusProgramTile,
    recording_tile::MusicusRecordingTile,
    search_entry::MusicusSearchEntry,
    search_tag::Tag,
    tag_tile::MusicusTagTile,
};

mod imp {
    use super::*;

    #[derive(Properties, Debug, Default, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::MusicusHomePage)]
    #[template(file = "data/ui/home_page.blp")]
    pub struct MusicusHomePage {
        #[property(get, construct_only)]
        pub navigation: OnceCell<adw::NavigationView>,

        #[property(get, construct_only)]
        pub library: OnceCell<MusicusLibrary>,

        #[property(get, construct_only)]
        pub player: OnceCell<MusicusPlayer>,

        pub programs: RefCell<Vec<Program>>,
        pub composers: RefCell<Vec<Person>>,
        pub performers: RefCell<Vec<Person>>,
        pub ensembles: RefCell<Vec<Ensemble>>,
        pub works: RefCell<Vec<Work>>,
        pub recordings: RefCell<Vec<Recording>>,
        pub albums: RefCell<Vec<Album>>,

        #[template_child]
        pub search_entry: TemplateChild<MusicusSearchEntry>,
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
        pub works_flow_box: TemplateChild<gtk::FlowBox>,
        #[template_child]
        pub recordings_flow_box: TemplateChild<gtk::FlowBox>,
        #[template_child]
        pub albums_flow_box: TemplateChild<gtk::FlowBox>,
        #[template_child]
        pub play_button: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MusicusHomePage {
        const NAME: &'static str = "MusicusHomePage";
        type Type = super::MusicusHomePage;
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
    impl ObjectImpl for MusicusHomePage {
        fn constructed(&self) {
            self.parent_constructed();

            self.search_entry.set_key_capture_widget(&*self.obj());

            let obj = self.obj().to_owned();
            self.search_entry.connect_query_changed(move |entry| {
                obj.query(&entry.query());
            });

            let obj = self.obj().to_owned();
            self.library.get().unwrap().connect_changed(move |_| {
                obj.imp().search_entry.reset();
            });

            self.player
                .get()
                .unwrap()
                .bind_property("active", &self.play_button.get(), "visible")
                .invert_boolean()
                .sync_create()
                .build();

            let settings = gio::Settings::new(&config::APP_ID);

            let programs = vec![
                Program::deserialize(&settings.string("program1")).unwrap(),
                Program::deserialize(&settings.string("program2")).unwrap(),
                Program::deserialize(&settings.string("program3")).unwrap(),
            ];

            for program in &programs {
                self.programs_flow_box
                    .append(&MusicusProgramTile::new(program.to_owned()));
            }

            self.programs.replace(programs);

            self.obj().query(&LibraryQuery::default());
        }
    }

    impl WidgetImpl for MusicusHomePage {}
    impl NavigationPageImpl for MusicusHomePage {}
}

glib::wrapper! {
    pub struct MusicusHomePage(ObjectSubclass<imp::MusicusHomePage>)
        @extends gtk::Widget, adw::NavigationPage;
}

#[gtk::template_callbacks]
impl MusicusHomePage {
    pub fn new(
        navigation: &adw::NavigationView,
        library: &MusicusLibrary,
        player: &MusicusPlayer,
    ) -> Self {
        glib::Object::builder()
            .property("navigation", navigation)
            .property("library", library)
            .property("player", player)
            .build()
    }

    #[template_callback]
    fn back_button_clicked(&self) {
        self.imp().search_entry.reset();
    }

    #[template_callback]
    fn edit_button_clicked(&self) {
        if let Some(tag) = self.imp().search_entry.tags().first() {
            match tag {
                Tag::Composer(person) | Tag::Performer(person) => {
                    self.navigation().push(&MusicusPersonEditor::new(
                        &self.navigation(),
                        &self.library(),
                        Some(person),
                    ));
                }
                Tag::Ensemble(ensemble) => {
                    self.navigation().push(&MusicusEnsembleEditor::new(
                        &self.navigation(),
                        &self.library(),
                        Some(ensemble),
                    ));
                }
                Tag::Work(work) => self.navigation().push(&MusicusWorkEditor::new(
                    &self.navigation(),
                    &self.library(),
                    Some(work),
                    false,
                )),
            }
        }
    }

    #[template_callback]
    fn play(&self) {
        let program = Program::from_query(self.imp().search_entry.query());
        self.player().set_program(program);

        self.player().play();
    }

    #[template_callback]
    fn select(&self, search_entry: &MusicusSearchEntry) {
        let imp = self.imp();

        if imp.programs_flow_box.is_visible() {
            if let Some(program) = imp.programs.borrow().first().cloned() {
                self.player().set_program(program);
            }
        } else {
            let (composer, performer, ensemble, work, recording, album) = {
                (
                    imp.composers.borrow().first().cloned(),
                    imp.performers.borrow().first().cloned(),
                    imp.ensembles.borrow().first().cloned(),
                    imp.works.borrow().first().cloned(),
                    imp.recordings.borrow().first().cloned(),
                    imp.albums.borrow().first().cloned(),
                )
            };

            if let Some(person) = composer {
                search_entry.add_tag(Tag::Composer(person));
            } else if let Some(person) = performer {
                search_entry.add_tag(Tag::Performer(person));
            } else if let Some(ensemble) = ensemble {
                search_entry.add_tag(Tag::Ensemble(ensemble));
            } else if let Some(work) = work {
                search_entry.add_tag(Tag::Work(work));
            } else if let Some(recording) = recording {
                self.player().play_recording(&recording);
            } else if let Some(album) = album {
                self.show_album(&album);
            }
        }
    }

    #[template_callback]
    fn program_selected(&self, tile: &gtk::FlowBoxChild, _: &gtk::FlowBox) {
        self.player()
            .set_program(tile.downcast_ref::<MusicusProgramTile>().unwrap().program());
    }

    #[template_callback]
    fn tile_selected(&self, tile: &gtk::FlowBoxChild, _: &gtk::FlowBox) {
        self.imp()
            .search_entry
            .add_tag(tile.downcast_ref::<MusicusTagTile>().unwrap().tag().clone())
    }

    #[template_callback]
    fn recording_selected(&self, tile: &gtk::FlowBoxChild, _: &gtk::FlowBox) {
        self.player().play_recording(
            tile.downcast_ref::<MusicusRecordingTile>()
                .unwrap()
                .recording(),
        );
    }

    #[template_callback]
    fn album_selected(&self, tile: &gtk::FlowBoxChild, _: &gtk::FlowBox) {
        self.show_album(tile.downcast_ref::<MusicusAlbumTile>().unwrap().album());
    }

    fn show_album(&self, _album: &Album) {
        todo!("Show album");
    }

    fn query(&self, query: &LibraryQuery) {
        let imp = self.imp();
        let results = self.library().query(query).unwrap();

        for flowbox in [
            &imp.composers_flow_box,
            &imp.performers_flow_box,
            &imp.ensembles_flow_box,
            &imp.works_flow_box,
            &imp.recordings_flow_box,
            &imp.albums_flow_box,
        ] {
            while let Some(widget) = flowbox.first_child() {
                flowbox.remove(&widget);
            }
        }

        imp.programs_flow_box.set_visible(query.is_empty());

        if let Some(tag) = imp.search_entry.tags().first() {
            match tag {
                Tag::Composer(person) | Tag::Performer(person) => {
                    imp.title_label.set_text(&person.name.get());
                    imp.subtitle_label.set_visible(false);
                }
                Tag::Ensemble(ensemble) => {
                    imp.title_label.set_text(&ensemble.name.get());
                    imp.subtitle_label.set_visible(false);
                }
                Tag::Work(work) => {
                    imp.title_label.set_text(&work.name.get());
                    if let Some(composers) = work.composers_string() {
                        imp.subtitle_label.set_text(&composers);
                        imp.subtitle_label.set_visible(true);
                    } else {
                        imp.subtitle_label.set_visible(false);
                    }
                }
            }

            imp.header_box.set_visible(true);
        } else {
            imp.header_box.set_visible(false);
        }

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
            imp.works_flow_box.set_visible(!results.works.is_empty());
            imp.recordings_flow_box
                .set_visible(!results.recordings.is_empty());
            imp.albums_flow_box.set_visible(!results.albums.is_empty());

            for composer in &results.composers {
                imp.composers_flow_box
                    .append(&MusicusTagTile::new(Tag::Composer(composer.clone())));
            }

            for performer in &results.performers {
                imp.performers_flow_box
                    .append(&MusicusTagTile::new(Tag::Performer(performer.clone())));
            }

            for ensemble in &results.ensembles {
                imp.ensembles_flow_box
                    .append(&MusicusTagTile::new(Tag::Ensemble(ensemble.clone())));
            }

            for work in &results.works {
                imp.works_flow_box
                    .append(&MusicusTagTile::new(Tag::Work(work.clone())));
            }

            for recording in &results.recordings {
                imp.recordings_flow_box.append(&MusicusRecordingTile::new(
                    &self.navigation(),
                    &self.library(),
                    recording,
                ));
            }

            for album in &results.albums {
                imp.albums_flow_box.append(&MusicusAlbumTile::new(album));
            }

            imp.composers.replace(results.composers);
            imp.performers.replace(results.performers);
            imp.ensembles.replace(results.ensembles);
            imp.works.replace(results.works);
            imp.recordings.replace(results.recordings);
            imp.albums.replace(results.albums);
        }
    }
}
