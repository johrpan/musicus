use super::track_set_editor::{TrackData, TrackSetData, TrackSetEditor};
use crate::navigator::{NavigationHandle, Screen};
use crate::widgets::{List, Widget};

use adw::prelude::*;
use anyhow::Result;
use glib::clone;
use gtk_macros::get_widget;
use musicus_backend::db::{generate_id, Medium, Track};
use musicus_backend::import::ImportSession;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

/// A dialog for editing metadata while importing music into the music library.
pub struct MediumEditor {
    handle: NavigationHandle<Medium>,
    session: Arc<ImportSession>,
    widget: gtk::Stack,
    done_button: gtk::Button,
    name_row: adw::EntryRow,
    status_page: adw::StatusPage,
    track_set_list: Rc<List>,
    track_sets: RefCell<Vec<TrackSetData>>,
}

impl Screen<(Arc<ImportSession>, Option<Medium>), Medium> for MediumEditor {
    /// Create a new medium editor.
    fn new(
        (session, medium): (Arc<ImportSession>, Option<Medium>),
        handle: NavigationHandle<Medium>,
    ) -> Rc<Self> {
        // Create UI

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/medium_editor.ui");

        get_widget!(builder, gtk::Stack, widget);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::Button, done_button);
        get_widget!(builder, adw::EntryRow, name_row);
        get_widget!(builder, gtk::Button, add_button);
        get_widget!(builder, gtk::Frame, frame);
        get_widget!(builder, adw::StatusPage, status_page);
        get_widget!(builder, gtk::Button, try_again_button);
        get_widget!(builder, gtk::Button, cancel_button);

        let list = List::new();
        frame.set_child(Some(&list.widget));

        let this = Rc::new(Self {
            handle,
            session,
            widget,
            done_button,
            name_row,
            status_page,
            track_set_list: list,
            track_sets: RefCell::new(Vec::new()),
        });

        // Connect signals and callbacks

        back_button.connect_clicked(clone!(@weak this =>  move |_| {
            this.handle.pop(None);
        }));

        this.done_button
            .connect_clicked(clone!(@weak this =>  move |_| {
                this.widget.set_visible_child_name("loading");
                spawn!(@clone this, async move {
                    match this.save().await {
                        Ok(medium) => this.handle.pop(Some(medium)),
                        Err(err) => {
                            this.status_page.set_description(Some(&err.to_string()));
                            this.widget.set_visible_child_name("error");
                        }
                    }
                });
            }));

        this.name_row
            .connect_changed(clone!(@weak this =>  move |_| this.validate()));

        add_button.connect_clicked(clone!(@weak this =>  move |_| {
            spawn!(@clone this, async move {
                if let Some(track_set) = push!(this.handle, TrackSetEditor, Arc::clone(&this.session)).await {
                    let length = {
                        let mut track_sets = this.track_sets.borrow_mut();
                        track_sets.push(track_set);
                        track_sets.len()
                    };

                    this.track_set_list.update(length);
                    this.validate();
                }
            });
        }));

        this.track_set_list.set_make_widget_cb(
            clone!(@weak this =>  @default-panic, move |index| {
                let track_set = &this.track_sets.borrow()[index];

                let title = track_set.recording.work.get_title();
                let subtitle = track_set.recording.get_performers();

                let edit_image = gtk::Image::from_icon_name("document-edit-symbolic");
                let edit_button = gtk::Button::new();
                edit_button.set_has_frame(false);
                edit_button.set_valign(gtk::Align::Center);
                edit_button.set_child(Some(&edit_image));

                let row = adw::ActionRow::builder()
                    .focusable(false)
                    .title(title)
                    .subtitle(subtitle)
                    .activatable_widget(&edit_button)
                    .build();

                row.add_suffix(&edit_button);

                edit_button.connect_clicked(clone!(@weak this =>  move |_| {
                    // TODO: Implement editing.
                }));

                row.upcast()
            }),
        );

        try_again_button.connect_clicked(clone!(@weak this =>  move |_| {
            this.widget.set_visible_child_name("content");
        }));

        cancel_button.connect_clicked(clone!(@weak this =>  move |_| {
            this.handle.pop(None);
        }));

        // Initialize, if necessary.

        if let Some(medium) = medium {
            this.name_row.set_text(&medium.name);

            let mut track_sets: Vec<TrackSetData> = Vec::new();

            for track in medium.tracks {
                let track_data = TrackData {
                    track_source: track.source_index,
                    work_parts: track.work_parts,
                };

                if let Some(track_set) = track_sets.last_mut() {
                    if track.recording.id == track_set.recording.id {
                        track_set.tracks.push(track_data);
                        continue;
                    }
                }

                track_sets.push(TrackSetData {
                    recording: track.recording,
                    tracks: vec![track_data],
                });
            }

            let length = track_sets.len();
            this.track_sets.replace(track_sets);
            this.track_set_list.update(length);
        }

        this.validate();

        this
    }
}

impl MediumEditor {
    /// Validate inputs and enable/disable saving.
    fn validate(&self) {
        self.done_button.set_sensitive(
            !self.name_row.text().is_empty() && !self.track_sets.borrow().is_empty(),
        );
    }

    /// Create the medium.
    async fn save(&self) -> Result<Medium> {
        // Convert the track set data to real track sets.

        let mut tracks = Vec::new();

        for track_set_data in &*self.track_sets.borrow() {
            for track_data in &track_set_data.tracks {
                let track = Track::new(
                    track_set_data.recording.clone(),
                    track_data.work_parts.clone(),
                    track_data.track_source,
                    String::new(),
                );

                tracks.push(track);
            }
        }

        let medium = Medium::new(
            generate_id(),
            self.name_row.text().to_string(),
            Some(self.session.source_id().to_owned()),
            tracks,
        );

        // The medium is not added to the database, because the track paths are not known until the
        // medium is actually imported into the music library. This step will be handled by the
        // medium preview dialog.

        Ok(medium)
    }
}

impl Widget for MediumEditor {
    fn get_widget(&self) -> gtk::Widget {
        self.widget.clone().upcast()
    }
}
