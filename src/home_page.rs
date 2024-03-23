use crate::{
    db::models::*,
    library::{LibraryQuery, MusicusLibrary},
    player::MusicusPlayer,
    playlist_item::PlaylistItem,
    recording_tile::MusicusRecordingTile,
    search_entry::MusicusSearchEntry,
    search_tag::Tag,
    tag_tile::MusicusTagTile,
};
use adw::subclass::{navigation_page::NavigationPageImpl, prelude::*};
use gtk::{
    glib::{self, clone, Properties},
    prelude::*,
};
use std::cell::{OnceCell, RefCell};

mod imp {
    use super::*;

    #[derive(Properties, Debug, Default, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::MusicusHomePage)]
    #[template(file = "data/ui/home_page.blp")]
    pub struct MusicusHomePage {
        #[property(get, construct_only)]
        pub library: OnceCell<MusicusLibrary>,

        #[property(get, construct_only)]
        pub player: OnceCell<MusicusPlayer>,

        pub composers: RefCell<Vec<Person>>,
        pub performers: RefCell<Vec<Person>>,
        pub ensembles: RefCell<Vec<Ensemble>>,
        pub works: RefCell<Vec<Work>>,
        pub recordings: RefCell<Vec<Recording>>,

        #[template_child]
        pub search_entry: TemplateChild<MusicusSearchEntry>,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
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

            self.search_entry
                .connect_query_changed(clone!(@weak self as _self => move |entry| {
                    _self.obj().query(&entry.query());
                }));

            self.player
                .get()
                .unwrap()
                .bind_property("active", &self.play_button.get(), "visible")
                .invert_boolean()
                .sync_create()
                .build();

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
    pub fn new(library: &MusicusLibrary, player: &MusicusPlayer) -> Self {
        glib::Object::builder()
            .property("library", library)
            .property("player", player)
            .build()
    }

    #[template_callback]
    fn play(&self, _: &gtk::Button) {
        log::info!("Play button clicked");
        self.player().play();
    }

    #[template_callback]
    fn select(&self, search_entry: &MusicusSearchEntry) {
        let imp = self.imp();

        let (composer, performer, ensemble, work, recording) = {
            (
                imp.composers.borrow().first().cloned(),
                imp.performers.borrow().first().cloned(),
                imp.ensembles.borrow().first().cloned(),
                imp.works.borrow().first().cloned(),
                imp.recordings.borrow().first().cloned(),
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
            self.play_recording(&recording);
        }
    }

    #[template_callback]
    fn tile_selected(&self, tile: &gtk::FlowBoxChild, _: &gtk::FlowBox) {
        self.imp()
            .search_entry
            .add_tag(tile.downcast_ref::<MusicusTagTile>().unwrap().tag().clone())
    }

    #[template_callback]
    fn recording_selected(&self, tile: &gtk::FlowBoxChild, _: &gtk::FlowBox) {
        self.play_recording(
            tile.downcast_ref::<MusicusRecordingTile>()
                .unwrap()
                .recording(),
        );
    }

    fn play_recording(&self, recording: &Recording) {
        let tracks = &recording.tracks;

        if tracks.is_empty() {
            log::warn!("Ignoring recording without tracks being added to the playlist.");
            return;
        }

        let title = format!(
            "{}: {}",
            recording.work.composers_string(),
            recording.work.name.get(),
        );

        let performances = recording.performers_string();

        let mut items = Vec::new();

        if tracks.len() == 1 {
            items.push(PlaylistItem::new(
                true,
                &title,
                Some(&performances),
                None,
                &tracks[0].path,
            ));
        } else {
            let mut tracks = tracks.into_iter();
            let first_track = tracks.next().unwrap();

            let track_title = |track: &Track, number: usize| -> String {
                let title = track
                    .works
                    .iter()
                    .map(|w| w.name.get().to_string())
                    .collect::<Vec<String>>()
                    .join(", ");

                if title.is_empty() {
                    format!("Track {number}")
                } else {
                    title
                }
            };

            items.push(PlaylistItem::new(
                true,
                &title,
                Some(&performances),
                Some(&track_title(&first_track, 1)),
                &first_track.path,
            ));

            for (index, track) in tracks.enumerate() {
                items.push(PlaylistItem::new(
                    false,
                    &title,
                    Some(&performances),
                    // track number = track index + 1 (first track) + 1 (zero based)
                    Some(&track_title(&track, index + 2)),
                    &track.path,
                ));
            }
        }

        self.player().append(items);
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
        ] {
            while let Some(widget) = flowbox.first_child() {
                flowbox.remove(&widget);
            }
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
                imp.recordings_flow_box
                    .append(&MusicusRecordingTile::new(recording));
            }

            imp.composers.replace(results.composers);
            imp.performers.replace(results.performers);
            imp.ensembles.replace(results.ensembles);
            imp.works.replace(results.works);
            imp.recordings.replace(results.recordings);
        }
    }
}
