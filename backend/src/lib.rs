use musicus_database::Database;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use tokio::sync::{broadcast, broadcast::Sender};

pub use musicus_database as db;
pub use musicus_import as import;

pub mod error;
pub use error::*;

pub mod library;
pub use library::*;

mod logger;

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
}

impl Backend {
    /// Create a new backend initerface. The user interface should subscribe to the state stream
    /// and call init() afterwards. There may be only one backend for a process and this method
    /// may only be called exactly once. Otherwise it will panic.
    pub fn new() -> Self {
        logger::register();

        let (library_updated_sender, _) = broadcast::channel(1024);

        Backend {
            state_cb: RefCell::new(None),
            settings: gio::Settings::new("de.johrpan.musicus"),
            music_library_path: RefCell::new(None),
            library_updated_sender,
            database: RefCell::new(None),
            player: RefCell::new(None),
        }
    }

    /// Set the closure to be called whenever the backend state changes.
    pub fn set_state_cb<F: Fn(BackendState) + 'static>(&self, cb: F) {
        self.state_cb.replace(Some(Box::new(cb)));
    }

    /// Initialize the backend. A state callback should already have been registered using
    /// [`set_state_cb()`] to react to the result.
    pub fn init(&self) -> Result<()> {
        self.init_library()?;

        match self.get_music_library_path() {
            None => self.set_state(BackendState::NoMusicLibrary),
            Some(_) => self.set_state(BackendState::Ready),
        };

        Ok(())
    }

    /// Set the current state and notify the user interface.
    fn set_state(&self, state: BackendState) {
        if let Some(cb) = &*self.state_cb.borrow() {
            cb(state);
        }
    }
}

impl Default for Backend {
    fn default() -> Self {
        Self::new()
    }
}
