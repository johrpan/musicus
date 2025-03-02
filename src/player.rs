use std::{
    cell::{Cell, OnceCell, RefCell},
    path::PathBuf,
};

use anyhow::{anyhow, Context, Result};
use fragile::Fragile;
use gstreamer_play::gst;
use gtk::{
    gio,
    glib::{self, clone, subclass::Signal, Properties},
    prelude::*,
    subclass::prelude::*,
};
use once_cell::sync::Lazy;

use crate::{
    config,
    db::models::{Recording, Track},
    library::Library,
    playlist_item::PlaylistItem,
    program::Program,
};

mod imp {
    use super::*;

    #[derive(Properties, Debug, Default)]
    #[properties(wrapper_type = super::Player)]
    pub struct Player {
        #[property(get, set)]
        pub library: RefCell<Option<Library>>,
        #[property(get, set)]
        pub active: Cell<bool>,
        #[property(get, set)]
        pub playing: Cell<bool>,
        #[property(get, set)]
        pub program: RefCell<Option<Program>>,
        #[property(get, construct_only)]
        pub playlist: OnceCell<gio::ListStore>,
        #[property(get, set = Self::set_current_index)]
        pub current_index: Cell<u32>,
        #[property(get, set)]
        pub duration_ms: Cell<u64>,
        #[property(get, set)]
        pub position_ms: Cell<u64>,

        pub play: OnceCell<gstreamer_play::Play>,
        pub play_signal_adapter: OnceCell<gstreamer_play::PlaySignalAdapter>,
        pub mpris: OnceCell<mpris_server::Player>,
    }

    impl Player {
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

                let obj = self.obj().clone();
                let item_clone = item.clone();
                glib::spawn_future_local(async move {
                    obj.imp()
                        .mpris
                        .get()
                        .unwrap()
                        .set_metadata(
                            mpris_server::Metadata::builder()
                                .artist(vec![item_clone.make_title()])
                                .title(item_clone.make_subtitle().unwrap_or_else(String::new))
                                .build(),
                        )
                        .await
                        .unwrap();
                });

                let uri = glib::filename_to_uri(item.path(), None)
                    .expect("track path should be parsable as an URI");

                let play = self.play.get().unwrap();
                play.set_uri(Some(&uri));
                if self.playing.get() {
                    play.play();
                }

                self.current_index.set(index);
                item.set_is_playing(true);

                self.library
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .track_played(&item.track_id())
                    .unwrap();
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Player {
        const NAME: &'static str = "MusicusPlayer";
        type Type = super::Player;
    }

    #[glib::derived_properties]
    impl ObjectImpl for Player {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> =
                Lazy::new(|| vec![Signal::builder("raise").build()]);

            SIGNALS.as_ref()
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj().clone();
            glib::spawn_future_local(async move {
                obj.init_mpris().await;
            });

            let play = gstreamer_play::Play::new(None::<gstreamer_play::PlayVideoRenderer>);

            let mut config = play.config();
            config.set_position_update_interval(250);
            play.set_config(config).unwrap();
            play.set_video_track_enabled(false);

            let play_signal_adapter = gstreamer_play::PlaySignalAdapter::new(&play);

            let obj = Fragile::new(self.obj().to_owned());
            play_signal_adapter.connect_end_of_stream(move |_| {
                obj.get().next();
            });

            let obj = Fragile::new(self.obj().to_owned());
            play_signal_adapter.connect_position_updated(move |_, position| {
                if let Some(position) = position {
                    let obj = obj.get();
                    obj.imp().position_ms.set(position.mseconds());
                    obj.notify_position_ms();
                }
            });

            let obj = Fragile::new(self.obj().to_owned());
            play_signal_adapter.connect_duration_changed(move |_, duration| {
                if let Some(duration) = duration {
                    let obj = obj.get();
                    let imp = obj.imp();

                    imp.position_ms.set(0);
                    obj.notify_position_ms();

                    imp.duration_ms.set(duration.mseconds());
                    obj.notify_duration_ms();
                }
            });

            self.play.set(play).unwrap();
            self.play_signal_adapter.set(play_signal_adapter).unwrap();
        }
    }
}

glib::wrapper! {
    pub struct Player(ObjectSubclass<imp::Player>);
}

impl Player {
    pub fn new() -> Self {
        glib::Object::builder()
            .property("active", false)
            .property("playing", false)
            .property("playlist", gio::ListStore::new::<PlaylistItem>())
            .property("current-index", 0u32)
            .property("position-ms", 0u64)
            .property("duration-ms", 60_000u64)
            .build()
    }

    pub fn connect_raise<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("raise", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            f(&obj);
            None
        })
    }

    pub fn recording_to_playlist(&self, recording: &Recording) -> Vec<PlaylistItem> {
        let tracks = &self
            .library()
            .unwrap()
            .tracks_for_recording(&recording.recording_id)
            .unwrap();

        if tracks.is_empty() {
            log::warn!("Recording without tracks: {}.", &recording.recording_id);
            return Vec::new();
        }

        let performances = recording.performers_string();

        let mut items = Vec::new();

        if tracks.len() == 1 {
            items.push(PlaylistItem::new(
                true,
                recording.work.composers_string(),
                &recording.work.name.get(),
                Some(&performances),
                None,
                &self.library_path_to_file_path(&tracks[0].path),
                &tracks[0].track_id,
            ));
        } else {
            let mut tracks = tracks.into_iter();
            let first_track = tracks.next().unwrap();

            let track_title = |track: &Track, number: usize| -> String {
                let title = track
                    .works
                    .iter()
                    .map(|w| w.name.get().to_string())
                    .collect::<Vec<String>>()
                    .join(", ");

                if title.is_empty() {
                    format!("Track {number}")
                } else {
                    title
                }
            };

            items.push(PlaylistItem::new(
                true,
                recording.work.composers_string(),
                &recording.work.name.get(),
                Some(&performances),
                Some(&track_title(&first_track, 1)),
                &self.library_path_to_file_path(&first_track.path),
                &first_track.track_id,
            ));

            for (index, track) in tracks.enumerate() {
                items.push(PlaylistItem::new(
                    false,
                    recording.work.composers_string(),
                    &recording.work.name.get(),
                    Some(&performances),
                    // track number = track index + 1 (first track) + 1 (zero based)
                    Some(&track_title(&track, index + 2)),
                    &self.library_path_to_file_path(&track.path),
                    &track.track_id,
                ));
            }
        }

        items
    }

    /// Append playlist items to the playlist and return the index of the first newly added item.
    /// An error will be returned if `items` is empty.
    pub fn append(&self, items: Vec<PlaylistItem>) -> Result<u32> {
        if !items.is_empty() {
            let playlist = self.playlist();
            let first_index = playlist.n_items();

            for item in items {
                playlist.append(&item);
            }

            // If playlist was empty:
            if first_index == 0 {
                self.set_active(true);
                self.set_current_index(0);
            }

            Ok(first_index)
        } else {
            Err(anyhow!("At least one item has to be added to the playlist"))
        }
    }

    /// Append playlist items to the playlist and immediately start playing the first newly added
    /// item. This will discard the error if `items` is empty.
    pub fn append_and_play(&self, items: Vec<PlaylistItem>) {
        match self.append(items) {
            Ok(index) => {
                self.set_current_index(index);
                self.play();
            }
            Err(err) => {
                log::warn!("Failed to append and play items: {err}");
            }
        }
    }

    /// Generate new playlist items based on the current program and immediately start playing the
    /// first new item.
    pub fn play_from_program(&self) {
        if let Some(program) = self.program() {
            match self.generate_items(&program) {
                Ok(index) => {
                    self.set_current_index(index);
                    self.play();
                }
                Err(err) => {
                    log::warn!("Failed to play from program: {err}");
                }
            }
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
        let imp = self.imp();
        imp.play.get().unwrap().play();
        self.set_playing(true);

        let obj = self.clone();
        glib::spawn_future_local(async move {
            obj.imp()
                .mpris
                .get()
                .unwrap()
                .set_playback_status(mpris_server::PlaybackStatus::Playing)
                .await
                .unwrap();
        });
    }

    pub fn pause(&self) {
        let imp = self.imp();
        imp.play.get().unwrap().pause();
        self.set_playing(false);

        let obj = self.clone();
        glib::spawn_future_local(async move {
            obj.imp()
                .mpris
                .get()
                .unwrap()
                .set_playback_status(mpris_server::PlaybackStatus::Paused)
                .await
                .unwrap();
        });
    }

    pub fn seek_to(&self, time_ms: u64) {
        let imp = self.imp();
        imp.play
            .get()
            .unwrap()
            .seek(gst::ClockTime::from_mseconds(time_ms));
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
        } else if let Some(program) = self.program() {
            match self.generate_items(&program) {
                Ok(index) => self.set_current_index(index),
                Err(err) => log::warn!("Failed to continue playing from program: {err}"),
            }
        }
    }

    pub fn previous(&self) {
        if self.current_index() > 0 {
            self.set_current_index(self.current_index() - 1);
        }
    }

    async fn init_mpris(&self) {
        let mpris = mpris_server::Player::builder(config::APP_ID)
            .desktop_entry(config::APP_ID)
            .can_raise(true)
            .can_play(true)
            .can_pause(true)
            .can_go_previous(true)
            .can_go_next(true)
            .build()
            .await
            .unwrap();

        let obj = self.clone();

        mpris.connect_raise(clone!(
            #[weak]
            obj,
            move |_| obj.emit_by_name::<()>("raise", &[])
        ));

        mpris.connect_play(clone!(
            #[weak]
            obj,
            move |_| obj.play()
        ));

        mpris.connect_pause(clone!(
            #[weak]
            obj,
            move |_| obj.pause()
        ));

        mpris.connect_play_pause(clone!(
            #[weak]
            obj,
            move |_| obj.play_pause()
        ));

        mpris.connect_previous(clone!(
            #[weak]
            obj,
            move |_| obj.previous()
        ));

        mpris.connect_next(clone!(
            #[weak]
            obj,
            move |_| obj.next()
        ));

        self.imp()
            .mpris
            .set(mpris)
            .expect("mpris should not be set");
    }

    /// Generate new playlist items based on `program` and return the index of the first newly
    /// added item if successful.
    fn generate_items(&self, program: &Program) -> Result<u32> {
        let recording = self
            .library()
            .unwrap()
            .generate_recording(program)
            .context("Failed to generate playlist items from program")?;

        let playlist = self.recording_to_playlist(&recording);
        self.append(playlist)
    }

    fn library_path_to_file_path(&self, path: &str) -> String {
        PathBuf::from(self.library().unwrap().folder())
            .join(path)
            .to_str()
            .unwrap()
            .to_owned()
    }
}

impl Default for Player {
    fn default() -> Self {
        Self::new()
    }
}
