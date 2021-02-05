use futures::prelude::*;
use futures_channel::mpsc;
use gio::prelude::*;
use log::warn;
use musicus_client::{Client, LoginData};
use musicus_database::DbThread;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

pub use musicus_client as client;
pub use musicus_database as db;

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
    /// A future resolving to the next state of the backend. Initially, this should be assumed to
    /// be BackendState::Loading. Changes should be awaited before calling init().
    state_stream: RefCell<mpsc::Receiver<BackendState>>,

    /// The internal sender to publish the state via state_stream.
    state_sender: RefCell<mpsc::Sender<BackendState>>,

    /// Access to GSettings.
    settings: gio::Settings,

    /// The current path to the music library, which is used by the player and the database. This
    /// is guaranteed to be Some, when the state is set to BackendState::Ready.
    music_library_path: RefCell<Option<PathBuf>>,

    /// The database. This can be assumed to exist, when the state is set to BackendState::Ready.
    database: RefCell<Option<Rc<DbThread>>>,

    /// The player handling playlist and playback. This can be assumed to exist, when the state is
    /// set to BackendState::Ready.
    player: RefCell<Option<Rc<Player>>>,

    /// A client for the Wolfgang server.
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

    /// Wait for the next state change. Initially, the state should be assumed to be
    /// BackendState::Loading. Changes should be awaited before calling init().
    pub async fn next_state(&self) -> Option<BackendState> {
        self.state_stream.borrow_mut().next().await
    }

    /// Initialize the backend updating the state accordingly.
    pub async fn init(&self) -> Result<()> {
        self.init_library().await?;

        if let Some(url) = self.settings.get_string("server-url") {
            if !url.is_empty() {
                self.client.set_server_url(&url);
            }
        }

        match Self::load_login_data().await {
            Ok(Some(data)) => self.client.set_login_data(data),
            Err(err) => warn!("The login data could not be loaded from SecretService. It will not \
                be available. Error message: {}", err),
            _ => (),
        }

        if self.get_music_library_path().is_none() {
            self.set_state(BackendState::NoMusicLibrary);
        } else {
            self.set_state(BackendState::Ready);
        }

        Ok(())
    }

    /// Set the URL of the Musicus server to connect to.
    pub fn set_server_url(&self, url: &str) {
        if let Err(err) = self.settings.set_string("server-url", url) {
            warn!("An error happened while trying to save the server URL to GSettings. Most \
                likely it will not be available at the next startup. Error message: {}", err);
        }

        self.client.set_server_url(url);
    }

    /// Get the currently set server URL.
    pub fn get_server_url(&self) -> Option<String> {
        self.client.get_server_url()
    }

    /// Set the user credentials to use.
    pub async fn set_login_data(&self, data: LoginData) {
        if let Err(err) = Self::store_login_data(data.clone()).await {
            warn!("An error happened while trying to store the login data using SecretService. \
                This means, that they will not be available at the next startup most likely. \
                Error message: {}", err);
        }

        self.client.set_login_data(data);
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
