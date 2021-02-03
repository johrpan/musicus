use crate::backend::Backend;
use crate::database::{Recording, TrackSet};
use crate::editors::RecordingEditor;
use crate::navigator::{NavigatorWindow, NavigationHandle, Screen};
use crate::player::PlaylistItem;
use crate::widgets;
use crate::widgets::{List, Section, Widget};
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use libadwaita::prelude::*;
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

/// A screen for showing a recording.
pub struct RecordingScreen {
    handle: NavigationHandle<()>,
    recording: Recording,
    widget: widgets::Screen,
    list: Rc<List>,
    track_sets: RefCell<Vec<TrackSet>>,
    items: RefCell<Vec<ListItem>>,
}

impl Screen<Recording, ()> for RecordingScreen {
    /// Create a new recording screen for the specified recording and load the
    /// contents asynchronously.
    fn new(recording: Recording, handle: NavigationHandle<()>) -> Rc<Self> {
        let widget = widgets::Screen::new();
        widget.set_title(&recording.work.get_title());
        widget.set_subtitle(&recording.get_performers());

        let list = List::new();
        let section = Section::new(&gettext("Tracks"), &list.widget);
        widget.add_content(&section.widget);

        let this = Rc::new(Self {
            handle,
            recording,
            widget,
            list,
            track_sets: RefCell::new(Vec::new()),
            items: RefCell::new(Vec::new()),
        });

        section.add_action("media-playback-start-symbolic", clone!(@weak this => move || {
            if let Some(player) = this.handle.backend.get_player() {
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

        this.widget.set_back_cb(clone!(@weak this => move || {
            this.handle.pop(None);
        }));

        this.widget.add_action(&gettext("Edit recording"), clone!(@weak this => move || {
            spawn!(@clone this, async move {
                let window = NavigatorWindow::new(this.handle.backend.clone());
                replace!(window.navigator, RecordingEditor, Some(this.recording.clone())).await;
            });
        }));

        this.widget.add_action(&gettext("Delete recording"), clone!(@weak this => move || {
            spawn!(@clone this, async move {
                this.handle.backend.db().delete_recording(&this.recording.id).await.unwrap();
                this.handle.backend.library_changed();
            });
        }));

        this.list.set_make_widget_cb(clone!(@weak this => move |index| {
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

                    let row = libadwaita::ActionRow::new();
                    row.set_title(Some(&title));

                    row.upcast()
                }
                ListItem::Separator => {
                    let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
                    separator.upcast()
                }
            }
        }));

        // Load the content asynchronously.

        spawn!(@clone this, async move {
            let track_sets = this.handle
                .backend
                .db()
                .get_track_sets(&this.recording.id)
                .await
                .unwrap();

            this.show_track_sets(track_sets);
            this.widget.ready();
        });

        this
    }
}

impl RecordingScreen {
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

impl Widget for RecordingScreen {
    fn get_widget(&self) -> gtk::Widget {
        self.widget.widget.clone().upcast()
    }
}
