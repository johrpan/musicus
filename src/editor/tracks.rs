mod parts_popover;
mod track_row;

use std::{
    cell::{OnceCell, RefCell},
    path::PathBuf,
};

use adw::{prelude::*, subclass::prelude::*};
use anyhow::{anyhow, Result};
use gettextrs::gettext;
use gtk::{
    gio,
    glib::{self, clone, subclass::Signal, Properties},
};
use once_cell::sync::Lazy;
use track_row::{TrackLocation, TracksEditorTrackData, TracksEditorTrackRow};

use musicus_library::{
    db::models::{Recording, Track, Work},
    library::TrackUpdate,
};

use crate::{
    editor::recording::RecordingEditor, library::Library,
    selector::recording::RecordingSelectorPopover, util,
};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, Properties)]
    #[properties(wrapper_type = super::TracksEditor)]
    #[template(file = "data/ui/editor/tracks.blp")]
    pub struct TracksEditor {
        #[property(get, construct_only)]
        pub toast_overlay: OnceCell<adw::ToastOverlay>,
        #[property(get, construct_only)]
        pub navigation: OnceCell<adw::NavigationView>,
        #[property(get, construct_only)]
        pub library: OnceCell<Library>,

        pub recording: RefCell<Option<Recording>>,
        pub recordings_popover: OnceCell<RecordingSelectorPopover>,
        pub track_rows: RefCell<Vec<TracksEditorTrackRow>>,
        pub removed_tracks: RefCell<Vec<Track>>,

        #[template_child]
        pub recording_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub select_recording_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub tracks_label: TemplateChild<gtk::Label>,
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
                    .param_types([glib::BoxedAnyObject::static_type()])
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
            recordings_popover.connect_create(move |_, prefill| {
                let editor =
                    RecordingEditor::new(obj.imp().navigation.get().unwrap(), &obj.library(), None);

                editor.prefill(&prefill);

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

    impl NavigationPageImpl for TracksEditor {
        fn shown(&self) {
            self.parent_shown();

            if self.recording.borrow().is_none() {
                self.obj().select_recording();
            }
        }
    }
}

glib::wrapper! {
    pub struct TracksEditor(ObjectSubclass<imp::TracksEditor>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

#[gtk::template_callbacks]
impl TracksEditor {
    pub fn new(
        toast_overlay: &adw::ToastOverlay,
        navigation: &adw::NavigationView,
        library: &Library,
        recording: Option<Recording>,
    ) -> Self {
        let obj: Self = glib::Object::builder()
            .property("toast-overlay", toast_overlay)
            .property("navigation", navigation)
            .property("library", library)
            .build();

        if let Some(recording) = recording {
            obj.imp().save_row.set_title(&gettext("_Save changes"));
            obj.set_recording(recording);
        }

        obj
    }

    #[template_callback]
    fn select_recording(&self) {
        self.imp().recordings_popover.get().unwrap().popup();
    }

    #[template_callback]
    async fn add_files(&self) {
        let dialog = gtk::FileDialog::builder()
            .title(gettext("Select audio files"))
            .modal(true)
            .build();

        let root = self.root();
        let window = root.as_ref().and_then(|r| r.downcast_ref::<gtk::Window>());

        let obj = self.clone();
        match dialog.open_multiple_future(window).await {
            Err(err) => {
                if !err.matches(gtk::DialogError::Dismissed) {
                    log::error!("File selection failed: {err:?}");
                }
            }
            Ok(files) => {
                for file in &files {
                    // A selected item that is not a local file (or that cannot
                    // be converted to a path) is skipped rather than fatal.
                    match file
                        .ok()
                        .and_then(|file| file.downcast::<gio::File>().ok())
                        .and_then(|file| file.path())
                    {
                        Some(path) => obj.add_file(path),
                        None => log::warn!("Skipping a selected file with no local path"),
                    }
                }
            }
        }
    }

    fn set_recording(&self, recording: Recording) {
        self.imp()
            .recording_row
            .set_title(&recording.work.to_string());
        self.imp()
            .recording_row
            .set_subtitle(&recording.performers_string());

        // Remove previously added track rows. This is not ideal because the user might be under
        // the impression that changing the recording will allow to transfer tracks to it. But:
        // What would happen to the old recording's tracks? What would happen with previously
        // selected work parts?
        for track_row in self.imp().track_rows.borrow_mut().drain(..) {
            self.imp().track_list.remove(&track_row);
        }

        // Forget previously removed tracks (see above).
        self.imp().removed_tracks.borrow_mut().clear();

        let tracks = match self.library().tracks_for_recording(&recording.recording_id) {
            Ok(tracks) => tracks,
            Err(err) => {
                util::error_toast(
                    "Failed to load the tracks of this recording",
                    err,
                    &self.toast_overlay(),
                );
                return;
            }
        };

        if !tracks.is_empty() {
            self.imp().save_row.set_title(&gettext("_Save changes"));

            for track in tracks {
                self.add_track_row(
                    recording.clone(),
                    TracksEditorTrackData {
                        location: TrackLocation::Library(track.clone()),
                        parts: track.works,
                    },
                );
            }
        }

        self.imp().tracks_label.set_sensitive(true);
        self.imp().track_list.set_sensitive(true);
        self.imp().save_row.set_sensitive(true);

        self.imp().recording.replace(Some(recording));
    }

    fn add_file(&self, path: PathBuf) {
        if let Some(recording) = &*self.imp().recording.borrow() {
            let parts_taken = {
                self.imp()
                    .track_rows
                    .borrow()
                    .iter()
                    .flat_map(|t| t.track_data().parts.clone())
                    .collect::<Vec<Work>>()
            };

            let next_part = recording
                .work
                .parts
                .iter()
                .find(|p| !parts_taken.contains(p))
                .into_iter()
                .cloned()
                .collect::<Vec<Work>>();

            self.add_track_row(
                recording.to_owned(),
                TracksEditorTrackData {
                    location: TrackLocation::System(path),
                    parts: next_part,
                },
            );
        } else {
            log::warn!("Tried to add track row without recording selected");
        }
    }

    fn add_track_row(&self, recording: Recording, track_data: TracksEditorTrackData) {
        let track_row =
            TracksEditorTrackRow::new(&self.navigation(), &self.library(), recording, track_data);

        track_row.connect_move(clone!(
            #[weak(rename_to = this)]
            self,
            move |target, source| {
                let mut track_rows = this.imp().track_rows.borrow_mut();
                if let Some(index) = track_rows.iter().position(|p| p == target) {
                    this.imp().track_list.remove(&source);
                    track_rows.retain(|p| p != &source);
                    this.imp().track_list.insert(&source, index as i32);
                    track_rows.insert(index, source);
                }
            }
        ));

        track_row.connect_remove(clone!(
            #[weak(rename_to = this)]
            self,
            move |row| {
                if let TrackLocation::Library(track) = row.track_data().location {
                    this.imp().removed_tracks.borrow_mut().push(track);
                }

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
        if let Some(recording) = &*self.imp().recording.borrow() {
            match self.import(recording) {
                Ok(()) => {
                    self.navigation().pop();
                }
                Err(err) => {
                    // Stay on the page: nothing was saved, so the user can fix
                    // whatever went wrong and try again.
                    util::error_toast("Failed to import tracks", err, &self.toast_overlay());
                }
            }
        }
    }

    fn import(&self, recording: &Recording) -> Result<()> {
        // Describe the whole batch before saving any of it, so that a track
        // without a file aborts the save before anything has been written.
        let tracks = {
            let mut tracks = Vec::new();

            for track_row in self.imp().track_rows.borrow().iter() {
                let track_data = track_row.track_data();

                tracks.push(match track_data.location {
                    TrackLocation::Undefined => {
                        return Err(anyhow!("A track has no file assigned to it."));
                    }
                    TrackLocation::Library(track) => TrackUpdate::Existing {
                        track_id: track.track_id,
                        works: track_data.parts,
                    },
                    TrackLocation::System(path) => TrackUpdate::New {
                        path,
                        works: track_data.parts,
                    },
                });
            }

            tracks
        };

        // Nothing may be borrowed while the library is saving: it notifies its
        // subscribers, which can call back into this page.
        let removed_tracks = self.imp().removed_tracks.borrow().clone();

        self.library()
            .set_recording_tracks(&recording.recording_id, tracks, &removed_tracks)?;

        // Only once everything has been saved may the pending changes be
        // forgotten, so that a failed save stays retryable.
        self.imp().removed_tracks.borrow_mut().clear();

        for track_row in self.imp().track_rows.borrow_mut().drain(..) {
            self.imp().track_list.remove(&track_row);
        }

        Ok(())
    }
}
