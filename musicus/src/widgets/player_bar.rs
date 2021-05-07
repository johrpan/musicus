use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use musicus_backend::Player;
use std::cell::RefCell;
use std::rc::Rc;

pub struct PlayerBar {
    pub widget: gtk::Revealer,
    title_label: gtk::Label,
    subtitle_label: gtk::Label,
    previous_button: gtk::Button,
    play_button: gtk::Button,
    next_button: gtk::Button,
    position_label: gtk::Label,
    duration_label: gtk::Label,
    play_image: gtk::Image,
    pause_image: gtk::Image,
    player: Rc<RefCell<Option<Rc<Player>>>>,
    playlist_cb: Rc<RefCell<Option<Box<dyn Fn()>>>>,
}

impl PlayerBar {
    pub fn new() -> Self {
        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/player_bar.ui");

        get_widget!(builder, gtk::Revealer, widget);
        get_widget!(builder, gtk::Label, title_label);
        get_widget!(builder, gtk::Label, subtitle_label);
        get_widget!(builder, gtk::Button, previous_button);
        get_widget!(builder, gtk::Button, play_button);
        get_widget!(builder, gtk::Button, next_button);
        get_widget!(builder, gtk::Label, position_label);
        get_widget!(builder, gtk::Label, duration_label);
        get_widget!(builder, gtk::Button, playlist_button);
        get_widget!(builder, gtk::Image, play_image);
        get_widget!(builder, gtk::Image, pause_image);

        let player = Rc::new(RefCell::new(None::<Rc<Player>>));
        let playlist_cb = Rc::new(RefCell::new(None::<Box<dyn Fn()>>));

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

        playlist_button.connect_clicked(clone!(@strong playlist_cb => move |_| {
            if let Some(cb) = &*playlist_cb.borrow() {
                cb();
            }
        }));

        Self {
            widget,
            title_label,
            subtitle_label,
            previous_button,
            play_button,
            next_button,
            position_label,
            duration_label,
            play_image,
            pause_image,
            player,
            playlist_cb,
        }
    }

    pub fn set_player(&self, player: Option<Rc<Player>>) {
        self.player.replace(player.clone());

        if let Some(player) = player {
            let playlist = Rc::new(RefCell::new(Vec::new()));

            player.add_playlist_cb(clone!(
                @strong player,
                @strong self.widget as widget,
                @strong self.previous_button as previous_button,
                @strong self.next_button as next_button,
                @strong playlist
                => move |new_playlist| {
                    widget.set_reveal_child(!new_playlist.is_empty());
                    playlist.replace(new_playlist);
                    previous_button.set_sensitive(player.has_previous());
                    next_button.set_sensitive(player.has_next());
                }
            ));

            player.add_track_cb(clone!(
                @strong player,
                @strong playlist,
                @strong self.previous_button as previous_button,
                @strong self.next_button as next_button,
                @strong self.title_label as title_label,
                @strong self.subtitle_label as subtitle_label,
                @strong self.position_label as position_label
                => move |current_track| {
                    previous_button.set_sensitive(player.has_previous());
                    next_button.set_sensitive(player.has_next());

                    let track = &playlist.borrow()[current_track];

                    let mut parts = Vec::<String>::new();
                    for part in &track.work_parts {
                        parts.push(track.recording.work.parts[*part].title.clone());
                    }

                    let mut title = track.recording.work.get_title();
                    if !parts.is_empty() {
                        title = format!("{}: {}", title, parts.join(", "));
                    }

                    title_label.set_text(&title);
                    subtitle_label.set_text(&track.recording.get_performers());
                    position_label.set_text("0:00");
                }
            ));

            player.add_duration_cb(clone!(
                @strong self.duration_label as duration_label
                => move |ms| {
                    let min = ms / 60000;
                    let sec = (ms % 60000) / 1000;
                    duration_label.set_text(&format!("{}:{:02}", min, sec));
                }
            ));

            player.add_playing_cb(clone!(
                @strong self.play_button as play_button,
                @strong self.play_image as play_image,
                @strong self.pause_image as pause_image
                => move |playing| {
                    play_button.set_child(Some(if playing {
                        &pause_image
                    } else {
                        &play_image
                    }));
                }
            ));

            player.add_position_cb(clone!(
                @strong self.position_label as position_label
                => move |ms| {
                    let min = ms / 60000;
                    let sec = (ms % 60000) / 1000;
                    position_label.set_text(&format!("{}:{:02}", min, sec));
                }
            ));
        } else {
            self.widget.set_reveal_child(false);
        }
    }

    pub fn set_playlist_cb<F: Fn() + 'static>(&self, cb: F) {
        self.playlist_cb.replace(Some(Box::new(cb)));
    }
}
