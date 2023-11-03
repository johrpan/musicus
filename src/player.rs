use crate::playlist_item::PlaylistItem;
use fragile::Fragile;
use gstreamer_player::gst;
use gtk::{
    gio,
    glib::{self, clone, Properties},
    prelude::*,
    subclass::prelude::*,
};
use mpris_player::{MprisPlayer, PlaybackStatus};
use std::{
    cell::{Cell, OnceCell},
    sync::Arc,
};

mod imp {
    use mpris_player::Metadata;

    use super::*;

    #[derive(Properties, Debug, Default)]
    #[properties(wrapper_type = super::MusicusPlayer)]
    pub struct MusicusPlayer {
        #[property(get, set)]
        pub active: Cell<bool>,
        #[property(get, set)]
        pub playing: Cell<bool>,
        #[property(get, construct_only)]
        pub playlist: OnceCell<gio::ListStore>,
        #[property(get, set = Self::set_current_index)]
        pub current_index: Cell<u32>,
        #[property(get, set)]
        pub duration_ms: Cell<u64>,
        #[property(get, set)]
        pub current_time_ms: Cell<u64>,
        #[property(get, set = Self::set_position)]
        pub position: Cell<f64>,
        #[property(get, construct_only)]
        pub player: OnceCell<gstreamer_player::Player>,

        pub mpris: OnceCell<Arc<MprisPlayer>>,
    }

    impl MusicusPlayer {
        pub fn set_current_index(&self, index: u32) {
            let playlist = self.playlist.get().unwrap();

            if let Some(item) = playlist.item(index) {
                if let Some(old_item) = playlist.item(self.current_index.get()) {
                    old_item
                        .downcast::<PlaylistItem>()
                        .unwrap()
                        .set_is_playing(false);
                }

                let item = item.downcast::<PlaylistItem>().unwrap();
                self.mpris.get().unwrap().set_metadata(Metadata {
                    artist: Some(vec![item.make_title()]),
                    title: item.make_subtitle(),
                    ..Default::default()
                });

                let uri = glib::filename_to_uri(&item.path(), None)
                    .expect("track path should be parsable as an URI");

                let player = self.player.get().unwrap();
                player.set_uri(Some(&uri));
                if self.playing.get() {
                    player.play();
                }

                self.current_index.set(index);
                item.set_is_playing(true);
            }
        }

        pub fn set_position(&self, position: f64) {
            self.player
                .get()
                .unwrap()
                .seek(gst::ClockTime::from_mseconds(
                    (position * self.duration_ms.get() as f64) as u64,
                ));
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MusicusPlayer {
        const NAME: &'static str = "MusicusPlayer";
        type Type = super::MusicusPlayer;
    }

    #[glib::derived_properties]
    impl ObjectImpl for MusicusPlayer {
        fn constructed(&self) {
            self.parent_constructed();

            let mpris = MprisPlayer::new(
                "de.johrpan.musicus".to_string(),
                "Musicus".to_string(),
                "de.johrpan.musicus.desktop".to_string(),
            );

            mpris.set_can_play(true);
            mpris.set_can_pause(true);
            mpris.set_can_go_previous(true);
            mpris.set_can_go_next(true);
            mpris.set_can_seek(false);
            mpris.set_can_raise(false);
            mpris.set_can_set_fullscreen(false);

            let obj = self.obj();
            mpris.connect_play(clone!(@weak obj => move || obj.play()));
            mpris.connect_pause(clone!(@weak obj => move || obj.pause()));
            mpris.connect_play_pause(clone!(@weak obj => move || obj.play_pause()));
            mpris.connect_previous(clone!(@weak obj => move || obj.previous()));
            mpris.connect_next(clone!(@weak obj => move || obj.next()));

            self.mpris.set(mpris).expect("mpris should not be set");

            let player = self.player.get().unwrap();

            let mut config = player.config();
            config.set_position_update_interval(250);
            player.set_config(config).unwrap();
            player.set_video_track_enabled(false);

            let obj = Fragile::new(self.obj().to_owned());
            player.connect_end_of_stream(move |_| {
                obj.get().next();
            });

            let obj = Fragile::new(self.obj().to_owned());
            player.connect_position_updated(move |_, current_time| {
                if let Some(current_time) = current_time {
                    let obj = obj.get();
                    let imp = obj.imp();

                    let current_time_ms = current_time.mseconds();
                    let duration_ms = imp.duration_ms.get();
                    let mut position = current_time_ms as f64 / duration_ms as f64;
                    if position > 1.0 {
                        position = 1.0
                    }

                    imp.current_time_ms.set(current_time_ms);
                    obj.notify_current_time_ms();

                    imp.position.set(position);
                    obj.notify_position();
                }
            });

            let obj = Fragile::new(self.obj().to_owned());
            player.connect_duration_changed(move |_, duration| {
                if let Some(duration) = duration {
                    let obj = obj.get();
                    let imp = obj.imp();

                    let duration_ms = duration.mseconds();

                    imp.duration_ms.set(duration_ms);
                    obj.notify_duration_ms();

                    imp.current_time_ms.set(0);
                    obj.notify_current_time_ms();

                    imp.position.set(0.0);
                    obj.notify_position();
                }
            });
        }
    }
}

glib::wrapper! {
    pub struct MusicusPlayer(ObjectSubclass<imp::MusicusPlayer>);
}

impl MusicusPlayer {
    pub fn new() -> Self {
        let player = gstreamer_player::Player::new(
            None::<gstreamer_player::PlayerVideoRenderer>,
            Some(gstreamer_player::PlayerGMainContextSignalDispatcher::new(
                None,
            )),
        );

        glib::Object::builder()
            .property("active", false)
            .property("playing", false)
            .property("playlist", gio::ListStore::new::<PlaylistItem>())
            .property("current-index", 0u32)
            .property("current-time-ms", 0u64)
            .property("duration-ms", 60_000u64)
            .property("position", 0.0)
            .property("player", player)
            .build()
    }

    pub fn append(&self, tracks: Vec<PlaylistItem>) {
        let playlist = self.playlist();

        for track in tracks {
            playlist.append(&track);
        }

        if !self.active() && playlist.n_items() > 0 {
            self.set_active(true);
            self.set_current_index(0);
            self.play();
        }
    }

    pub fn play_pause(&self) {
        if self.playing() {
            self.pause();
        } else {
            self.play();
        }
    }

    pub fn play(&self) {
        self.player().play();
        self.set_playing(true);
        self.imp().mpris.get().unwrap().set_playback_status(PlaybackStatus::Playing);
    }

    pub fn pause(&self) {
        self.player().pause();
        self.set_playing(false);
        self.imp().mpris.get().unwrap().set_playback_status(PlaybackStatus::Paused);
    }

    pub fn current_item(&self) -> Option<PlaylistItem> {
        let imp = self.imp();
        imp.playlist
            .get()
            .unwrap()
            .item(imp.current_index.get())
            .and_downcast::<PlaylistItem>()
    }

    pub fn next(&self) {
        if self.current_index() < self.playlist().n_items() - 1 {
            self.set_current_index(self.current_index() + 1);
        }
    }

    pub fn previous(&self) {
        if self.current_index() > 0 {
            self.set_current_index(self.current_index() - 1);
        }
    }
}

impl Default for MusicusPlayer {
    fn default() -> Self {
        Self::new()
    }
}
