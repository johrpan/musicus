use crate::backend::*;
use crate::database::*;
use crate::editors::RecordingEditor;
use crate::player::*;
use crate::widgets::{List, Navigator, NavigatorScreen, NavigatorWindow};
use gettextrs::gettext;
use gio::prelude::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use libhandy::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// Representation of one entry within the track list.
enum ListItem {
    /// A track row. This hold an index to the track set and an index to the
    /// track within the track set.
    Track(usize, usize),

    /// A separator intended for use between track sets.
    Separator,
}

pub struct RecordingScreen {
    backend: Rc<Backend>,
    recording: Recording,
    widget: gtk::Box,
    stack: gtk::Stack,
    list: Rc<List>,
    track_sets: RefCell<Vec<TrackSet>>,
    items: RefCell<Vec<ListItem>>,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl RecordingScreen {
    pub fn new(backend: Rc<Backend>, recording: Recording) -> Rc<Self> {
        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/recording_screen.ui");

        get_widget!(builder, gtk::Box, widget);
        get_widget!(builder, gtk::Label, title_label);
        get_widget!(builder, gtk::Label, subtitle_label);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::Stack, stack);
        get_widget!(builder, gtk::Frame, frame);
        get_widget!(builder, gtk::Button, add_to_playlist_button);

        title_label.set_label(&recording.work.get_title());
        subtitle_label.set_label(&recording.get_performers());

        let edit_action = gio::SimpleAction::new("edit", None);
        let delete_action = gio::SimpleAction::new("delete", None);
        let edit_tracks_action = gio::SimpleAction::new("edit-tracks", None);
        let delete_tracks_action = gio::SimpleAction::new("delete-tracks", None);

        let actions = gio::SimpleActionGroup::new();
        actions.add_action(&edit_action);
        actions.add_action(&delete_action);
        actions.add_action(&edit_tracks_action);
        actions.add_action(&delete_tracks_action);

        widget.insert_action_group("widget", Some(&actions));

        let list = List::new();
        frame.set_child(Some(&list.widget));

        let this = Rc::new(Self {
            backend,
            recording,
            widget,
            stack,
            list,
            track_sets: RefCell::new(Vec::new()),
            items: RefCell::new(Vec::new()),
            navigator: RefCell::new(None),
        });

        this.list.set_make_widget_cb(clone!(@strong this => move |index| {
            match this.items.borrow()[index] {
                ListItem::Track(track_set_index, track_index) => {
                    let track_set = &this.track_sets.borrow()[track_set_index];
                    let track = &track_set.tracks[track_index];

                    let mut title_parts = Vec::<String>::new();
                    for part in &track.work_parts {
                        title_parts.push(this.recording.work.parts[*part].title.clone());
                    }

                    let title = if title_parts.is_empty() {
                        gettext("Unknown")
                    } else {
                        title_parts.join(", ")
                    };

                    let row = libhandy::ActionRow::new();
                    row.set_title(Some(&title));

                    row.upcast()
                }
                ListItem::Separator => {
                    let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
                    separator.upcast()
                }
            }
        }));

        back_button.connect_clicked(clone!(@strong this => move |_| {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                navigator.clone().pop();
            }
        }));

        // TODO: Decide whether to handle multiple track sets.
        add_to_playlist_button.connect_clicked(clone!(@strong this => move |_| {
            if let Some(player) = this.backend.get_player() {
                if let Some(track_set) = this.track_sets.borrow().get(0).cloned() {
                    let indices = (0..track_set.tracks.len()).collect();

                    let playlist_item = PlaylistItem {
                        track_set,
                        indices,
                    };

                    player.add_item(playlist_item).unwrap();
                }
            }
        }));

        edit_action.connect_activate(clone!(@strong this => move |_, _| {
            let editor = RecordingEditor::new(this.backend.clone(), Some(this.recording.clone()));
            let window = NavigatorWindow::new(editor);
            window.show();
        }));

        delete_action.connect_activate(clone!(@strong this => move |_, _| {
            let context = glib::MainContext::default();
            let clone = this.clone();
            context.spawn_local(async move {
                clone.backend.db().delete_recording(&clone.recording.id).await.unwrap();
                clone.backend.library_changed();
            });
        }));

        edit_tracks_action.connect_activate(clone!(@strong this => move |_, _| {
            // let editor = TracksEditor::new(this.backend.clone(), Some(this.recording.clone()), this.tracks.borrow().clone());
            // let window = NavigatorWindow::new(editor);
            // window.show();
        }));

        delete_tracks_action.connect_activate(clone!(@strong this => move |_, _| {
            let context = glib::MainContext::default();
            let clone = this.clone();
            context.spawn_local(async move {
                // clone.backend.db().delete_tracks(&clone.recording.id).await.unwrap();
                // clone.backend.library_changed();
            });
        }));

        let context = glib::MainContext::default();
        let clone = this.clone();
        context.spawn_local(async move {
            let track_sets = clone
                .backend
                .db()
                .get_track_sets(&clone.recording.id)
                .await
                .unwrap();

            clone.show_track_sets(track_sets);
            clone.stack.set_visible_child_name("content");
        });

        this
    }

    /// Update the track sets variable as well as the user interface.
    fn show_track_sets(&self, track_sets: Vec<TrackSet>) {
        let mut first = true;
        let mut items = Vec::new();

        for (track_set_index, track_set) in track_sets.iter().enumerate() {
            if !first {
                items.push(ListItem::Separator);
            } else {
                first = false;
            }

            for (track_index, _) in track_set.tracks.iter().enumerate() {
                items.push(ListItem::Track(track_set_index, track_index));
            }
        }

        let length = items.len();
        self.items.replace(items);
        self.track_sets.replace(track_sets);
        self.list.update(length);
    }
}

impl NavigatorScreen for RecordingScreen {
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
