use crate::database::*;
use anyhow::anyhow;
use anyhow::Result;
use gstreamer_player::prelude::*;
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

#[derive(Clone)]
pub struct PlaylistItem {
    pub tracks: TrackSet,
    pub file_names: Vec<String>,
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
            Err(anyhow!(
                "Tried to add playlist item without tracks to playlist!"
            ))
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
        } else {
            self.player.play();
            self.playing.set(true);

            for cb in &*self.playing_cbs.borrow() {
                cb(true);
            }
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
        let mut current_item = self.current_item.get().ok_or(anyhow!("No current item!"))?;
        let mut current_track = self
            .current_track
            .get()
            .ok_or(anyhow!("No current track!"))?;

        let playlist = self.playlist.borrow();
        if current_track > 0 {
            current_track -= 1;
        } else if current_item > 0 {
            current_item -= 1;
            current_track = playlist[current_item].indices.len() - 1;
        } else {
            return Err(anyhow!("No previous track!"));
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
        let mut current_item = self.current_item.get().ok_or(anyhow!("No current item!"))?;
        let mut current_track = self
            .current_track
            .get()
            .ok_or(anyhow!("No current track!"))?;

        let playlist = self.playlist.borrow();
        let item = &playlist[current_item];
        if current_track + 1 < item.indices.len() {
            current_track += 1;
        } else if current_item + 1 < playlist.len() {
            current_item += 1;
            current_track = 0;
        } else {
            return Err(anyhow!("No next track!"));
        }

        self.set_track(current_item, current_track)
    }

    pub fn set_track(&self, current_item: usize, current_track: usize) -> Result<()> {
        let uri = format!(
            "file://{}",
            self.music_library_path
                .join(
                    self.playlist.borrow()[current_item].file_names[current_track].clone(),
                )
                .to_str()
                .unwrap(),
        );

        self.player.set_uri(&uri);
        if self.is_playing() {
            self.player.play();
        }

        self.current_item.set(Some(current_item));
        self.current_track.set(Some(current_track));

        for cb in &*self.track_cbs.borrow() {
            cb(current_item, current_track);
        }

        Ok(())
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
    }
}
