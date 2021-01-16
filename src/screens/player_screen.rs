use crate::player::*;
use crate::widgets::*;
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct PlaylistElement {
    pub item: usize,
    pub track: usize,
    pub title: String,
    pub subtitle: Option<String>,
    pub playable: bool,
}

pub struct PlayerScreen {
    pub widget: gtk::Box,
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
    list: Rc<List<PlaylistElement>>,
    player: Rc<RefCell<Option<Rc<Player>>>>,
    seeking: Rc<Cell<bool>>,
    current_item: Rc<Cell<usize>>,
    current_track: Rc<Cell<usize>>,
    back_cb: Rc<RefCell<Option<Box<dyn Fn() -> ()>>>>,
}

impl PlayerScreen {
    pub fn new() -> Self {
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

        let back_cb = Rc::new(RefCell::new(None::<Box<dyn Fn() -> ()>>));

        back_button.connect_clicked(clone!(@strong back_cb => move |_| {
            if let Some(cb) = &*back_cb.borrow() {
                cb();
            }
        }));

        let player = Rc::new(RefCell::new(None::<Rc<Player>>));
        let seeking = Rc::new(Cell::new(false));

        previous_button.connect_clicked(clone!(@strong player => move |_| {
            if let Some(player) = &*player.borrow() {
                player.previous().unwrap();
            }
        }));

        play_button.connect_clicked(clone!(@strong player => move |_| {
            if let Some(player) = &*player.borrow() {
                player.play_pause();
            }
        }));

        next_button.connect_clicked(clone!(@strong player => move |_| {
            if let Some(player) = &*player.borrow() {
                player.next().unwrap();
            }
        }));

        stop_button.connect_clicked(clone!(@strong player, @strong back_cb => move |_| {
            if let Some(player) = &*player.borrow() {
                if let Some(cb) = &*back_cb.borrow() {
                    cb();
                }

                player.clear();
            }
        }));

        position_scale.connect_button_press_event(clone!(@strong seeking => move |_, _| {
            seeking.replace(true);
            Inhibit(false)
        }));

        position_scale.connect_button_release_event(
            clone!(@strong seeking, @strong position, @strong player => move |_, _| {
                if let Some(player) = &*player.borrow() {
                    player.seek(position.get_value() as u64);
                }

                seeking.replace(false);
                Inhibit(false)
            }),
        );

        position_scale.connect_value_changed(
            clone!(@strong seeking, @strong position, @strong position_label => move |_| {
                if seeking.get() {
                    let ms = position.get_value() as u64;
                    let min = ms / 60000;
                    let sec = (ms % 60000) / 1000;
                    position_label.set_text(&format!("{}:{:02}", min, sec));
                }
            }),
        );

        let current_item = Rc::new(Cell::<usize>::new(0));
        let current_track = Rc::new(Cell::<usize>::new(0));
        let list = List::new("");

        list.set_make_widget(clone!(
            @strong current_item,
            @strong current_track
            => move |element: &PlaylistElement| {
                let title_label = gtk::Label::new(Some(&element.title));
                title_label.set_ellipsize(pango::EllipsizeMode::End);
                title_label.set_halign(gtk::Align::Start);
                let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
                vbox.add(&title_label);
                if let Some(subtitle) = &element.subtitle {
                    let subtitle_label = gtk::Label::new(Some(&subtitle));
                    subtitle_label.set_ellipsize(pango::EllipsizeMode::End);
                    subtitle_label.set_halign(gtk::Align::Start);
                    subtitle_label.set_opacity(0.5);
                    vbox.add(&subtitle_label);
                }

                let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 6);
                hbox.set_border_width(6);

                if element.playable {
                    let image = gtk::Image::new();

                    if element.item == current_item.get() && element.track == current_track.get() {
                        image.set_from_icon_name(
                            Some("media-playback-start-symbolic"),
                            gtk::IconSize::Button,
                        );
                    }

                    hbox.add(&image);
                } else if element.item > 0 {
                    hbox.set_margin_top(18);
                }
                hbox.add(&vbox);
                hbox.upcast()
            }
        ));

        list.set_selected(clone!(@strong player => move |element| {
            if let Some(player) = &*player.borrow() {
                player.set_track(element.item, element.track).unwrap();
            }
        }));

        frame.add(&list.widget);

        Self {
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
            player,
            seeking,
            current_item,
            current_track,
            back_cb,
        }
    }

    pub fn set_player(&self, player: Option<Rc<Player>>) {
        self.player.replace(player.clone());

        if let Some(player) = player {
            let playlist = Rc::new(RefCell::new(Vec::<PlaylistItem>::new()));

            player.add_playlist_cb(clone!(
                @strong player,
                @strong self.previous_button as previous_button,
                @strong self.next_button as next_button,
                @strong self.list as list,
                @strong playlist
                => move |new_playlist| {
                    playlist.replace(new_playlist);
                    previous_button.set_sensitive(player.has_previous());
                    next_button.set_sensitive(player.has_next());

                    let mut elements = Vec::new();
                    for (item_index, item) in playlist.borrow().iter().enumerate() {
                        elements.push(PlaylistElement {
                            item: item_index,
                            track: 0,
                            title: item.track_set.recording.work.get_title(),
                            subtitle: Some(item.track_set.recording.get_performers()),
                            playable: false,
                        });

                        for track_index in &item.indices {
                            let track = &item.track_set.tracks[*track_index];

                            let mut parts = Vec::<String>::new();
                            for part in &track.work_parts {
                                parts.push(item.track_set.recording.work.parts[*part].title.clone());
                            }

                            let title = if parts.is_empty() {
                                gettext("Unknown")
                            } else {
                                parts.join(", ")
                            };

                            elements.push(PlaylistElement {
                                item: item_index,
                                track: *track_index,
                                title: title,
                                subtitle: None,
                                playable: true,
                            });
                        }
                    }

                    list.show_items(elements);
                }
            ));

            player.add_track_cb(clone!(
                @strong player,
                @strong playlist,
                @strong self.previous_button as previous_button,
                @strong self.next_button as next_button,
                @strong self.title_label as title_label,
                @strong self.subtitle_label as subtitle_label,
                @strong self.position_label as position_label,
                @strong self.current_item as self_item,
                @strong self.current_track as self_track,
                @strong self.list as list
                => move |current_item, current_track| {
                    previous_button.set_sensitive(player.has_previous());
                    next_button.set_sensitive(player.has_next());

                    let item = &playlist.borrow()[current_item];
                    let track = &item.track_set.tracks[current_track];

                    let mut parts = Vec::<String>::new();
                    for part in &track.work_parts {
                        parts.push(item.track_set.recording.work.parts[*part].title.clone());
                    }

                    let mut title = item.track_set.recording.work.get_title();
                    if !parts.is_empty() {
                        title = format!("{}: {}", title, parts.join(", "));
                    }

                    title_label.set_text(&title);
                    subtitle_label.set_text(&item.track_set.recording.get_performers());
                    position_label.set_text("0:00");

                    self_item.replace(current_item);
                    self_track.replace(current_track);
                    list.update();
                }
            ));

            player.add_duration_cb(clone!(
                @strong self.duration_label as duration_label,
                @strong self.position as position
                => move |ms| {
                    let min = ms / 60000;
                    let sec = (ms % 60000) / 1000;
                    duration_label.set_text(&format!("{}:{:02}", min, sec));
                    position.set_upper(ms as f64);
                }
            ));

            player.add_playing_cb(clone!(
                @strong self.play_button as play_button,
                @strong self.play_image as play_image,
                @strong self.pause_image as pause_image
                => move |playing| {
                    if let Some(child) = play_button.get_child() {
                        play_button.remove( &child);
                    }

                    play_button.add(if playing {
                        &pause_image
                    } else {
                        &play_image
                    });
                }
            ));

            player.add_position_cb(clone!(
                @strong self.position_label as position_label,
                @strong self.position as position,
                @strong self.seeking as seeking
                => move |ms| {
                    if !seeking.get() {
                        let min = ms / 60000;
                        let sec = (ms % 60000) / 1000;
                        position_label.set_text(&format!("{}:{:02}", min, sec));
                        position.set_value(ms as f64);
                    }
                }
            ));
        }
    }

    pub fn set_back_cb<F: Fn() -> () + 'static>(&self, cb: F) {
        self.back_cb.replace(Some(Box::new(cb)));
    }
}
