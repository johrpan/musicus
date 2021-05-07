use gio::prelude::*;
use log::warn;
use musicus_client::{Client, LoginData};
use musicus_database::DbThread;
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;
use tokio::sync::{broadcast, broadcast::Sender};

pub use musicus_client as client;
pub use musicus_database as db;
pub use musicus_import as import;

pub mod error;
pub use error::*;

pub mod library;
pub use library::*;

mod logger;

pub mod player;
pub use player::*;

#[cfg(all(feature = "dbus"))]
mod secure;

/// General states the application can be in.
#[derive(Debug, Clone)]
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
    /// The internal sender to publish the state via state_stream.
    state_sender: Sender<BackendState>,

    /// Access to GSettings.
    settings: gio::Settings,

    /// Whether the server should be used by default when searching for or changing items.
    use_server: Cell<bool>,

    /// The current path to the music library, which is used by the player and the database. This
    /// is guaranteed to be Some, when the state is set to BackendState::Ready.
    music_library_path: RefCell<Option<PathBuf>>,

    /// The sender for sending library update notifications.
    library_updated_sender: Sender<()>,

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
    /// and call init() afterwards. There may be only one backend for a process and this method
    /// may only be called exactly once. Otherwise it will panic.
    pub fn new() -> Self {
        logger::register();

        let (state_sender, _) = broadcast::channel(1024);
        let (library_updated_sender, _) = broadcast::channel(1024);

        Backend {
            state_sender,
            settings: gio::Settings::new("de.johrpan.musicus"),
            use_server: Cell::new(true),
            music_library_path: RefCell::new(None),
            library_updated_sender,
            database: RefCell::new(None),
            player: RefCell::new(None),
            client: Client::new(),
        }
    }

    /// Wait for the next state change. Initially, the state should be assumed to be
    /// BackendState::Loading. Changes should be awaited before calling init().
    pub async fn next_state(&self) -> Result<BackendState> {
        Ok(self.state_sender.subscribe().recv().await?)
    }

    /// Initialize the backend updating the state accordingly.
    pub async fn init(&self) -> Result<()> {
        self.init_library().await?;

        if let Some(url) = self.settings.get_string("server-url") {
            if !url.is_empty() {
                self.client.set_server_url(&url);
            }
        }

        #[cfg(all(feature = "dbus"))]
        match Self::load_login_data().await {
            Ok(Some(data)) => self.client.set_login_data(Some(data)),
            Err(err) => warn!(
                "The login data could not be loaded from SecretService. It will not \
                be available. Error message: {}",
                err
            ),
            _ => (),
        }

        if self.get_music_library_path().is_none() {
            self.set_state(BackendState::NoMusicLibrary);
        } else {
            self.set_state(BackendState::Ready);
        }

        Ok(())
    }

    /// Whether the server should be used by default.
    pub fn use_server(&self) -> bool {
        self.use_server.get()
    }

    /// Set whether the server should be used by default.
    pub fn set_use_server(&self, enabled: bool) {
        self.use_server.set(enabled);
    }

    /// Set the URL of the Musicus server to connect to.
    pub fn set_server_url(&self, url: &str) {
        if let Err(err) = self.settings.set_string("server-url", url) {
            warn!(
                "An error happened while trying to save the server URL to GSettings. Most \
                likely it will not be available at the next startup. Error message: {}",
                err
            );
        }

        self.client.set_server_url(url);
    }

    /// Get the currently set server URL.
    pub fn get_server_url(&self) -> Option<String> {
        self.client.get_server_url()
    }

    /// Set the user credentials to use.
    pub async fn set_login_data(&self, data: Option<LoginData>) {
        #[cfg(all(feature = "dbus"))]
        if let Some(data) = &data {
            if let Err(err) = Self::store_login_data(data.clone()).await {
                warn!(
                    "An error happened while trying to store the login data using SecretService. \
                    This means, that they will not be available at the next startup most likely. \
                    Error message: {}",
                    err
                );
            }
        } else {
            if let Err(err) = Self::delete_secrets().await {
                warn!(
                    "An error happened while trying to delete the login data from SecretService. \
                    This may result in the login data being reloaded at the next startup. Error \
                    message: {}",
                    err
                );
            }
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
        self.state_sender.send(state).unwrap();
    }
}
