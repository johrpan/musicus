use gio::traits::SettingsExt;
use log::warn;
use musicus_database::Database;
use std::{
    cell::{Cell, RefCell},
    path::PathBuf,
    rc::Rc,
    sync::Arc,
};
use tokio::sync::{broadcast, broadcast::Sender};

pub use musicus_database as db;
pub use musicus_import as import;

pub mod error;
pub use error::*;

pub mod library;
pub use library::*;

mod logger;
pub use logger::{LogMessage, Logger};

pub mod player;
pub use player::*;

/// General states the application can be in.
#[derive(Debug, Copy, Clone)]
pub enum BackendState {
    /// The backend is not set up yet. This means that no backend methods except for setting the
    /// music library path should be called. The user interface should adapt and only present this
    /// option.
    NoMusicLibrary,

    /// The backend is loading the music library. No methods should be called. The user interface
    /// should represent that state by prohibiting all interaction.
    Loading,

    /// The backend is ready and all methods may be called.
    Ready,
}

/// A collection of all backend state and functionality.
pub struct Backend {
    /// Registered instance of [Logger].
    logger: Arc<Logger>,

    /// A closure that will be called whenever the backend state changes.
    state_cb: RefCell<Option<Box<dyn Fn(BackendState)>>>,

    /// Access to GSettings.
    settings: gio::Settings,

    /// The current path to the music library, which is used by the player and the database. This
    /// is guaranteed to be Some, when the state is set to BackendState::Ready.
    music_library_path: RefCell<Option<PathBuf>>,

    /// The sender for sending library update notifications.
    library_updated_sender: Sender<()>,

    /// The database. This can be assumed to exist, when the state is set to BackendState::Ready.
    database: RefCell<Option<Rc<Database>>>,

    /// The player handling playlist and playback. This can be assumed to exist, when the state is
    /// set to BackendState::Ready.
    player: RefCell<Option<Rc<Player>>>,

    /// Whether to keep playing random tracks after the playlist ends.
    keep_playing: Cell<bool>,

    /// Whether to choose full recordings for random playback.
    play_full_recordings: Cell<bool>,
}

impl Backend {
    /// Create a new backend initerface. The user interface should subscribe to the state stream
    /// and call init() afterwards. There may be only one backend for a process and this method
    /// may only be called exactly once. Otherwise it will panic.
    pub fn new() -> Self {
        let logger = logger::register();
        let (library_updated_sender, _) = broadcast::channel(1024);

        Backend {
            logger,
            state_cb: RefCell::new(None),
            settings: gio::Settings::new("de.johrpan.musicus"),
            music_library_path: RefCell::new(None),
            library_updated_sender,
            database: RefCell::new(None),
            player: RefCell::new(None),
            keep_playing: Cell::new(false),
            play_full_recordings: Cell::new(true),
        }
    }

    /// Get the registered instance of [Logger].
    pub fn logger(&self) -> Arc<Logger> {
        Arc::clone(&self.logger)
    }

    /// Set the closure to be called whenever the backend state changes.
    pub fn set_state_cb<F: Fn(BackendState) + 'static>(&self, cb: F) {
        self.state_cb.replace(Some(Box::new(cb)));
    }

    /// Initialize the backend. A state callback should already have been registered using
    /// [`set_state_cb()`] to react to the result.
    pub fn init(self: Rc<Self>) -> Result<()> {
        self.keep_playing.set(self.settings.boolean("keep-playing"));
        self.play_full_recordings
            .set(self.settings.boolean("play-full-recordings"));

        Rc::clone(&self).init_library()?;

        match self.get_music_library_path() {
            None => self.set_state(BackendState::NoMusicLibrary),
            Some(_) => self.set_state(BackendState::Ready),
        };

        Ok(())
    }

    /// Whether to keep playing random tracks after the playlist ends.
    pub fn keep_playing(&self) -> bool {
        self.keep_playing.get()
    }

    /// Set whether to keep playing random tracks after the playlist ends.
    pub fn set_keep_playing(self: Rc<Self>, keep_playing: bool) {
        if let Err(err) = self.settings.set_boolean("keep-playing", keep_playing) {
            warn!(
                "The preference \"keep-playing\" could not be saved using GSettings. It will most \
                likely not be available at the next startup. Error message: {}",
                err
            );
        }

        self.keep_playing.set(keep_playing);
        self.update_track_generator();
    }

    /// Whether to choose full recordings for random playback.
    pub fn play_full_recordings(&self) -> bool {
        self.play_full_recordings.get()
    }

    /// Set whether to choose full recordings for random playback.
    pub fn set_play_full_recordings(self: Rc<Self>, play_full_recordings: bool) {
        if let Err(err) = self
            .settings
            .set_boolean("play-full-recordings", play_full_recordings)
        {
            warn!(
                "The preference \"play-full-recordings\" could not be saved using GSettings. It \
                will most likely not be available at the next startup. Error message: {}",
                err
            );
        }

        self.play_full_recordings.set(play_full_recordings);
        self.update_track_generator();
    }

    /// Set the current state and notify the user interface.
    fn set_state(&self, state: BackendState) {
        if let Some(cb) = &*self.state_cb.borrow() {
            cb(state);
        }
    }

    /// Apply the current track generation settings.
    fn update_track_generator(self: Rc<Self>) {
        if let Some(player) = self.get_player() {
            if self.keep_playing() {
                if self.play_full_recordings() {
                    player.set_track_generator(Some(RandomRecordingGenerator::new(self)));
                } else {
                    player.set_track_generator(Some(RandomTrackGenerator::new(self)));
                }
            } else {
                player.set_track_generator(None::<RandomRecordingGenerator>);
            }
        }
    }
}

impl Default for Backend {
    fn default() -> Self {
        Self::new()
    }
}
