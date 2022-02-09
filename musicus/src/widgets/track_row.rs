use super::Widget;
use gtk::prelude::*;
use gtk_macros::get_widget;
use musicus_backend::db::Track;

/// A widget for showing a single track in a list.
pub struct TrackRow {
    /// The actual GTK widget.
    pub widget: gtk::ListBoxRow,
}

impl TrackRow {
    /// Create a new track row.
    pub fn new(track: &Track, show_header: bool, playing: bool) -> Self {
        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/track_row.ui");

        get_widget!(builder, gtk::ListBoxRow, widget);
        get_widget!(builder, gtk::Revealer, playing_revealer);
        get_widget!(builder, gtk::Image, playing_image);
        get_widget!(builder, gtk::Box, header_box);
        get_widget!(builder, gtk::Label, work_title_label);
        get_widget!(builder, gtk::Label, performances_label);
        get_widget!(builder, gtk::Label, track_title_label);

        playing_revealer.set_reveal_child(playing);

        let mut parts = Vec::<&str>::new();
        for part in &track.work_parts {
            parts.push(&track.recording.work.parts[*part].title);
        }

        if parts.is_empty() || show_header {
            work_title_label.set_text(&track.recording.work.get_title());
            performances_label.set_text(&track.recording.get_performers());
            header_box.show();
        } else {
            playing_image.set_margin_top(0);
        }

        if !parts.is_empty() {
            track_title_label.set_text(&parts.join(", "));
        } else {
            track_title_label.hide();
        }

        Self { widget }
    }
}

impl Widget for TrackRow {
    fn get_widget(&self) -> gtk::Widget {
        self.widget.clone().upcast()
    }
}
