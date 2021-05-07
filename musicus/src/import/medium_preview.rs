use super::medium_editor::MediumEditor;
use crate::navigator::{NavigationHandle, Screen};
use crate::widgets::Widget;
use anyhow::{anyhow, Result};
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use musicus_backend::db::Medium;
use musicus_backend::import::{ImportSession, State};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

/// A dialog for presenting the selected medium when importing music.
pub struct MediumPreview {
    handle: NavigationHandle<()>,
    session: Arc<ImportSession>,
    medium: RefCell<Option<Medium>>,
    widget: gtk::Stack,
    import_button: gtk::Button,
    done_stack: gtk::Stack,
    name_label: gtk::Label,
    medium_box: gtk::Box,
    status_page: libadwaita::StatusPage,
}

impl Screen<(Arc<ImportSession>, Medium), ()> for MediumPreview {
    /// Create a new medium preview screen.
    fn new(
        (session, medium): (Arc<ImportSession>, Medium),
        handle: NavigationHandle<()>,
    ) -> Rc<Self> {
        // Create UI

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/medium_preview.ui");

        get_widget!(builder, gtk::Stack, widget);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::Button, edit_button);
        get_widget!(builder, gtk::Button, import_button);
        get_widget!(builder, gtk::Stack, done_stack);
        get_widget!(builder, gtk::Box, medium_box);
        get_widget!(builder, gtk::Label, name_label);
        get_widget!(builder, libadwaita::StatusPage, status_page);
        get_widget!(builder, gtk::Button, try_again_button);

        let this = Rc::new(Self {
            handle,
            session,
            medium: RefCell::new(None),
            widget,
            import_button,
            done_stack,
            name_label,
            medium_box,
            status_page,
        });

        // Connect signals and callbacks

        back_button.connect_clicked(clone!(@weak this =>  move |_| {
            this.handle.pop(None);
        }));

        edit_button.connect_clicked(clone!(@weak this =>  move |_| {
            spawn!(@clone this, async move {
                let old_medium = this.medium.borrow().clone().unwrap();
                if let Some(medium) = push!(this.handle, MediumEditor, (this.session.clone(), Some(old_medium))).await {
                    this.set_medium(medium);
                }
            });
        }));

        this.import_button
            .connect_clicked(clone!(@weak this =>  move |_| {
                this.widget.set_visible_child_name("loading");

                spawn!(@clone this, async move {
                    match this.import().await {
                        Ok(()) => this.handle.pop(Some(())),
                        Err(err) => {
                            this.widget.set_visible_child_name("error");
                            this.status_page.set_description(Some(&err.to_string()));
                        }
                    }
                });
            }));

        try_again_button.connect_clicked(clone!(@weak this =>  move |_| {
            this.widget.set_visible_child_name("content");
        }));

        this.set_medium(medium);

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
    /// Set a new medium and update the view accordingly.
    fn set_medium(&self, medium: Medium) {
        self.name_label.set_text(&medium.name);

        if let Some(widget) = self.medium_box.first_child() {
            let mut child = widget;

            loop {
                let next_child = child.next_sibling();
                self.medium_box.remove(&child);

                match next_child {
                    Some(widget) => child = widget,
                    None => break,
                }
            }
        }

        let mut last_recording_id = "";
        let mut last_list = None::<gtk::ListBox>;

        let import_tracks = self.session.tracks();

        for track in &medium.tracks {
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
                    let frame = gtk::FrameBuilder::new().margin_bottom(12).build();

                    frame.set_child(Some(list));
                    self.medium_box.append(&frame);
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
                    .subtitle(&import_tracks[track.source_index].name)
                    .margin_start(12)
                    .build();

                list.append(&row);
            }
        }

        if let Some(list) = &last_list {
            let frame = gtk::FrameBuilder::new().margin_bottom(12).build();

            frame.set_child(Some(list));
            self.medium_box.append(&frame);
        }

        self.medium.replace(Some(medium));
    }

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
        let medium = self.medium.borrow();
        let medium = medium.as_ref().ok_or_else(|| anyhow!("No medium set!"))?;

        // Create a new directory in the music library path for the imported medium.

        let music_library_path = self.handle.backend.get_music_library_path().unwrap();

        let directory_name = sanitize_filename::sanitize_with_options(
            &medium.name,
            sanitize_filename::Options {
                windows: true,
                truncate: true,
                replacement: "",
            },
        );

        let directory = PathBuf::from(&directory_name);
        std::fs::create_dir(&music_library_path.join(&directory))?;

        // Copy the tracks to the music library.

        let mut tracks = Vec::new();
        let import_tracks = self.session.tracks();

        for track in &medium.tracks {
            let mut track = track.clone();

            // Set the track path to the new audio file location.

            let import_track = &import_tracks[track.source_index];
            let track_path = directory.join(import_track.path.file_name().unwrap());
            track.path = track_path.to_str().unwrap().to_owned();

            // Copy the corresponding audio file to the music library.
            std::fs::copy(&import_track.path, &music_library_path.join(&track_path))?;

            tracks.push(track);
        }

        // Add the modified medium to the database.

        let medium = Medium {
            id: medium.id.clone(),
            name: medium.name.clone(),
            discid: medium.discid.clone(),
            tracks,
        };

        self.handle
            .backend
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
