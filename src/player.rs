use crate::database::*;
use anyhow::anyhow;
use anyhow::Result;
use gstreamer_player::prelude::*;
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

#[derive(Clone)]
pub struct PlaylistItem {
    pub recording: RecordingDescription,
    pub tracks: Vec<TrackDescription>,
}

pub struct Player {
    music_library_path: PathBuf,
    player: gstreamer_player::Player,
    playlist: RefCell<Vec<PlaylistItem>>,
    current_item: Cell<Option<usize>>,
    current_track: Cell<Option<usize>>,
    playing: Cell<bool>,
    playlist_cb: RefCell<Option<Box<dyn Fn(Vec<PlaylistItem>) -> ()>>>,
    track_cb: RefCell<Option<Box<dyn Fn(usize, usize) -> ()>>>,
    duration_cb: RefCell<Option<Box<dyn Fn(u64) -> ()>>>,
    playing_cb: RefCell<Option<Box<dyn Fn(bool) -> ()>>>,
    position_cb: RefCell<Option<Box<dyn Fn(u64) -> ()>>>,
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
            playlist_cb: RefCell::new(None),
            track_cb: RefCell::new(None),
            duration_cb: RefCell::new(None),
            playing_cb: RefCell::new(None),
            position_cb: RefCell::new(None),
        });

        let clone = fragile::Fragile::new(result.clone());
        player.connect_end_of_stream(move |_| {
            let clone = clone.get();
            if clone.has_next() {
                clone.next().unwrap();
            } else {
                clone.player.stop();

                if let Some(cb) = &*clone.playing_cb.borrow() {
                    cb(false);
                }
            }
        });

        let clone = fragile::Fragile::new(result.clone());
        player.connect_position_updated(move |_, position| {
            if let Some(cb) = &*clone.get().position_cb.borrow() {
                cb(position.mseconds().unwrap());
            }
        });

        let clone = fragile::Fragile::new(result.clone());
        player.connect_duration_changed(move |_, duration| {
            if let Some(cb) = &*clone.get().duration_cb.borrow() {
                cb(duration.mseconds().unwrap());
            }
        });

        result
    }

    pub fn set_playlist_cb<F: Fn(Vec<PlaylistItem>) -> () + 'static>(&self, cb: F) {
        self.playlist_cb.replace(Some(Box::new(cb)));
    }

    pub fn set_track_cb<F: Fn(usize, usize) -> () + 'static>(&self, cb: F) {
        self.track_cb.replace(Some(Box::new(cb)));
    }

    pub fn set_duration_cb<F: Fn(u64) -> () + 'static>(&self, cb: F) {
        self.duration_cb.replace(Some(Box::new(cb)));
    }

    pub fn set_playing_cb<F: Fn(bool) -> () + 'static>(&self, cb: F) {
        self.playing_cb.replace(Some(Box::new(cb)));
    }

    pub fn set_position_cb<F: Fn(u64) -> () + 'static>(&self, cb: F) {
        self.position_cb.replace(Some(Box::new(cb)));
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
        if item.tracks.is_empty() {
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

            if let Some(cb) = &*self.playlist_cb.borrow() {
                cb(self.playlist.borrow().clone());
            }

            if was_empty {
                self.set_track(0, 0)?;
                self.player.play();
                self.playing.set(true);

                if let Some(cb) = &*self.playing_cb.borrow() {
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

            if let Some(cb) = &*self.playing_cb.borrow() {
                cb(false);
            }
        } else {
            self.player.play();
            self.playing.set(true);

            if let Some(cb) = &*self.playing_cb.borrow() {
                cb(true);
            }
        }
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
            current_track = playlist[current_item].tracks.len() - 1;
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

                current_track + 1 < item.tracks.len() || current_item + 1 < playlist.len()
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
        if current_track + 1 < item.tracks.len() {
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
                    self.playlist
                        .borrow()
                        .get(current_item)
                        .ok_or(anyhow!("Playlist item doesn't exist!"))?
                        .tracks
                        .get(current_track)
                        .ok_or(anyhow!("Track doesn't exist!"))?
                        .file_name
                        .clone(),
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

        if let Some(cb) = &*self.track_cb.borrow() {
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

        if let Some(cb) = &*self.playing_cb.borrow() {
            cb(false);
        }

        if let Some(cb) = &*self.playlist_cb.borrow() {
            cb(Vec::new());
        }
    }
}
