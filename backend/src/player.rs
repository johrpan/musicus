use crate::{Error, Result};
use musicus_database::TrackSet;
use glib::clone;
use gstreamer_player::prelude::*;
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

#[cfg(target_os = "linux")]
use mpris_player::{Metadata, MprisPlayer, PlaybackStatus};

#[derive(Clone)]
pub struct PlaylistItem {
    pub track_set: TrackSet,
    pub indices: Vec<usize>,
}

pub struct Player {
    music_library_path: PathBuf,
    player: gstreamer_player::Player,
    playlist: RefCell<Vec<PlaylistItem>>,
    current_item: Cell<Option<usize>>,
    current_track: Cell<Option<usize>>,
    playing: Cell<bool>,
    playlist_cbs: RefCell<Vec<Box<dyn Fn(Vec<PlaylistItem>)>>>,
    track_cbs: RefCell<Vec<Box<dyn Fn(usize, usize)>>>,
    duration_cbs: RefCell<Vec<Box<dyn Fn(u64)>>>,
    playing_cbs: RefCell<Vec<Box<dyn Fn(bool)>>>,
    position_cbs: RefCell<Vec<Box<dyn Fn(u64)>>>,
    raise_cb: RefCell<Option<Box<dyn Fn()>>>,

    #[cfg(target_os = "linux")]
    mpris: Arc<MprisPlayer>,
}

impl Player {
    pub fn new(music_library_path: PathBuf) -> Rc<Self> {
        let dispatcher = gstreamer_player::PlayerGMainContextSignalDispatcher::new(None);
        let player = gstreamer_player::Player::new(None, Some(&dispatcher.upcast()));
        let mut config = player.get_config();
        config.set_position_update_interval(250);
        player.set_config(config).unwrap();
        player.set_video_track_enabled(false);

        let result = Rc::new(Self {
            music_library_path,
            player: player.clone(),
            playlist: RefCell::new(Vec::new()),
            current_item: Cell::new(None),
            current_track: Cell::new(None),
            playing: Cell::new(false),
            playlist_cbs: RefCell::new(Vec::new()),
            track_cbs: RefCell::new(Vec::new()),
            duration_cbs: RefCell::new(Vec::new()),
            playing_cbs: RefCell::new(Vec::new()),
            position_cbs: RefCell::new(Vec::new()),
            raise_cb: RefCell::new(None),
            #[cfg(target_os = "linux")]
            mpris: {
                let mpris = MprisPlayer::new(
                    "de.johrpan.musicus".to_string(),
                    "Musicus".to_string(),
                    "de.johrpan.musicus.desktop".to_string(),
                );

                mpris.set_can_raise(true);
                mpris.set_can_play(false);
                mpris.set_can_go_previous(false);
                mpris.set_can_go_next(false);
                mpris.set_can_seek(false);
                mpris.set_can_set_fullscreen(false);

                mpris
            },
        });

        let clone = fragile::Fragile::new(result.clone());
        player.connect_end_of_stream(move |_| {
            let clone = clone.get();
            if clone.has_next() {
                clone.next().unwrap();
            } else {
                clone.player.stop();
                clone.playing.replace(false);

                for cb in &*clone.playing_cbs.borrow() {
                    cb(false);
                }

                #[cfg(target_os = "linux")]
                clone.mpris.set_playback_status(PlaybackStatus::Paused);
            }
        });

        let clone = fragile::Fragile::new(result.clone());
        player.connect_position_updated(move |_, position| {
            for cb in &*clone.get().position_cbs.borrow() {
                cb(position.mseconds().unwrap());
            }
        });

        let clone = fragile::Fragile::new(result.clone());
        player.connect_duration_changed(move |_, duration| {
            for cb in &*clone.get().duration_cbs.borrow() {
                cb(duration.mseconds().unwrap());
            }
        });

        #[cfg(target_os = "linux")]
        {
            result.mpris.connect_play_pause(clone!(@weak result => move || {
                result.play_pause();
            }));

            result.mpris.connect_play(clone!(@weak result => move || {
                if !result.is_playing() {
                    result.play_pause();
                }
            }));

            result.mpris.connect_pause(clone!(@weak result => move || {
                if result.is_playing() {
                    result.play_pause();
                }
            }));

            result.mpris.connect_previous(clone!(@weak result => move || {
                let _ = result.previous();
            }));

            result.mpris.connect_next(clone!(@weak result => move || {
                let _ = result.next();
            }));

            result.mpris.connect_raise(clone!(@weak result => move || {
                let cb = result.raise_cb.borrow();
                if let Some(cb) = &*cb {
                    cb()
                }
            }));
        }

        result
    }

    pub fn add_playlist_cb<F: Fn(Vec<PlaylistItem>) + 'static>(&self, cb: F) {
        self.playlist_cbs.borrow_mut().push(Box::new(cb));
    }

    pub fn add_track_cb<F: Fn(usize, usize) + 'static>(&self, cb: F) {
        self.track_cbs.borrow_mut().push(Box::new(cb));
    }

    pub fn add_duration_cb<F: Fn(u64) + 'static>(&self, cb: F) {
        self.duration_cbs.borrow_mut().push(Box::new(cb));
    }

    pub fn add_playing_cb<F: Fn(bool) + 'static>(&self, cb: F) {
        self.playing_cbs.borrow_mut().push(Box::new(cb));
    }

    pub fn add_position_cb<F: Fn(u64) + 'static>(&self, cb: F) {
        self.position_cbs.borrow_mut().push(Box::new(cb));
    }

    pub fn set_raise_cb<F: Fn() + 'static>(&self, cb: F) {
        self.raise_cb.replace(Some(Box::new(cb)));
    }

    pub fn get_playlist(&self) -> Vec<PlaylistItem> {
        self.playlist.borrow().clone()
    }

    pub fn get_current_item(&self) -> Option<usize> {
        self.current_item.get()
    }

    pub fn get_current_track(&self) -> Option<usize> {
        self.current_track.get()
    }

    pub fn get_duration(&self) -> gstreamer::ClockTime {
        self.player.get_duration()
    }

    pub fn is_playing(&self) -> bool {
        self.playing.get()
    }

    pub fn add_item(&self, item: PlaylistItem) -> Result<()> {
        if item.indices.is_empty() {
            Err(Error::Other(String::from("Tried to add an empty playlist item!")))
        } else {
            let was_empty = {
                let mut playlist = self.playlist.borrow_mut();
                let was_empty = playlist.is_empty();

                playlist.push(item);

                was_empty
            };

            for cb in &*self.playlist_cbs.borrow() {
                cb(self.playlist.borrow().clone());
            }

            if was_empty {
                self.set_track(0, 0)?;
                self.player.play();
                self.playing.set(true);

                for cb in &*self.playing_cbs.borrow() {
                    cb(true);
                }

                #[cfg(target_os = "linux")]
                {
                    self.mpris.set_can_play(true);
                    self.mpris.set_playback_status(PlaybackStatus::Playing);
                }
            }

            Ok(())
        }
    }

    pub fn play_pause(&self) {
        if self.is_playing() {
            self.player.pause();
            self.playing.set(false);

            for cb in &*self.playing_cbs.borrow() {
                cb(false);
            }

            #[cfg(target_os = "linux")]
            self.mpris.set_playback_status(PlaybackStatus::Paused);
        } else {
            self.player.play();
            self.playing.set(true);

            for cb in &*self.playing_cbs.borrow() {
                cb(true);
            }

            #[cfg(target_os = "linux")]
            self.mpris.set_playback_status(PlaybackStatus::Playing);
        }
    }

    pub fn seek(&self, ms: u64) {
        self.player.seek(gstreamer::ClockTime::from_mseconds(ms));
    }

    pub fn has_previous(&self) -> bool {
        if let Some(current_item) = self.current_item.get() {
            if let Some(current_track) = self.current_track.get() {
                current_track > 0 || current_item > 0
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn previous(&self) -> Result<()> {
        let mut current_item = self.current_item.get()
            .ok_or(Error::Other(String::from("Player tried to access non existant current item.")))?;

        let mut current_track = self
            .current_track
            .get()
            .ok_or(Error::Other(String::from("Player tried to access non existant current track.")))?;

        let playlist = self.playlist.borrow();
        if current_track > 0 {
            current_track -= 1;
        } else if current_item > 0 {
            current_item -= 1;
            current_track = playlist[current_item].indices.len() - 1;
        } else {
            return Err(Error::Other(String::from("No existing previous track.")));
        }

        self.set_track(current_item, current_track)
    }

    pub fn has_next(&self) -> bool {
        if let Some(current_item) = self.current_item.get() {
            if let Some(current_track) = self.current_track.get() {
                let playlist = self.playlist.borrow();
                let item = &playlist[current_item];

                current_track + 1 < item.indices.len() || current_item + 1 < playlist.len()
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn next(&self) -> Result<()> {
        let mut current_item = self.current_item.get()
            .ok_or(Error::Other(String::from("Player tried to access non existant current item.")))?;
        let mut current_track = self
            .current_track
            .get()
            .ok_or(Error::Other(String::from("Player tried to access non existant current track.")))?;

        let playlist = self.playlist.borrow();
        let item = &playlist[current_item];
        if current_track + 1 < item.indices.len() {
            current_track += 1;
        } else if current_item + 1 < playlist.len() {
            current_item += 1;
            current_track = 0;
        } else {
            return Err(Error::Other(String::from("No existing previous track.")));
        }

        self.set_track(current_item, current_track)
    }

    pub fn set_track(&self, current_item: usize, current_track: usize) -> Result<()> {
        let item = &self.playlist.borrow()[current_item];
        let track = &item.track_set.tracks[current_track];

        let path = self.music_library_path.join(track.path.clone())
            .into_os_string().into_string().unwrap();

        let uri = glib::filename_to_uri(&path, None)
            .or(Err(Error::Other(format!("Failed to create URI from path: {}", path))))?;

        self.player.set_uri(&uri);

        if self.is_playing() {
            self.player.play();
        }

        self.current_item.set(Some(current_item));
        self.current_track.set(Some(current_track));

        for cb in &*self.track_cbs.borrow() {
            cb(current_item, current_track);
        }

        #[cfg(target_os = "linux")]
        {
            let mut parts = Vec::<String>::new();
            for part in &track.work_parts {
                parts.push(item.track_set.recording.work.parts[*part].title.clone());
            }

            let mut title = item.track_set.recording.work.get_title();
            if !parts.is_empty() {
                title = format!("{}: {}", title, parts.join(", "));
            }

            let subtitle = item.track_set.recording.get_performers();

            let mut metadata = Metadata::new();
            metadata.artist = Some(vec![title]);
            metadata.title = Some(subtitle);

            self.mpris.set_metadata(metadata);
            self.mpris.set_can_go_previous(self.has_previous());
            self.mpris.set_can_go_next(self.has_next());
        }

        Ok(())
    }

    pub fn send_data(&self) {
        for cb in &*self.playlist_cbs.borrow() {
            cb(self.playlist.borrow().clone());
        }

        for cb in &*self.track_cbs.borrow() {
            cb(self.current_item.get().unwrap(), self.current_track.get().unwrap());
        }

        for cb in &*self.duration_cbs.borrow() {
            cb(self.player.get_duration().mseconds().unwrap());
        }

        for cb in &*self.playing_cbs.borrow() {
            cb(self.is_playing());
        }
    }

    pub fn clear(&self) {
        self.player.stop();
        self.playing.set(false);
        self.current_item.set(None);
        self.current_track.set(None);
        self.playlist.replace(Vec::new());

        for cb in &*self.playing_cbs.borrow() {
            cb(false);
        }

        for cb in &*self.playlist_cbs.borrow() {
            cb(Vec::new());
        }

        #[cfg(target_os = "linux")]
        self.mpris.set_can_play(false);
    }
}
