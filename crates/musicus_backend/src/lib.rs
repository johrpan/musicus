use futures_channel::mpsc;
use gio::prelude::*;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

pub use musicus_client::*;
pub use musicus_database::*;

pub mod error;
pub use error::*;

// Override the identically named types from the other crates.
pub use error::{Error, Result};

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
    client: Client,
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
            client: Client::new(),
        }
    }

    /// Initialize the backend updating the state accordingly.
    pub async fn init(self: Rc<Backend>) -> Result<()> {
        self.init_library().await?;

        if let Some(url) = self.settings.get_string("server-url") {
            if !url.is_empty() {
                self.client.set_server_url(&url);
            }
        }

        if let Some(data) = secure::load_login_data()? {
            self.client.set_login_data(data);
        }

        Ok(())
    }

    /// Set the URL of the Musicus server to connect to.
    pub fn set_server_url(&self, url: &str) -> Result<()> {
        self.settings.set_string("server-url", url)?;
        self.client.set_server_url(url);
        Ok(())
    }

    /// Get the currently set server URL.
    pub fn get_server_url(&self) -> Option<String> {
        self.client.get_server_url()
    }

    /// Set the user credentials to use.
    pub async fn set_login_data(&self, data: LoginData) -> Result<()> {
        secure::store_login_data(data.clone()).await?;
        self.client.set_login_data(data);
        Ok(())
    }

    pub fn cl(&self) -> &Client {
        &self.client
    }

    /// Get the currently stored login credentials.
    pub fn get_login_data(&self) -> Option<LoginData> {
        self.client.get_login_data()
    }

    /// Set the current state and notify the user interface.
    fn set_state(&self, state: BackendState) {
        self.state_sender.borrow_mut().try_send(state).unwrap();
    }
}
