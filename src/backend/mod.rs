use futures_channel::mpsc;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

pub mod client;
pub use client::*;

pub mod database;
pub use database::*;

pub mod error;
pub use error::*;

pub mod library;
pub use library::*;

pub mod player;
pub use player::*;

mod secure;

/// General states the application can be in.
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
    pub state_stream: RefCell<mpsc::Receiver<BackendState>>,
    state_sender: RefCell<mpsc::Sender<BackendState>>,
    settings: gio::Settings,
    music_library_path: RefCell<Option<PathBuf>>,
    database: RefCell<Option<Rc<DbThread>>>,
    player: RefCell<Option<Rc<Player>>>,
    server_url: RefCell<Option<String>>,
    login_data: RefCell<Option<LoginData>>,
    token: RefCell<Option<String>>,
}

impl Backend {
    /// Create a new backend initerface. The user interface should subscribe to the state stream
    /// and call init() afterwards.
    pub fn new() -> Self {
        let (state_sender, state_stream) = mpsc::channel(1024);

        Backend {
            state_stream: RefCell::new(state_stream),
            state_sender: RefCell::new(state_sender),
            settings: gio::Settings::new("de.johrpan.musicus"),
            music_library_path: RefCell::new(None),
            database: RefCell::new(None),
            player: RefCell::new(None),
            server_url: RefCell::new(None),
            login_data: RefCell::new(None),
            token: RefCell::new(None),
        }
    }

    /// Initialize the backend updating the state accordingly.
    pub async fn init(self: Rc<Backend>) -> Result<()> {
        self.init_library().await?;
        self.init_client()?;

        Ok(())
    }

    /// Set the current state and notify the user interface.
    fn set_state(&self, state: BackendState) {
        self.state_sender.borrow_mut().try_send(state).unwrap();
    }
}
