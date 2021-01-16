use super::disc_source::DiscSource;
use super::track_set_editor::{TrackSetData, TrackSetEditor};
use crate::database::{generate_id, Medium, Track, TrackSet};
use crate::backend::Backend;
use crate::widgets::{Navigator, NavigatorScreen};
use crate::widgets::new_list::List;
use anyhow::Result;
use glib::clone;
use glib::prelude::*;
use gtk::prelude::*;
use gtk_macros::get_widget;
use libhandy::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// A dialog for editing metadata while importing music into the music library.
pub struct MediumEditor {
    backend: Rc<Backend>,
    source: Rc<DiscSource>,
    widget: gtk::Stack,
    done_button: gtk::Button,
    done_stack: gtk::Stack,
    done: gtk::Image,
    name_entry: gtk::Entry,
    publish_switch: gtk::Switch,
    track_set_list: List,
    track_sets: RefCell<Vec<TrackSetData>>,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl MediumEditor {
    /// Create a new medium editor.
    pub fn new(backend: Rc<Backend>, source: DiscSource) -> Rc<Self> {
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

        let list = List::new("No recordings added.");
        frame.add(&list.widget);

        let this = Rc::new(Self {
            backend,
            source: Rc::new(source),
            widget,
            done_button,
            done_stack,
            done,
            name_entry,
            publish_switch,
            track_set_list: list,
            track_sets: RefCell::new(Vec::new()),
            navigator: RefCell::new(None),
        });

        // Connect signals and callbacks

        back_button.connect_clicked(clone!(@strong this => move |_| {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                navigator.pop();
            }
        }));

        this.done_button.connect_clicked(clone!(@strong this => move |_| {
            let context = glib::MainContext::default();
            let clone = this.clone();
            context.spawn_local(async move {
                clone.widget.set_visible_child_name("loading");
                match clone.clone().save().await {
                    Ok(_) => (),
                    Err(err) => {
                        println!("{:?}", err);
                        // clone.info_bar.set_revealed(true);
                    }
                }

            });
        }));

        add_button.connect_clicked(clone!(@strong this => move |_| {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                let editor = TrackSetEditor::new(this.backend.clone(), Rc::clone(&this.source));

                editor.set_done_cb(clone!(@strong this => move |track_set| {
                    let length = {
                        let mut track_sets = this.track_sets.borrow_mut();
                        track_sets.push(track_set);
                        track_sets.len()
                    };

                    this.track_set_list.update(length);
                }));

                navigator.push(editor);
            }
        }));

        this.track_set_list.set_make_widget(clone!(@strong this => move |index| {
            let track_set = &this.track_sets.borrow()[index];

            let title = track_set.recording.work.get_title();
            let subtitle = track_set.recording.get_performers();

            let edit_image = gtk::Image::from_icon_name(Some("document-edit-symbolic"), gtk::IconSize::Button);
            let edit_button = gtk::Button::new();
            edit_button.set_relief(gtk::ReliefStyle::None);
            edit_button.set_valign(gtk::Align::Center);
            edit_button.add(&edit_image);

            let row = libhandy::ActionRow::new();
            row.set_activatable(true);
            row.set_title(Some(&title));
            row.set_subtitle(Some(&subtitle));
            row.add(&edit_button);
            row.set_activatable_widget(Some(&edit_button));
            row.show_all();

            edit_button.connect_clicked(clone!(@strong this => move |_| {

            }));

            row.upcast()
        }));

        // Start ripping the CD in the background.
        let context = glib::MainContext::default();
        let clone = this.clone();
        context.spawn_local(async move {
            match clone.source.rip().await {
                Err(error) => {
                    // TODO: Present error.
                    println!("Failed to rip: {}", error);
                },
                Ok(_) => {
                    clone.done_stack.set_visible_child(&clone.done);
                    clone.done_button.set_sensitive(true);
                }
            }
        });

        this
    }

    /// Save the medium and possibly upload it to the server.
    async fn save(self: Rc<Self>) -> Result<()> {
        let name = self.name_entry.get_text().to_string();

        // Create a new directory in the music library path for the imported medium.

        let mut path = self.backend.get_music_library_path().unwrap().clone();
        path.push(&name);
        std::fs::create_dir(&path)?;

        // Convert the track set data to real track sets.

        let mut track_sets = Vec::new();

        for track_set_data in &*self.track_sets.borrow() {
            let mut tracks = Vec::new();

            for track_data in &track_set_data.tracks {
                // Copy the corresponding audio file to the music library.

                let track_source = &self.source.tracks[track_data.track_source];
                let file_name = format!("track_{:02}.flac", track_source.number);

                let mut track_path = path.clone();
                track_path.push(&file_name);

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
            name: self.name_entry.get_text().to_string(),
            discid: Some(self.source.discid.clone()),
            tracks: track_sets,
        };

        let upload = self.publish_switch.get_active();
        if upload {
            // self.backend.post_medium(&medium).await?;
        }

        self.backend
            .db()
            .update_medium(medium.clone())
            .await?;

        self.backend.library_changed();

        let navigator = self.navigator.borrow().clone();
        if let Some(navigator) = navigator {
            navigator.clone().pop();
        }

        Ok(())
    }
}

impl NavigatorScreen for MediumEditor {
    fn attach_navigator(&self, navigator: Rc<Navigator>) {
        self.navigator.replace(Some(navigator));
    }

    fn get_widget(&self) -> gtk::Widget {
        self.widget.clone().upcast()
    }

    fn detach_navigator(&self) {
        self.navigator.replace(None);
    }
}
