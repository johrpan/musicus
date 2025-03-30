use std::cell::OnceCell;

use adw::prelude::*;
use gettextrs::gettext;
use gtk::{gio, glib, subclass::prelude::*};

use crate::{
    db::models::Recording, editor::recording::RecordingEditor, library::Library, player::Player,
};

mod imp {
    use super::*;
    use crate::{editor::tracks::TracksEditor, util};

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/recording_tile.blp")]
    pub struct RecordingTile {
        #[template_child]
        pub composer_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub work_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub performances_label: TemplateChild<gtk::Label>,

        pub toast_overlay: OnceCell<adw::ToastOverlay>,
        pub navigation: OnceCell<adw::NavigationView>,
        pub library: OnceCell<Library>,
        pub player: OnceCell<Player>,
        pub recording: OnceCell<Recording>,
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
                    let playlist = player.recording_to_playlist(obj.imp().recording.get().unwrap());
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
                            Some(&obj.imp().recording.get().unwrap()),
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
                        .heading(&gettext("Delete recording?"))
                        .body(&gettext("The recording will be removed from your music library and the corresponding audio files will be deleted. This action cannot be undone."))
                        .build();

                    dialog.add_response("delete", &gettext("Delete"));
                    dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
                    dialog.add_response("cancel", &gettext("Cancel"));
                    dialog.set_default_response(Some("cancel"));
                    dialog.set_close_response("cancel");

                    let obj = obj.clone();
                    glib::spawn_future_local(async move {
                        if dialog.choose_future(&obj).await == "delete" {
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
        @extends gtk::Widget, gtk::FlowBoxChild;
}

impl RecordingTile {
    pub fn new(
        toast_overlay: &adw::ToastOverlay,
        navigation: &adw::NavigationView,
        library: &Library,
        player: &Player,
        recording: &Recording,
    ) -> Self {
        let obj: Self = glib::Object::new();
        let imp = obj.imp();

        imp.work_label.set_label(&recording.work.name.get());
        imp.composer_label.set_label(
            &recording
                .work
                .composers_string()
                .unwrap_or_else(|| gettext("No composers")),
        );
        imp.performances_label
            .set_label(&recording.performers_string());

        imp.toast_overlay.set(toast_overlay.to_owned()).unwrap();
        imp.navigation.set(navigation.to_owned()).unwrap();
        imp.library.set(library.to_owned()).unwrap();
        imp.player.set(player.to_owned()).unwrap();
        imp.recording.set(recording.to_owned()).unwrap();

        obj
    }

    pub fn recording(&self) -> &Recording {
        self.imp().recording.get().unwrap()
    }
}
