use crate::backend::Backend;
use crate::database::{Recording, Track, TrackSet};
use crate::selectors::{PersonSelector, RecordingSelector, WorkSelector};
use crate::widgets::{Navigator, NavigatorScreen};
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use libhandy::prelude::*;
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::rc::Rc;

/// Representation of a track that can be imported into the music library.
#[derive(Debug, Clone)]
struct TrackSource {
    /// A short string identifying the track for the user.
    pub description: String,

    /// Whether the track is ready to be imported.
    pub ready: bool,
}

/// Representation of a medium that can be imported into the music library.
#[derive(Debug, Clone)]
struct MediumSource {
    /// The tracks that can be imported from the medium.
    pub tracks: Vec<TrackSource>,

    /// Whether all tracks are ready to be imported.
    pub ready: bool,
}

impl MediumSource {
    /// Create a dummy medium source for testing purposes.
    fn dummy() -> Self {
        let mut tracks = Vec::new();

        for index in 0..20 {
            tracks.push(TrackSource {
                description: format!("Track {}", index + 1),
                ready: Cell::new(true),
            });
        }

        Self {
            tracks,
            ready: Cell::new(true),
        }
    }
}

/// A track while being edited.
#[derive(Debug, Clone)]
struct TrackData<'a> {
    /// A reference to the selected track source.
    pub source: &'a TrackSource,

    /// The actual value for the track.
    pub track: Track,
}

/// A track set while being edited.
#[derive(Debug, Clone)]
struct TrackSetData<'a> {
    /// The recording to which the tracks belong.
    pub recording: Option<Recording>,

    /// The tracks that are being edited.
    pub tracks: Vec<TrackData<'a>>,
}

impl TrackSetData {
    /// Create a new empty track set.
    pub fn new() -> Self {
        Self {
            recording: None,
            tracks: Vec::new(),
        }
    }
}

/// A screen for editing a set of tracks for one recording.
pub struct TrackSetEditor {
    backend: Rc<Backend>,
    source: Rc<RefCell<MediumSource>>,
    widget: gtk::Box,
    save_button: gtk::Button,
    recording_row: libhandy::ActionRow,
    track_list: List,
    data: RefCell<TrackSetData>,
    done_cb: RefCell<Option<Box<dyn Fn(TrackSet)>>>,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl TrackSetEditor {
    /// Create a new track set editor.
    pub fn new(backend: Rc<Backend>, source: Rc<TrackSource>) -> Rc<Self> {
        // TODO: Replace with argument.
        let source = Rc::new(RefCell::new(MediumSource::dummy()));

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
            data: RefCell::new(TrackSetData::new()),
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
            if let Some(cb) = &*this.done_cb.borrow() {}
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
                            let mut data = this.data.borrow_mut();
                            data.recording = Some(recording);
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
                let selector = TrackSelector::new(Rc::clone(this.source));

                selector.set_selected_cb(clone!(@strong this => move |selection| {
                    let mut tracks = Vec::new();

                    for index in selection {
                        let track = Track {
                            work_parts: Vec::new(),
                        };

                        let source = this.source.tracks[index].clone();

                        let data = TrackData {
                            track,
                            source,
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
            let data = &this.tracks.borrow()[index];

            let mut title_parts = Vec::<String>::new();

            if let Some(recording) = &*this.recording.borrow() {
                for part in &data.track.work_parts {
                    title_parts.push(recording.work.parts[*part].title.clone());
                }
            }

            let title = if title_parts.is_empty() {
                gettext("Unknown")
            } else {
                title_parts.join(", ")
            };

            let subtitle = data.source.description.clone();

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
                    let editor = TrackEditor::new(recording, Vec::new());

                    editor.set_selected_cb(clone!(@strong this => move |selection| {
                        {
                            let mut tracks = &mut this.data.borrow_mut().tracks;
                            let mut track = &mut tracks[index];
                            track.track.work_parts = selection;
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
    pub fn set_done_cb<F: Fn(TrackSet) + 'static>(&self, cb: F) {
        self.done_cb.replace(Some(Box::new(cb)));
    }

    /// Set everything up after selecting a recording.
    fn recording_selected(&self) {
        if let Some(recording) = self.data.borrow().recording {
            self.recording_row.set_title(Some(&recording.work.get_title()));
            self.recording_row.set_subtitle(Some(&recording.get_performers()));
            self.save_button.set_sensitive(true);
        }

        self.autofill_parts();
    }

    /// Automatically try to put work part information from the selected recording into the
    /// selected tracks.
    fn autofill_parts(&self) {
        if let Some(recording) = self.data.borrow().recording {
            let mut tracks = self.tracks.borrow_mut();

            for (index, _) in recording.work.parts.iter().enumerate() {
                if let Some(mut data) = tracks.get_mut(index) {
                    data.track.work_parts = vec![index];
                } else {
                    break;
                }
            }
        }

        self.update_tracks();
    }

    /// Update the track list.
    fn update_tracks(&self) {
        let length = self.data.borrow().tracks.len();
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


/// A screen for selecting tracks from a medium.
struct TrackSelector {
    source: Rc<RefCell<MediumSource>>,
    widget: gtk::Box,
    select_button: gtk::Button,
    selection: RefCell<Vec<usize>>,
    selected_cb: RefCell<Option<Box<dyn Fn(Vec<usize>)>>>,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl TrackSelector {
    /// Create a new track selector.
    pub fn new(source: Rc<RefCell<MediumSource>>) -> Rc<Self> {
        // Create UI

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/track_selector.ui");

        get_widget!(builder, gtk::Box, widget);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::Button, select_button);
        get_widget!(builder, gtk::Frame, tracks_frame);

        let track_list = gtk::ListBox::new();
        track_list.set_selection_mode(gtk::SelectionMode::None);
        track_list.set_vexpand(false);
        track_list.show();
        tracks_frame.add(&track_list);

        let this = Rc::new(Self {
            source,
            widget,
            select_button,
            selection: RefCell::new(Vec::new()),
            selected_cb: RefCell::new(None),
            navigator: RefCell::new(None),
        });

        // Connect signals and callbacks

        back_button.connect_clicked(clone!(@strong this => move |_| {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                navigator.pop();
            }
        }));

        this.select_button.connect_clicked(clone!(@strong this => move |_| {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                navigator.pop();
            }

            if let Some(cb) = &*this.selected_cb.borrow() {
                let selection = this.selection.borrow().clone();
                cb(selection);
            }
        }));

        for (index, track) in this.tracks.iter().enumerate() {
            let check = gtk::CheckButton::new();

            check.connect_toggled(clone!(@strong this => move |check| {
                let mut selection = this.selection.borrow_mut();
                if check.get_active() {
                    selection.push(index);
                } else {
                    if let Some(pos) = selection.iter().position(|part| *part == index) {
                        selection.remove(pos);
                    }
                }

                if selection.is_empty() {
                    this.select_button.set_sensitive(false);
                } else {
                    this.select_button.set_sensitive(true);
                }
            }));

            let row = libhandy::ActionRow::new();
            row.add_prefix(&check);
            row.set_activatable_widget(Some(&check));
            row.set_title(Some(&track.description));
            row.show_all();

            track_list.add(&row);
        }

        this
    }

    /// Set the closure to be called when the user has selected tracks. The
    /// closure will be called with the indices of the selected tracks as its
    /// argument.
    pub fn set_selected_cb<F: Fn(Vec<usize>) + 'static>(&self, cb: F) {
        self.selected_cb.replace(Some(Box::new(cb)));
    }
}

impl NavigatorScreen for TrackSelector {
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

/// A screen for editing a single track.
struct TrackEditor {
    widget: gtk::Box,
    selection: RefCell<Vec<usize>>,
    selected_cb: RefCell<Option<Box<dyn Fn(Vec<usize>)>>>,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl TrackEditor {
    /// Create a new track editor.
    pub fn new(recording: Recording, selection: Vec<usize>) -> Rc<Self> {
        // Create UI

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/track_editor.ui");

        get_widget!(builder, gtk::Box, widget);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::Button, select_button);
        get_widget!(builder, gtk::Frame, parts_frame);

        let parts_list = gtk::ListBox::new();
        parts_list.set_selection_mode(gtk::SelectionMode::None);
        parts_list.set_vexpand(false);
        parts_list.show();
        parts_frame.add(&parts_list);

        let this = Rc::new(Self {
            widget,
            selection: RefCell::new(selection),
            selected_cb: RefCell::new(None),
            navigator: RefCell::new(None),
        });

        // Connect signals and callbacks

        back_button.connect_clicked(clone!(@strong this => move |_| {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                navigator.pop();
            }
        }));

        select_button.connect_clicked(clone!(@strong this => move |_| {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                navigator.pop();
            }

            if let Some(cb) = &*this.selected_cb.borrow() {
                let selection = this.selection.borrow().clone();
                cb(selection);
            }
        }));

        for (index, part) in recording.work.parts.iter().enumerate() {
            let check = gtk::CheckButton::new();

            check.connect_toggled(clone!(@strong this => move |check| {
                let mut selection = this.selection.borrow_mut();
                if check.get_active() {
                    selection.push(index);
                } else {
                    if let Some(pos) = selection.iter().position(|part| *part == index) {
                        selection.remove(pos);
                    }
                }
            }));

            let row = libhandy::ActionRow::new();
            row.add_prefix(&check);
            row.set_activatable_widget(Some(&check));
            row.set_title(Some(&part.title));
            row.show_all();

            parts_list.add(&row);
        }

        this
    }

    /// Set the closure to be called when the user has edited the track.
    pub fn set_selected_cb<F: Fn(Vec<usize>) + 'static>(&self, cb: F) {
        self.selected_cb.replace(Some(Box::new(cb)));
    }
}

impl NavigatorScreen for TrackEditor {
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

/// A simple list of widgets.
struct List {
    pub widget: gtk::ListBox,
    make_widget: RefCell<Option<Box<dyn Fn(usize) -> gtk::Widget>>>,
}

impl List {
    /// Create a new list. The list will be empty.
    pub fn new(placeholder_text: &str) -> Self {
        let placeholder_label = gtk::Label::new(Some(placeholder_text));
        placeholder_label.set_margin_top(6);
        placeholder_label.set_margin_bottom(6);
        placeholder_label.set_margin_start(6);
        placeholder_label.set_margin_end(6);
        placeholder_label.show();

        let widget = gtk::ListBox::new();
        widget.set_selection_mode(gtk::SelectionMode::None);
        widget.set_placeholder(Some(&placeholder_label));
        widget.show();

        Self {
            widget,
            make_widget: RefCell::new(None),
        }
    }

    /// Set the closure to be called to construct widgets for the items.
    pub fn set_make_widget<F: Fn(usize) -> gtk::Widget + 'static>(&self, make_widget: F) {
        self.make_widget.replace(Some(Box::new(make_widget)));
    }

    /// Call the make_widget function for each item. This will automatically
    /// show all children by indices 0..length.
    pub fn update(&self, length: usize) {
        for child in self.widget.get_children() {
            self.widget.remove(&child);
        }

        if let Some(make_widget) = &*self.make_widget.borrow() {
            for index in 0..length {
                let row = make_widget(index);
                self.widget.insert(&row, -1);
            }
        }
    }
}
