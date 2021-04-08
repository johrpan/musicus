use crate::navigator::{NavigationHandle, Screen};
use crate::widgets::Widget;
use anyhow::Result;
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use musicus_backend::db::Medium;
use musicus_backend::import::{ImportSession, State};
use std::rc::Rc;
use std::sync::Arc;

/// A dialog for presenting the selected medium when importing music.
pub struct MediumPreview {
    handle: NavigationHandle<()>,
    session: Arc<ImportSession>,
    medium: Medium,
    widget: gtk::Box,
    import_button: gtk::Button,
    done_stack: gtk::Stack,
}

impl Screen<(Arc<ImportSession>, Medium), ()> for MediumPreview {
    /// Create a new medium preview screen.
    fn new((session, medium): (Arc<ImportSession>, Medium), handle: NavigationHandle<()>) -> Rc<Self> {
        // Create UI

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/medium_preview.ui");

        get_widget!(builder, gtk::Box, widget);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::Button, import_button);
        get_widget!(builder, gtk::Stack, done_stack);
        get_widget!(builder, gtk::Box, medium_box);
        get_widget!(builder, gtk::Label, name_label);

        let this = Rc::new(Self {
            handle,
            session,
            medium,
            widget,
            import_button,
            done_stack,
        });

        // Connect signals and callbacks

        back_button.connect_clicked(clone!(@weak this => move |_| {
            this.handle.pop(None);
        }));

        this.import_button.connect_clicked(clone!(@weak this => move |_| {
            spawn!(@clone this, async move {
                this.import().await.unwrap();
                this.handle.pop(Some(()));
            });
        }));

        // Populate the widget

        name_label.set_text(&this.medium.name);

        let mut last_recording_id = "";
        let mut last_list = None::<gtk::ListBox>;

        for track in &this.medium.tracks {
            if track.recording.id != last_recording_id {
                last_recording_id = &track.recording.id;

                let list = gtk::ListBoxBuilder::new()
                    .selection_mode(gtk::SelectionMode::None)
                    .build();

                let header = libadwaita::ActionRowBuilder::new()
                    .activatable(false)
                    .title(&track.recording.work.get_title())
                    .subtitle(&track.recording.get_performers())
                    .build();

                list.append(&header);

                if let Some(list) = &last_list {
                    let frame = gtk::FrameBuilder::new()
                        .margin_bottom(12)
                        .build();

                    frame.set_child(Some(list));
                    medium_box.append(&frame);

                    last_list = None;
                }

                last_list = Some(list);
            }

            if let Some(list) = &last_list {
                let mut parts = Vec::<String>::new();
                for part in &track.work_parts {
                    parts.push(track.recording.work.parts[*part].title.clone());
                }

                let title = if parts.is_empty() {
                    gettext("Unknown")
                } else {
                    parts.join(", ")
                };

                let row = libadwaita::ActionRowBuilder::new()
                    .activatable(false)
                    .title(&title)
                    .subtitle(&track.path)
                    .margin_start(12)
                    .build();

                list.append(&row);
            }
        }

        if let Some(list) = &last_list {
            let frame = gtk::FrameBuilder::new()
                .margin_bottom(12)
                .build();

            frame.set_child(Some(list));
            medium_box.append(&frame);
        }

        this.handle_state(&this.session.state());
        spawn!(@clone this, async move {
            loop {
                let state = this.session.state_change().await;
                this.handle_state(&state);

                match state {
                    State::Ready | State::Error => break,
                    _ => (),
                }
            }
        });

        this
    }
}

impl MediumPreview {
    /// Handle a state change of the import process.
    fn handle_state(&self, state: &State) {
        match state {
            State::Waiting => todo!("This shouldn't happen."),
            State::Copying => self.done_stack.set_visible_child_name("loading"),
            State::Ready => {
                self.done_stack.set_visible_child_name("ready");
                self.import_button.set_sensitive(true);
            }
            State::Error => todo!("Import error!"),
        }
    }

    /// Copy the tracks to the music library and add the medium to the database.
    async fn import(&self) -> Result<()> {
        // Create a new directory in the music library path for the imported medium.

        let mut path = self.handle.backend.get_music_library_path().unwrap().clone();
        path.push(&self.medium.name);
        std::fs::create_dir(&path)?;

        // Copy the tracks to the music library.

        let mut tracks = Vec::new();
        let import_tracks = self.session.tracks();

        for (index, track) in self.medium.tracks.iter().enumerate() {
            let mut track = track.clone();

            // Set the track path to the new audio file location.

            let import_track = &import_tracks[index];
            let mut track_path = path.clone();
            track_path.push(import_track.path.file_name().unwrap());
            track.path = track_path.to_str().unwrap().to_owned();

            // Copy the corresponding audio file to the music library.
            std::fs::copy(&import_track.path, &track_path)?;

            tracks.push(track);
        }

        // Add the modified medium to the database.

        let medium = Medium {
            id: self.medium.id.clone(),
            name: self.medium.name.clone(),
            discid: self.medium.discid.clone(),
            tracks,
        };

        self.handle.backend
            .db()
            .update_medium(medium.clone())
            .await?;

        self.handle.backend.library_changed();

        Ok(())
    }
}

impl Widget for MediumPreview {
    fn get_widget(&self) -> gtk::Widget {
        self.widget.clone().upcast()
    }
}
