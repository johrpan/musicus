use super::{List, Widget};
use gtk::prelude::*;
use glib::clone;
use itertools::Itertools;
use musicus_backend::db::{Recording, Track, Work};
use std::{cell::RefCell, rc::Rc};

/// A widget for displaying a list of tracks.
pub struct TrackList {
    tracks: RefCell<Vec<Track>>,
    list: Rc<List>,
}

impl TrackList {
    pub fn new() -> Rc<Self> {
        let list = List::new();

        let this = Rc::new(Self {
            tracks: RefCell::new(Vec::new()),
            list,
        });

        this.list.set_make_widget_cb(clone!(@weak this => move |index| {
            this.track_row(index)
        }));

        this
    }

    fn track_row(&self, index: usize) -> gtk::Widget {
        let tracks = self.tracks.borrow();
        let track = &tracks[index];

        if index > 0 {
            let previous_track = &tracks[index - 1];
            if previous_track.recording.id != track.recording.id {
                return TrackRow::new
            }
        }


    }
}

impl Widget for TrackList {
    fn get_widget(&self) -> gtk::Widget {
        self.list.get_widget()
    }
}

/// Create a new separator row.
fn separator() -> gtk::ListBoxRow {
    gtk::ListBoxRowBuilder::new()
        .selectable(false)
        .activatable(false)
        .child(&gtk::Separator::new(gtk::Orientation::Horizontal))
        .build()
}

/// Return an unfinished builder for a recording row.
fn recording_row(recording: &Recording) -> adw::ActionRowBuilder {
    adw::ActionRowBuilder::new()
        .title(&recording.work.get_title())
        .subtitle(&recording.get_performers())
}

/// Get a string representing the given list of work parts.
fn parts_string(work: &Work, part_indices: &[usize]) -> String {
    part_indices
        .iter()
        .map(|index| work.parts[*index].title.clone())
        .collect::<Vec<String>>()
        .join(", ")
}

/// A widget for displaying a single track within a list box.
struct TrackRow {
    pub widget: gtk::ListBoxRow,
    status_image: gtk::Image,
}

impl TrackRow {
    /// Create a new track row.
    ///
    /// Depending on the value of `header`, the row will display additional
    /// information on the recording.
    pub fn new(track: &Track, header: bool) -> Self {
        let widget = gtk::ListBoxRow::new();
        let content = gtk::Box::new(gtk::Orientation::Horizontal, 6);

        let status_image = gtk::Image::from_icon_name(None);
        content.append(&status_image);

        if header {
            let work_label = gtk::LabelBuilder::new()
                .label(&track.recording.work.get_title())
                .css_classes(vec![String::from("heading")])
                .build();

            let performers_label = gtk::LabelBuilder::new()
                .label(&track.recording.get_performers())
                .css_classes(vec![String::from("heading")])
                .margin_bottom(6)
                .build();

            let labels = gtk::Box::new(gtk::Orientation::Vertical, 0);
            labels.append(&work_label);
            labels.append(&performers_label);

            let title = track.title();
            if !title.is_empty() {
                let title_label = gtk::Label::new(Some(&track.title()));
                labels.append(&title_label);
            }

            content.append(&labels);
        } else {
            content.append(&title_label);
        }

        widget.set_child(Some(&content));

        Self {
            widget,
            status_image,
        }
    }

    pub fn set_playing(&self, playing: bool) {
        if playing {
            self.status_image
                .set_from_icon_name(Some("media-playback-start-symbolic"));
        } else {
            self.status_image.set_from_icon_name(None);
        }
    }
}

struct ListItem {
    playing: bool,
    header: Option<(String, String)>,
    title: Option<String>,
}
