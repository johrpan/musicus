use crate::navigator::{NavigationHandle, Screen};
use crate::widgets::{List, Widget};
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use libadwaita::prelude::*;
use musicus_backend::PlaylistItem;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Elements for visually representing the playlist.
enum ListItem {
    /// A header shown on top of a track set. This contains an index
    /// referencing the playlist item containing this track set.
    Header(usize),

    /// A playable track. This contains an index to the playlist item, an
    /// index to the track and whether it is the currently played one.
    Track(usize, usize, bool),

    /// A separator shown between track sets.
    Separator,
}

pub struct PlayerScreen {
    handle: NavigationHandle<()>,
    widget: gtk::Box,
    title_label: gtk::Label,
    subtitle_label: gtk::Label,
    previous_button: gtk::Button,
    play_button: gtk::Button,
    next_button: gtk::Button,
    position_label: gtk::Label,
    position: gtk::Adjustment,
    duration_label: gtk::Label,
    play_image: gtk::Image,
    pause_image: gtk::Image,
    list: Rc<List>,
    playlist: RefCell<Vec<PlaylistItem>>,
    items: RefCell<Vec<ListItem>>,
    seeking: Cell<bool>,
    current_item: Cell<usize>,
    current_track: Cell<usize>,
}

impl Screen<(), ()> for PlayerScreen {
    fn new(_: (), handle: NavigationHandle<()>) -> Rc<Self> {
        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/player_screen.ui");

        get_widget!(builder, gtk::Box, widget);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::Label, title_label);
        get_widget!(builder, gtk::Label, subtitle_label);
        get_widget!(builder, gtk::Button, previous_button);
        get_widget!(builder, gtk::Button, play_button);
        get_widget!(builder, gtk::Button, next_button);
        get_widget!(builder, gtk::Button, stop_button);
        get_widget!(builder, gtk::Label, position_label);
        get_widget!(builder, gtk::Scale, position_scale);
        get_widget!(builder, gtk::Adjustment, position);
        get_widget!(builder, gtk::Label, duration_label);
        get_widget!(builder, gtk::Image, play_image);
        get_widget!(builder, gtk::Image, pause_image);
        get_widget!(builder, gtk::Frame, frame);

        let list = List::new();
        frame.set_child(Some(&list.widget));

        let event_controller = gtk::EventControllerLegacy::new();
        position_scale.add_controller(&event_controller);

        let this = Rc::new(Self {
            handle,
            widget,
            title_label,
            subtitle_label,
            previous_button,
            play_button,
            next_button,
            position_label,
            position,
            duration_label,
            play_image,
            pause_image,
            list,
            items: RefCell::new(Vec::new()),
            playlist: RefCell::new(Vec::new()),
            seeking: Cell::new(false),
            current_item: Cell::new(0),
            current_track: Cell::new(0),
        });

        let player = &this.handle.backend.pl();

        player.add_playlist_cb(clone!(@weak this => @default-return (), move |playlist| {
            if playlist.is_empty() {
                this.handle.pop(None);
            }

            this.playlist.replace(playlist);
            this.show_playlist();
        }));

        player.add_track_cb(clone!(@weak this, @weak player => @default-return (), move |current_item, current_track| {
            this.previous_button.set_sensitive(this.handle.backend.pl().has_previous());
            this.next_button.set_sensitive(this.handle.backend.pl().has_next());

            let item = &this.playlist.borrow()[current_item];
            let track = &item.track_set.tracks[current_track];

            let mut parts = Vec::<String>::new();
            for part in &track.work_parts {
                parts.push(item.track_set.recording.work.parts[*part].title.clone());
            }

            let mut title = item.track_set.recording.work.get_title();
            if !parts.is_empty() {
                title = format!("{}: {}", title, parts.join(", "));
            }

            this.title_label.set_text(&title);
            this.subtitle_label.set_text(&item.track_set.recording.get_performers());
            this.position_label.set_text("0:00");

            this.current_item.set(current_item);
            this.current_track.set(current_track);

            this.show_playlist();
        }));

        player.add_duration_cb(clone!(@weak this => @default-return (), move |ms| {
            let min = ms / 60000;
            let sec = (ms % 60000) / 1000;
            this.duration_label.set_text(&format!("{}:{:02}", min, sec));
            this.position.set_upper(ms as f64);
        }));

        player.add_playing_cb(clone!(@weak this => @default-return (), move |playing| {
            this.play_button.set_child(Some(if playing {
                &this.pause_image
            } else {
                &this.play_image
            }));
        }));

        player.add_position_cb(clone!(@weak this => @default-return (), move |ms| {
            if !this.seeking.get() {
                let min = ms / 60000;
                let sec = (ms % 60000) / 1000;
                this.position_label.set_text(&format!("{}:{:02}", min, sec));
                this.position.set_value(ms as f64);
            }
        }));

        back_button.connect_clicked(clone!(@weak this => move |_| {
            this.handle.pop(None);
        }));

        this.previous_button.connect_clicked(clone!(@weak this => move |_| {
            this.handle.backend.pl().previous().unwrap();
        }));

        this.play_button.connect_clicked(clone!(@weak this => move |_| {
            this.handle.backend.pl().play_pause();
        }));

        this.next_button.connect_clicked(clone!(@weak this => move |_| {
            this.handle.backend.pl().next().unwrap();
        }));

        stop_button.connect_clicked(clone!(@weak this => move |_| {
            this.handle.backend.pl().clear();
        }));

        event_controller.connect_event(clone!(@weak this => move |_, event| {
            if let Some(event) = event.downcast_ref::<gdk::ButtonEvent>() {
                if event.get_button() == gdk::BUTTON_PRIMARY {
                    match event.get_event_type() {
                        gdk::EventType::ButtonPress => {
                            this.seeking.replace(true);
                        }
                        gdk::EventType::ButtonRelease => {
                            this.handle.backend.pl().seek(this.position.get_value() as u64);
                            this.seeking.replace(false);
                        }
                        _ => (),
                    }
                }

            }

            glib::signal::Inhibit(false)
        }));

        position_scale.connect_value_changed(clone!(@weak this => move |_| {
            if this.seeking.get() {
                let ms = this.position.get_value() as u64;
                let min = ms / 60000;
                let sec = (ms % 60000) / 1000;

                this.position_label.set_text(&format!("{}:{:02}", min, sec));
            }
        }));

        this.list.set_make_widget_cb(clone!(@weak this => move |index| {
            let widget = match this.items.borrow()[index] {
                ListItem::Header(item_index) => {
                    let playlist_item = &this.playlist.borrow()[item_index];
                    let recording = &playlist_item.track_set.recording;

                    let row = libadwaita::ActionRow::new();
                    row.set_activatable(false);
                    row.set_selectable(false);
                    row.set_title(Some(&recording.work.get_title()));
                    row.set_subtitle(Some(&recording.get_performers()));

                    row.upcast()
                }
                ListItem::Track(item_index, track_index, playing) => {
                    let playlist_item = &this.playlist.borrow()[item_index];
                    let index = playlist_item.indices[track_index];
                    let track = &playlist_item.track_set.tracks[index];

                    let mut parts = Vec::<String>::new();
                    for part in &track.work_parts {
                        parts.push(playlist_item.track_set.recording.work.parts[*part].title.clone());
                    }

                    let title = if parts.is_empty() {
                        gettext("Unknown")
                    } else {
                        parts.join(", ")
                    };

                    let row = libadwaita::ActionRow::new();
                    row.set_selectable(false);
                    row.set_activatable(true);
                    row.set_title(Some(&title));

                    row.connect_activated(clone!(@weak this => move |_| {
                        this.handle.backend.pl().set_track(item_index, track_index).unwrap();
                    }));

                    let icon = if playing {
                        Some("media-playback-start-symbolic")
                    } else {
                        None
                    };

                    let image = gtk::Image::from_icon_name(icon);
                    row.add_prefix(&image);

                    row.upcast()
                }
                ListItem::Separator => {
                    let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
                    separator.upcast()
                }
            };

            widget
        }));

        player.send_data();

        this
    }
}

impl PlayerScreen {
    /// Update the user interface according to the playlist.
    fn show_playlist(&self) {
        let playlist = self.playlist.borrow();
        let current_item = self.current_item.get();
        let current_track = self.current_track.get();

        let mut first = true;
        let mut items = Vec::new();

        for (item_index, playlist_item) in playlist.iter().enumerate() {
            if !first {
                items.push(ListItem::Separator);
            } else {
                first = false;
            }

            items.push(ListItem::Header(item_index));

            for (index, _) in playlist_item.indices.iter().enumerate() {
                let playing = current_item == item_index && current_track == index;
                items.push(ListItem::Track(item_index, index, playing));
            }
        }

        let length = items.len();
        self.items.replace(items);
        self.list.update(length);
    }
}

impl Widget for PlayerScreen {
    fn get_widget(&self) -> gtk::Widget {
        self.widget.clone().upcast()
    }
}
