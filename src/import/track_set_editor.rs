use super::source::Source;
use super::track_editor::TrackEditor;
use super::track_selector::TrackSelector;
use crate::backend::Backend;
use crate::database::Recording;
use crate::navigator::{NavigationHandle, Screen};
use crate::selectors::{PersonSelector, RecordingSelector};
use crate::widgets::{List, Widget};
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// A track set before being imported.
#[derive(Clone, Debug)]
pub struct TrackSetData {
    pub recording: Recording,
    pub tracks: Vec<TrackData>,
}

/// A track before being imported.
#[derive(Clone, Debug)]
pub struct TrackData {
    /// Index of the track source within the medium source's tracks.
    pub track_source: usize,

    /// Actual track data.
    pub work_parts: Vec<usize>,
}

/// A screen for editing a set of tracks for one recording.
pub struct TrackSetEditor {
    handle: NavigationHandle<TrackSetData>,
    source: Rc<Box<dyn Source>>,
    widget: gtk::Box,
    save_button: gtk::Button,
    recording_row: libadwaita::ActionRow,
    track_list: Rc<List>,
    recording: RefCell<Option<Recording>>,
    tracks: RefCell<Vec<TrackData>>,
}

impl Screen<Rc<Box<dyn Source>>, TrackSetData> for TrackSetEditor {
    /// Create a new track set editor.
    fn new(source: Rc<Box<dyn Source>>, handle: NavigationHandle<TrackSetData>) -> Rc<Self> {
        // Create UI

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/track_set_editor.ui");

        get_widget!(builder, gtk::Box, widget);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::Button, save_button);
        get_widget!(builder, libadwaita::ActionRow, recording_row);
        get_widget!(builder, gtk::Button, select_recording_button);
        get_widget!(builder, gtk::Button, edit_tracks_button);
        get_widget!(builder, gtk::Frame, tracks_frame);

        let track_list = List::new();
        tracks_frame.set_child(Some(&track_list.widget));

        let this = Rc::new(Self {
            handle,
            source,
            widget,
            save_button,
            recording_row,
            track_list,
            recording: RefCell::new(None),
            tracks: RefCell::new(Vec::new()),
        });

        // Connect signals and callbacks

        back_button.connect_clicked(clone!(@weak this => move |_| {
            this.handle.pop(None);
        }));

        this.save_button.connect_clicked(clone!(@weak this => move |_| {
            let data = TrackSetData {
                recording: this.recording.borrow().clone().unwrap(),
                tracks: this.tracks.borrow().clone(),
            };

            this.handle.pop(Some(data));
        }));

        select_recording_button.connect_clicked(clone!(@weak this => move |_| {
            spawn!(@clone this, async move {
                if let Some(recording) = push!(this.handle, RecordingSelector).await {
                    this.recording.replace(Some(recording));
                    this.recording_selected();
                }
            });
        }));

        edit_tracks_button.connect_clicked(clone!(@weak this => move |_| {
            spawn!(@clone this, async move {
                if let Some(selection) = push!(this.handle, TrackSelector, Rc::clone(&this.source)).await {
                    let mut tracks = Vec::new();

                    for index in selection {
                        let data = TrackData {
                            track_source: index,
                            work_parts: Vec::new(),
                        };

                        tracks.push(data);
                    }

                    let length = tracks.len();
                    this.tracks.replace(tracks);
                    this.track_list.update(length);
                    this.autofill_parts();
                }
            });
        }));

        this.track_list.set_make_widget_cb(clone!(@weak this => move |index| {
            let track = &this.tracks.borrow()[index];

            let mut title_parts = Vec::<String>::new();

            if let Some(recording) = &*this.recording.borrow() {
                for part in &track.work_parts {
                    title_parts.push(recording.work.parts[*part].title.clone());
                }
            }

            let title = if title_parts.is_empty() {
                gettext("Unknown")
            } else {
                title_parts.join(", ")
            };

            let tracks = this.source.tracks().unwrap();
            let track_name = &tracks[track.track_source].name;

            let edit_image = gtk::Image::from_icon_name(Some("document-edit-symbolic"));
            let edit_button = gtk::Button::new();
            edit_button.set_has_frame(false);
            edit_button.set_valign(gtk::Align::Center);
            edit_button.set_child(Some(&edit_image));

            let row = libadwaita::ActionRow::new();
            row.set_activatable(true);
            row.set_title(Some(&title));
            row.set_subtitle(Some(track_name));
            row.add_suffix(&edit_button);
            row.set_activatable_widget(Some(&edit_button));

            edit_button.connect_clicked(clone!(@weak this => move |_| {
                let recording = this.recording.borrow().clone();
                if let Some(recording) = recording {
                    spawn!(@clone this, async move {
                        let track = &this.tracks.borrow()[index];
                        if let Some(selection) = push!(this.handle, TrackEditor, (recording, track.work_parts.clone())).await {
                            {
                                let mut tracks = this.tracks.borrow_mut();
                                let mut track = &mut tracks[index];
                                track.work_parts = selection;
                            };

                            this.update_tracks();
                        }
                    });
                }
            }));

            row.upcast()
        }));

        this
    }
}

impl TrackSetEditor {
    /// Set everything up after selecting a recording.
    fn recording_selected(&self) {
        if let Some(recording) = &*self.recording.borrow() {
            self.recording_row.set_title(Some(&recording.work.get_title()));
            self.recording_row.set_subtitle(Some(&recording.get_performers()));
            self.save_button.set_sensitive(true);
        }

        self.autofill_parts();
    }

    /// Automatically try to put work part information from the selected recording into the
    /// selected tracks.
    fn autofill_parts(&self) {
        if let Some(recording) = &*self.recording.borrow() {
            let mut tracks = self.tracks.borrow_mut();

            for (index, _) in recording.work.parts.iter().enumerate() {
                if let Some(mut track) = tracks.get_mut(index) {
                    track.work_parts = vec![index];
                } else {
                    break;
                }
            }
        }

        self.update_tracks();
    }

    /// Update the track list.
    fn update_tracks(&self) {
        let length = self.tracks.borrow().len();
        self.track_list.update(length);
    }
}

impl Widget for TrackSetEditor {
    fn get_widget(&self) -> gtk::Widget {
        self.widget.clone().upcast()
    }
}

