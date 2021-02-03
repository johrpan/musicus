use super::source::Source;
use super::track_set_editor::{TrackSetData, TrackSetEditor};
use crate::backend::generate_id;
use crate::backend::{Backend, Medium, Track, TrackSet};
use crate::navigator::{NavigationHandle, Screen};
use crate::widgets::{List, Widget};
use anyhow::{anyhow, Result};
use glib::clone;
use glib::prelude::*;
use gtk::prelude::*;
use gtk_macros::get_widget;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// A dialog for editing metadata while importing music into the music library.
pub struct MediumEditor {
    handle: NavigationHandle<()>,
    source: Rc<Box<dyn Source>>,
    widget: gtk::Stack,
    done_button: gtk::Button,
    done_stack: gtk::Stack,
    done: gtk::Image,
    name_entry: gtk::Entry,
    publish_switch: gtk::Switch,
    track_set_list: Rc<List>,
    track_sets: RefCell<Vec<TrackSetData>>,
}

impl Screen<Rc<Box<dyn Source>>, ()> for MediumEditor {
    /// Create a new medium editor.
    fn new(source: Rc<Box<dyn Source>>, handle: NavigationHandle<()>) -> Rc<Self> {
        // Create UI

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/medium_editor.ui");

        get_widget!(builder, gtk::Stack, widget);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::Button, done_button);
        get_widget!(builder, gtk::Stack, done_stack);
        get_widget!(builder, gtk::Image, done);
        get_widget!(builder, gtk::Entry, name_entry);
        get_widget!(builder, gtk::Switch, publish_switch);
        get_widget!(builder, gtk::Button, add_button);
        get_widget!(builder, gtk::Frame, frame);

        let list = List::new();
        frame.set_child(Some(&list.widget));

        let this = Rc::new(Self {
            handle,
            source,
            widget,
            done_button,
            done_stack,
            done,
            name_entry,
            publish_switch,
            track_set_list: list,
            track_sets: RefCell::new(Vec::new()),
        });

        // Connect signals and callbacks

        back_button.connect_clicked(clone!(@weak this => move |_| {
            this.handle.pop(None);
        }));

        this.done_button.connect_clicked(clone!(@weak this => move |_| {
            this.widget.set_visible_child_name("loading");
            spawn!(@clone this, async move {
                match this.save().await {
                    Ok(_) => (),
                    Err(err) => {
                        // TODO: Display errors.
                        println!("{:?}", err);
                    }
                }
            });
        }));

        add_button.connect_clicked(clone!(@weak this => move |_| {
            spawn!(@clone this, async move {
                if let Some(track_set) = push!(this.handle, TrackSetEditor, Rc::clone(&this.source)).await {
                    let length = {
                        let mut track_sets = this.track_sets.borrow_mut();
                        track_sets.push(track_set);
                        track_sets.len()
                    };

                    this.track_set_list.update(length);
                }
            });
        }));

        this.track_set_list.set_make_widget_cb(clone!(@weak this => move |index| {
            let track_set = &this.track_sets.borrow()[index];

            let title = track_set.recording.work.get_title();
            let subtitle = track_set.recording.get_performers();

            let edit_image = gtk::Image::from_icon_name(Some("document-edit-symbolic"));
            let edit_button = gtk::Button::new();
            edit_button.set_has_frame(false);
            edit_button.set_valign(gtk::Align::Center);
            edit_button.set_child(Some(&edit_image));

            let row = libadwaita::ActionRow::new();
            row.set_activatable(true);
            row.set_title(Some(&title));
            row.set_subtitle(Some(&subtitle));
            row.add_suffix(&edit_button);
            row.set_activatable_widget(Some(&edit_button));

            edit_button.connect_clicked(clone!(@weak this => move |_| {
                // TODO: Implement editing.
            }));

            row.upcast()
        }));

        spawn!(@clone this, async move {
            match this.source.copy().await {
                Err(error) => {
                    // TODO: Present error.
                    println!("Failed to copy source: {}", error);
                },
                Ok(_) => {
                    this.done_stack.set_visible_child(&this.done);
                    this.done_button.set_sensitive(true);
                }
            }
        });

        this
    }
}

impl MediumEditor {
    /// Save the medium and possibly upload it to the server.
    async fn save(&self) -> Result<()> {
        let name = self.name_entry.get_text().unwrap().to_string();

        // Create a new directory in the music library path for the imported medium.

        let mut path = self.handle.backend.get_music_library_path().unwrap().clone();
        path.push(&name);
        std::fs::create_dir(&path)?;

        // Convert the track set data to real track sets.

        let mut track_sets = Vec::new();
        let source_tracks = self.source.tracks().ok_or_else(|| anyhow!("Tracks not loaded!"))?;

        for track_set_data in &*self.track_sets.borrow() {
            let mut tracks = Vec::new();

            for track_data in &track_set_data.tracks {
                // Copy the corresponding audio file to the music library.

                let track_source = &source_tracks[track_data.track_source];

                let mut track_path = path.clone();
                track_path.push(track_source.path.file_name().unwrap());

                std::fs::copy(&track_source.path, &track_path)?;

                // Create the real track.

                let track = Track {
                    work_parts: track_data.work_parts.clone(),
                    path: track_path.to_str().unwrap().to_owned(),
                };

                tracks.push(track);
            }

            let track_set = TrackSet {
                recording: track_set_data.recording.clone(),
                tracks,
            };

            track_sets.push(track_set);
        }

        let medium = Medium {
            id: generate_id(),
            name: self.name_entry.get_text().unwrap().to_string(),
            discid: self.source.discid(),
            tracks: track_sets,
        };

        let upload = self.publish_switch.get_active();
        if upload {
            self.handle.backend.post_medium(&medium).await?;
        }

        self.handle.backend
            .db()
            .update_medium(medium.clone())
            .await?;

        self.handle.backend.library_changed();

        Ok(())
    }
}

impl Widget for MediumEditor {
    fn get_widget(&self) -> gtk::Widget {
        self.widget.clone().upcast()
    }
}
