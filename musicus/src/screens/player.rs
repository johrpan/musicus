use crate::navigator::{NavigationHandle, Screen};
use crate::widgets::{List, TrackRow, Widget};
use adw::prelude::*;
use glib::clone;
use gtk_macros::get_widget;
use musicus_backend::db::Track;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Elements for visually representing the playlist.
#[derive(Clone)]
enum ListItem {
    /// A playable track.
    Track {
        /// Index within the playlist.
        index: usize,

        /// Whether this is the first track of the recording.
        first: bool,

        /// Whether this is the currently played track.
        playing: bool,
    },

    /// A separator shown between recordings.
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
    playlist: RefCell<Vec<Track>>,
    items: RefCell<Vec<ListItem>>,
    seeking: Cell<bool>,
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
            current_track: Cell::new(0),
        });

        let player = &this.handle.backend.pl();

        player.add_playlist_cb(clone!(@weak this => move |playlist| {
            if playlist.is_empty() {
                this.handle.pop(None);
            }

            this.playlist.replace(playlist);
            this.show_playlist();
        }));

        player.add_track_cb(clone!(@weak this, @weak player => move |current_track| {
            this.previous_button.set_sensitive(this.handle.backend.pl().has_previous());
            this.next_button.set_sensitive(this.handle.backend.pl().has_next());

            let track = &this.playlist.borrow()[current_track];

            let mut parts = Vec::<String>::new();
            for part in &track.work_parts {
                parts.push(track.recording.work.parts[*part].title.clone());
            }

            let mut title = track.recording.work.get_title();
            if !parts.is_empty() {
                title = format!("{}: {}", title, parts.join(", "));
            }

            this.title_label.set_text(&title);
            this.subtitle_label.set_text(&track.recording.get_performers());
            this.position_label.set_text("0:00");

            this.current_track.set(current_track);

            this.show_playlist();
        }));

        player.add_duration_cb(clone!(@weak this => move |ms| {
            let min = ms / 60000;
            let sec = (ms % 60000) / 1000;
            this.duration_label.set_text(&format!("{}:{:02}", min, sec));
            this.position.set_upper(ms as f64);
        }));

        player.add_playing_cb(clone!(@weak this => move |playing| {
            this.play_button.set_child(Some(if playing {
                &this.pause_image
            } else {
                &this.play_image
            }));
        }));

        player.add_position_cb(clone!(@weak this => move |ms| {
            if !this.seeking.get() {
                let min = ms / 60000;
                let sec = (ms % 60000) / 1000;
                this.position_label.set_text(&format!("{}:{:02}", min, sec));
                this.position.set_value(ms as f64);
            }
        }));

        back_button.connect_clicked(clone!(@weak this =>  move |_| {
            this.handle.pop(None);
        }));

        this.previous_button
            .connect_clicked(clone!(@weak this =>  move |_| {
                this.handle.backend.pl().previous().unwrap();
            }));

        this.play_button
            .connect_clicked(clone!(@weak this =>  move |_| {
                this.handle.backend.pl().play_pause().unwrap();
            }));

        this.next_button
            .connect_clicked(clone!(@weak this =>  move |_| {
                this.handle.backend.pl().next().unwrap();
            }));

        stop_button.connect_clicked(clone!(@weak this =>  move |_| {
            this.handle.backend.pl().clear();
        }));

        event_controller.connect_event(
            clone!(@weak this => @default-return glib::signal::Inhibit(false), move |_, event| {
                if let Some(event) = event.downcast_ref::<gdk::ButtonEvent>() {
                    if event.button() == gdk::BUTTON_PRIMARY {
                        match event.event_type() {
                            gdk::EventType::ButtonPress => {
                                this.seeking.replace(true);
                            }
                            gdk::EventType::ButtonRelease => {
                                this.handle.backend.pl().seek(this.position.value() as u64);
                                this.seeking.replace(false);
                            }
                            _ => (),
                        }
                    }

                }

                glib::signal::Inhibit(false)
            }),
        );

        position_scale.connect_value_changed(clone!(@weak this =>  move |_| {
            if this.seeking.get() {
                let ms = this.position.value() as u64;
                let min = ms / 60000;
                let sec = (ms % 60000) / 1000;

                this.position_label.set_text(&format!("{}:{:02}", min, sec));
            }
        }));

        this.list
            .set_make_widget_cb(clone!(@weak this => @default-panic, move |index| {
                let widget = match this.items.borrow()[index] {
                    ListItem::Track {index, first, playing} => {
                        let track = &this.playlist.borrow()[index];
                        TrackRow::new(track, first, playing).get_widget()
                    }
                    ListItem::Separator => {
                        gtk::ListBoxRowBuilder::new()
                            .selectable(false)
                            .activatable(false)
                            .child(&gtk::Separator::new(gtk::Orientation::Horizontal))
                            .build()
                            .upcast()
                    }
                };

                widget
            }));

        this.list
            .widget
            .connect_row_activated(clone!(@weak this => move |_, row| {
                let list_index = row.index();
                let list_item = this.items.borrow()[list_index as usize].clone();
                if let ListItem::Track {index, ..} = list_item {
                    this.handle.backend.pl().set_track(index).unwrap();
                };
            }));

        player.send_data();

        this
    }
}

impl PlayerScreen {
    /// Update the user interface according to the playlist.
    fn show_playlist(&self) {
        let playlist = self.playlist.borrow();
        let current_track = self.current_track.get();

        let mut first = true;
        let mut items = Vec::new();

        let mut last_recording_id = "";

        for (index, track) in playlist.iter().enumerate() {
            let first_track = if track.recording.id != last_recording_id {
                last_recording_id = &track.recording.id;

                if !first {
                    items.push(ListItem::Separator);
                } else {
                    first = false;
                }

                true
            } else {
                false
            };

            let item = ListItem::Track {
                index,
                first: first_track,
                playing: index == current_track,
            };

            items.push(item);
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
