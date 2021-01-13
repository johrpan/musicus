use super::disc_source::DiscSource;
use super::track_editor::TrackEditor;
use super::track_selector::TrackSelector;
use crate::backend::Backend;
use crate::database::{Recording, Track, TrackSet};
use crate::selectors::{PersonSelector, RecordingSelector, WorkSelector};
use crate::widgets::{Navigator, NavigatorScreen};
use crate::widgets::new_list::List;
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use libhandy::prelude::*;
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
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
    backend: Rc<Backend>,
    source: Rc<DiscSource>,
    widget: gtk::Box,
    save_button: gtk::Button,
    recording_row: libhandy::ActionRow,
    track_list: List,
    recording: RefCell<Option<Recording>>,
    tracks: RefCell<Vec<TrackData>>,
    done_cb: RefCell<Option<Box<dyn Fn(TrackSetData)>>>,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl TrackSetEditor {
    /// Create a new track set editor.
    pub fn new(backend: Rc<Backend>, source: Rc<DiscSource>) -> Rc<Self> {
        // Create UI

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/track_set_editor.ui");

        get_widget!(builder, gtk::Box, widget);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::Button, save_button);
        get_widget!(builder, libhandy::ActionRow, recording_row);
        get_widget!(builder, gtk::Button, select_recording_button);
        get_widget!(builder, gtk::Button, edit_tracks_button);
        get_widget!(builder, gtk::Frame, tracks_frame);

        let track_list = List::new(&gettext!("No tracks added"));
        tracks_frame.add(&track_list.widget);

        let this = Rc::new(Self {
            backend,
            source,
            widget,
            save_button,
            recording_row,
            track_list,
            recording: RefCell::new(None),
            tracks: RefCell::new(Vec::new()),
            done_cb: RefCell::new(None),
            navigator: RefCell::new(None),
        });

        // Connect signals and callbacks

        back_button.connect_clicked(clone!(@strong this => move |_| {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                navigator.pop();
            }
        }));

        this.save_button.connect_clicked(clone!(@strong this => move |_| {
            if let Some(cb) = &*this.done_cb.borrow() {
                let data = TrackSetData {
                    recording: this.recording.borrow().clone().unwrap(),
                    tracks: this.tracks.borrow().clone(),
                };

                cb(data);
            }

            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                navigator.pop();
            }
        }));

        select_recording_button.connect_clicked(clone!(@strong this => move |_| {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                let person_selector = PersonSelector::new(this.backend.clone());

                person_selector.set_selected_cb(clone!(@strong this, @strong navigator => move |person| {
                    let work_selector = WorkSelector::new(this.backend.clone(), person.clone());

                    work_selector.set_selected_cb(clone!(@strong this, @strong navigator => move |work| {
                        let recording_selector = RecordingSelector::new(this.backend.clone(), work.clone());

                        recording_selector.set_selected_cb(clone!(@strong this, @strong navigator => move |recording| {
                            this.recording.replace(Some(recording.clone()));
                            this.recording_selected();

                            navigator.clone().pop();
                            navigator.clone().pop();
                            navigator.clone().pop();
                        }));

                        navigator.clone().push(recording_selector);
                    }));

                    navigator.clone().push(work_selector);
                }));

                navigator.clone().push(person_selector);
            }
        }));

        edit_tracks_button.connect_clicked(clone!(@strong this => move |_| {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                let selector = TrackSelector::new(Rc::clone(&this.source));

                selector.set_selected_cb(clone!(@strong this => move |selection| {
                    let mut tracks = Vec::new();

                    for index in selection {
                        let track = Track {
                            work_parts: Vec::new(),
                        };

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
                }));

                navigator.push(selector);
            }
        }));

        this.track_list.set_make_widget(clone!(@strong this => move |index| {
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

            let number = this.source.tracks[track.track_source].number;
            let subtitle = format!("Track {}", number);

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
                let recording = this.recording.borrow().clone();
                let navigator = this.navigator.borrow().clone();

                if let (Some(recording), Some(navigator)) = (recording, navigator) {
                    let track = &this.tracks.borrow()[index];

                    let editor = TrackEditor::new(recording, track.work_parts.clone());

                    editor.set_selected_cb(clone!(@strong this => move |selection| {
                        {
                            let mut tracks = this.tracks.borrow_mut();
                            let mut track = &mut tracks[index];
                            track.work_parts = selection;
                        };

                        this.update_tracks();
                    }));

                    navigator.push(editor);
                }
            }));

            row.upcast()
        }));

        this
    }

    /// Set the closure to be called when the user has created the track set.
    pub fn set_done_cb<F: Fn(TrackSetData) + 'static>(&self, cb: F) {
        self.done_cb.replace(Some(Box::new(cb)));
    }

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

impl NavigatorScreen for TrackSetEditor {
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





