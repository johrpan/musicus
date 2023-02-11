use crate::{Backend, BackendState, Player, Result};
use gio::prelude::*;
use log::warn;
use musicus_database::SqliteConnection;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

impl Backend {
    /// Initialize the music library if it is set in the settings.
    pub(super) fn init_library(self: Rc<Self>) -> Result<()> {
        let path = self.settings.string("music-library-path");
        if !path.is_empty() {
            self.set_music_library_path_priv(PathBuf::from(path.to_string()))?;
        }

        Ok(())
    }

    /// Set the path to the music library folder and connect to the database.
    pub fn set_music_library_path(self: Rc<Self>, path: PathBuf) -> Result<()> {
        if let Err(err) = self
            .settings
            .set_string("music-library-path", path.to_str().unwrap())
        {
            warn!(
                "The music library path could not be saved using GSettings. It will most likely \
                not be available at the next startup. Error message: {}",
                err
            );
        }

        self.set_music_library_path_priv(path)
    }

    /// Set the path to the music library folder and and connect to the database.
    pub fn set_music_library_path_priv(self: Rc<Self>, path: PathBuf) -> Result<()> {
        self.set_state(BackendState::Loading);

        self.music_library_path.replace(Some(path.clone()));

        let mut db_path = path.clone();
        db_path.push("musicus.db");

        let database = Arc::new(Mutex::new(musicus_database::connect(
            db_path.to_str().unwrap(),
        )?));

        self.database.replace(Some(database));

        let player = Player::new(path);
        self.player.replace(Some(player));

        Rc::clone(&self).update_track_generator();

        self.set_state(BackendState::Ready);

        Ok(())
    }

    /// Get the currently set music library path.
    pub fn get_music_library_path(&self) -> Option<PathBuf> {
        self.music_library_path.borrow().clone()
    }

    /// Get an interface to the database and panic if there is none.
    pub fn db(&self) -> Arc<Mutex<SqliteConnection>> {
        self.database.borrow().clone().unwrap()
    }

    /// Get an interface to the playback service.
    pub fn get_player(&self) -> Option<Rc<Player>> {
        self.player.borrow().clone()
    }

    /// Wait for the next library update.
    pub async fn library_update(&self) -> Result<()> {
        Ok(self.library_updated_sender.subscribe().recv().await?)
    }

    /// Notify the frontend that the library was changed.
    pub fn library_changed(&self) {
        self.library_updated_sender.send(()).unwrap();
    }

    /// Get an interface to the player and panic if there is none.
    pub fn pl(&self) -> Rc<Player> {
        self.get_player().unwrap()
    }
}
