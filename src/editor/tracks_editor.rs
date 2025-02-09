use super::tracks_editor_track_row::{PathType, TracksEditorTrackData};
use crate::{
    db::models::Recording,
    editor::{
        recording_editor::MusicusRecordingEditor,
        recording_selector_popover::RecordingSelectorPopover,
        tracks_editor_track_row::TracksEditorTrackRow,
    },
    library::MusicusLibrary,
};

use adw::{prelude::*, subclass::prelude::*};
use gettextrs::gettext;
use gtk::{
    gio,
    glib::{self, clone, subclass::Signal, Properties},
};
use once_cell::sync::Lazy;

use std::{
    cell::{OnceCell, RefCell},
    path::PathBuf,
};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, Properties)]
    #[properties(wrapper_type = super::TracksEditor)]
    #[template(file = "data/ui/tracks_editor.blp")]
    pub struct TracksEditor {
        #[property(get, construct_only)]
        pub navigation: OnceCell<adw::NavigationView>,
        #[property(get, construct_only)]
        pub library: OnceCell<MusicusLibrary>,

        pub recording: RefCell<Option<Recording>>,
        pub recordings_popover: OnceCell<RecordingSelectorPopover>,
        pub track_rows: RefCell<Vec<TracksEditorTrackRow>>,

        #[template_child]
        pub recording_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub select_recording_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub track_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub save_row: TemplateChild<adw::ButtonRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TracksEditor {
        const NAME: &'static str = "MusicusTracksEditor";
        type Type = super::TracksEditor;
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
    impl ObjectImpl for TracksEditor {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("created")
                    .param_types([Recording::static_type()])
                    .build()]
            });

            SIGNALS.as_ref()
        }

        fn constructed(&self) {
            self.parent_constructed();

            let recordings_popover = RecordingSelectorPopover::new(self.library.get().unwrap());

            let obj = self.obj().clone();
            recordings_popover.connect_selected(move |_, recording| {
                obj.set_recording(recording);
            });

            let obj = self.obj().clone();
            recordings_popover.connect_create(move |_| {
                let editor = MusicusRecordingEditor::new(
                    obj.imp().navigation.get().unwrap(),
                    &obj.library(),
                    None,
                );

                editor.connect_created(clone!(
                    #[weak]
                    obj,
                    move |_, recording| {
                        obj.set_recording(recording);
                    }
                ));

                obj.imp().navigation.get().unwrap().push(&editor);
            });

            self.select_recording_box.append(&recordings_popover);
            self.recordings_popover.set(recordings_popover).unwrap();
        }
    }

    impl WidgetImpl for TracksEditor {}
    impl NavigationPageImpl for TracksEditor {}
}

glib::wrapper! {
    pub struct TracksEditor(ObjectSubclass<imp::TracksEditor>)
        @extends gtk::Widget, adw::NavigationPage;
}

#[gtk::template_callbacks]
impl TracksEditor {
    pub fn new(
        navigation: &adw::NavigationView,
        library: &MusicusLibrary,
        recording: Option<Recording>,
    ) -> Self {
        let obj: Self = glib::Object::builder()
            .property("navigation", navigation)
            .property("library", library)
            .build();

        if let Some(recording) = recording {
            obj.imp().save_row.set_title(&gettext("Save changes"));
            obj.set_recording(recording);
        }

        obj
    }

    #[template_callback]
    fn select_recording(&self, _: &adw::ActionRow) {
        self.imp().recordings_popover.get().unwrap().popup();
    }

    #[template_callback]
    async fn add_files(&self, _: &adw::ActionRow) {
        let dialog = gtk::FileDialog::builder()
            .title(gettext("Select audio files"))
            .modal(true)
            .build();

        let root = self.root();
        let window = root
            .as_ref()
            .and_then(|r| r.downcast_ref::<gtk::Window>())
            .unwrap();

        let obj = self.clone();
        match dialog.open_multiple_future(Some(window)).await {
            Err(err) => {
                if !err.matches(gtk::DialogError::Dismissed) {
                    log::error!("File selection failed: {err}");
                }
            }
            Ok(files) => {
                for file in &files {
                    obj.add_file(
                        file.unwrap()
                            .downcast::<gio::File>()
                            .unwrap()
                            .path()
                            .unwrap(),
                    );
                }
            }
        }
    }

    fn set_recording(&self, recording: Recording) {
        self.imp().recording_row.set_title(&format!(
            "{}: {}",
            recording.work.composers_string(),
            recording.work.name.get(),
        ));

        self.imp()
            .recording_row
            .set_subtitle(&recording.performers_string());

        for track in self
            .library()
            .tracks_for_recording(&recording.recording_id)
            .unwrap()
        {
            self.add_track_row(TracksEditorTrackData {
                track_id: Some(track.track_id),
                path: PathType::Library(track.path),
                works: track.works,
            });
        }

        self.imp().recording.replace(Some(recording));
    }

    fn add_file(&self, path: PathBuf) {
        self.add_track_row(TracksEditorTrackData {
            track_id: None,
            path: PathType::System(path),
            works: Vec::new(),
        });
    }

    fn add_track_row(&self, track_data: TracksEditorTrackData) {
        let track_row = TracksEditorTrackRow::new(&self.navigation(), &self.library(), track_data);

        track_row.connect_remove(clone!(
            #[weak(rename_to = this)]
            self,
            move |row| {
                this.imp().track_list.remove(row);
                this.imp().track_rows.borrow_mut().retain(|p| p != row);
            }
        ));

        self.imp()
            .track_list
            .insert(&track_row, self.imp().track_rows.borrow().len() as i32);

        self.imp().track_rows.borrow_mut().push(track_row);
    }

    #[template_callback]
    fn save(&self) {
        // TODO

        self.navigation().pop();
    }
}
