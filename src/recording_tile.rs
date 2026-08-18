use std::cell::OnceCell;

use adw::prelude::*;
use gettextrs::gettext;
use gtk::{gio, glib, subclass::prelude::*};

use musicus_library::db::models::{Recording, Work};

use crate::{editor::recording::RecordingEditor, library::Library, player::Player};

mod imp {
    use super::*;
    use crate::{editor::tracks::TracksEditor, util};

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/recording_tile.blp")]
    pub struct RecordingTile {
        #[template_child]
        pub title_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub subtitle_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub description_label: TemplateChild<gtk::Label>,

        pub toast_overlay: OnceCell<adw::ToastOverlay>,
        pub navigation: OnceCell<adw::NavigationView>,
        pub library: OnceCell<Library>,
        pub player: OnceCell<Player>,
        pub recording: OnceCell<Recording>,

        /// The work whose page this tile is shown on.
        pub work: OnceCell<Option<Work>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RecordingTile {
        const NAME: &'static str = "MusicusRecordingTile";
        type Type = super::RecordingTile;
        type ParentType = gtk::FlowBoxChild;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for RecordingTile {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj().to_owned();
            let append_action = gio::ActionEntry::builder("add-to-playlist")
                .activate(move |_, _, _| {
                    let player = obj.imp().player.get().unwrap();
                    let recording = obj.imp().recording.get().unwrap();

                    let playlist = match obj.imp().work.get().unwrap() {
                        Some(work) => player.recording_to_playlist_for_work(recording, work),
                        None => player.recording_to_playlist(recording),
                    };

                    if let Err(err) = player.append(playlist) {
                        log::error!("Failed to add recording to playlist: {err:?}");
                    }
                })
                .build();

            let obj = self.obj().to_owned();
            let edit_recording_action = gio::ActionEntry::builder("edit-recording")
                .activate(move |_, _, _| {
                    obj.imp()
                        .navigation
                        .get()
                        .unwrap()
                        .push(&RecordingEditor::new(
                            obj.imp().navigation.get().unwrap(),
                            obj.imp().library.get().unwrap(),
                            Some(obj.imp().recording.get().unwrap()),
                        ));
                })
                .build();

            let obj = self.obj().to_owned();
            let edit_tracks_action = gio::ActionEntry::builder("edit-tracks")
                .activate(move |_, _, _| {
                    obj.imp().navigation.get().unwrap().push(&TracksEditor::new(
                        obj.imp().toast_overlay.get().unwrap(),
                        obj.imp().navigation.get().unwrap(),
                        obj.imp().library.get().unwrap(),
                        Some(obj.imp().recording.get().unwrap().clone()),
                    ));
                })
                .build();

            let obj = self.obj().to_owned();
            let delete_action = gio::ActionEntry::builder("delete")
                .activate(move |_, _, _| {
                    let dialog = adw::AlertDialog::builder()
                        .heading(gettext("Delete recording?"))
                        .body(gettext("The recording will be removed from your music library and the corresponding audio files will be deleted. This action cannot be undone."))
                        .build();

                    dialog.add_response("delete", &gettext("Delete"));
                    dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
                    dialog.add_response("cancel", &gettext("Cancel"));
                    dialog.set_default_response(Some("cancel"));
                    dialog.set_close_response("cancel");

                    let obj = obj.clone();
                    glib::spawn_future_local(async move {
                        if dialog.choose_future(Some(&obj)).await == "delete" {
                            if let Err(err) = obj.imp().library.get().unwrap().delete_recording_and_tracks(&obj.recording().recording_id) {
                                util::error_toast("Failed to delete recording", err, obj.imp().toast_overlay.get().unwrap());
                            }
                        }
                    });
                })
                .build();

            let actions = gio::SimpleActionGroup::new();
            actions.add_action_entries([
                append_action,
                edit_recording_action,
                edit_tracks_action,
                delete_action,
            ]);
            self.obj().insert_action_group("recording", Some(&actions));
        }
    }

    impl WidgetImpl for RecordingTile {}
    impl FlowBoxChildImpl for RecordingTile {}
}

glib::wrapper! {
    pub struct RecordingTile(ObjectSubclass<imp::RecordingTile>)
        @extends gtk::FlowBoxChild, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl RecordingTile {
    pub fn new(
        toast_overlay: &adw::ToastOverlay,
        navigation: &adw::NavigationView,
        library: &Library,
        player: &Player,
        recording: &Recording,
        work: Option<&Work>,
    ) -> Self {
        let obj: Self = glib::Object::new();
        let imp = obj.imp();

        let performers = recording.performers_string();

        let pieces: Vec<String> = if work.is_some() {
            Some(performers)
                .filter(|s| !s.is_empty())
                .into_iter()
                .collect()
        } else {
            [
                Some(recording.work.name.get().to_string()),
                recording.work.composers_string(),
                Some(performers).filter(|s| !s.is_empty()),
            ]
            .into_iter()
            .flatten()
            .collect()
        };

        let labels = [
            &imp.title_label,
            &imp.subtitle_label,
            &imp.description_label,
        ];

        for (label, text) in labels.iter().zip(&pieces) {
            label.set_label(text);
            label.set_visible(true);
        }

        for label in &labels[pieces.len()..] {
            label.set_visible(false);
        }

        imp.toast_overlay.set(toast_overlay.to_owned()).unwrap();
        imp.navigation.set(navigation.to_owned()).unwrap();
        imp.library.set(library.to_owned()).unwrap();
        imp.player.set(player.to_owned()).unwrap();
        imp.recording.set(recording.to_owned()).unwrap();
        imp.work.set(work.cloned()).unwrap();

        obj
    }

    pub fn recording(&self) -> &Recording {
        self.imp().recording.get().unwrap()
    }
}
