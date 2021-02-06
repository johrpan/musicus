use crate::navigator::{NavigationHandle, Screen};
use crate::widgets;
use crate::widgets::{List, Section, Widget};
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use libadwaita::prelude::*;
use musicus_backend::PlaylistItem;
use musicus_backend::db::Medium;
use std::rc::Rc;

/// Elements for visually representing the contents of the medium.
enum ListItem {
    /// A header shown on top of a track set. The value is the index of the corresponding track set
    /// within the medium.
    Header(usize),

    /// A track. The indices are from the track set and the track.
    Track(usize, usize),

    /// A separator shown between track sets.
    Separator,
}

/// A screen for showing the contents of a medium.
pub struct MediumScreen {
    handle: NavigationHandle<()>,
    medium: Medium,
    widget: widgets::Screen,
    list: Rc<List>,
    items: Vec<ListItem>,
}

impl Screen<Medium, ()> for MediumScreen {
    /// Create a new medium screen for the specified medium and load the
    /// contents asynchronously.
    fn new(medium: Medium, handle: NavigationHandle<()>) -> Rc<Self> {
        let mut items = Vec::new();
        let mut first = true;

        for (track_set_index, track_set) in medium.tracks.iter().enumerate() {
            if !first {
                items.push(ListItem::Separator);
            } else {
                first = false;
            }

            items.push(ListItem::Header(track_set_index));

            for (track_index, _) in track_set.tracks.iter().enumerate() {
                items.push(ListItem::Track(track_set_index, track_index));
            }
        }

        let widget = widgets::Screen::new();
        widget.set_title(&medium.name);

        let list = List::new();
        let section = Section::new("Recordings", &list.widget);
        widget.add_content(&section.widget);
        widget.ready();

        let this = Rc::new(Self {
            handle,
            medium,
            widget,
            list,
            items,
        });

        this.widget.set_back_cb(clone!(@weak this => move || {
            this.handle.pop(None);
        }));


        this.widget.add_action(&gettext("Edit medium"), clone!(@weak this => move || {
            // TODO: Show medium editor.
        }));

        this.widget.add_action(&gettext("Delete medium"), clone!(@weak this => move || {
            // TODO: Delete medium and maybe also the tracks?
        }));

        section.add_action("media-playback-start-symbolic", clone!(@weak this => move || {
            for track_set in &this.medium.tracks {
                let indices = (0..track_set.tracks.len()).collect();

                let playlist_item = PlaylistItem {
                    track_set: track_set.clone(),
                    indices,
                };

                this.handle.backend.pl().add_item(playlist_item).unwrap();
            }
        }));

        this.list.set_make_widget_cb(clone!(@weak this => move |index| {
            match this.items[index] {
                ListItem::Header(index) => {
                    let track_set = &this.medium.tracks[index];
                    let recording = &track_set.recording;

                    let row = libadwaita::ActionRow::new();
                    row.set_activatable(false);
                    row.set_selectable(false);
                    row.set_title(Some(&recording.work.get_title()));
                    row.set_subtitle(Some(&recording.get_performers()));

                    row.upcast()
                }
                ListItem::Track(track_set_index, track_index) => {
                    let track_set = &this.medium.tracks[track_set_index];
                    let track = &track_set.tracks[track_index];

                    let mut parts = Vec::<String>::new();
                    for part in &track.work_parts {
                        parts.push(track_set.recording.work.parts[*part].title.clone());
                    }

                    let title = if parts.is_empty() {
                        gettext("Unknown")
                    } else {
                        parts.join(", ")
                    };

                    let row = libadwaita::ActionRow::new();
                    row.set_selectable(false);
                    row.set_activatable(false);
                    row.set_title(Some(&title));
                    row.set_margin_start(12);

                    row.upcast()
                }
                ListItem::Separator => {
                    let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
                    separator.upcast()
                }
            }
        }));

        this.list.update(this.items.len());

        this
    }
}

impl Widget for MediumScreen {
    fn get_widget(&self) -> gtk::Widget {
        self.widget.widget.clone().upcast()
    }
}
