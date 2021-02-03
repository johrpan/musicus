use super::{Backend, BackendState, DbThread, Player};
use anyhow::Result;
use gio::prelude::*;
use std::path::PathBuf;
use std::rc::Rc;

impl Backend {
    /// Initialize the music library if it is set in the settings.
    pub(super) async fn init_library(&self) -> Result<()> {
        if let Some(path) = self.settings.get_string("music-library-path") {
            if !path.is_empty() {
                self.set_music_library_path_priv(PathBuf::from(path.to_string()))
                    .await?;
            }
        }

        Ok(())
    }

    /// Set the path to the music library folder and start a database thread in the background.
    pub async fn set_music_library_path(&self, path: PathBuf) -> Result<()> {
        self.settings
            .set_string("music-library-path", path.to_str().unwrap())?;
        self.set_music_library_path_priv(path).await
    }

    /// Set the path to the music library folder and start a database thread in the background.
    pub async fn set_music_library_path_priv(&self, path: PathBuf) -> Result<()> {
        self.set_state(BackendState::Loading);

        if let Some(db) = &*self.database.borrow() {
            db.stop().await?;
        }

        self.music_library_path.replace(Some(path.clone()));

        let mut db_path = path.clone();
        db_path.push("musicus.db");

        let database = DbThread::new(db_path.to_str().unwrap().to_string()).await?;
        self.database.replace(Some(Rc::new(database)));

        let player = Player::new(path);
        self.player.replace(Some(player));

        self.set_state(BackendState::Ready);

        Ok(())
    }

    /// Get the currently set music library path.
    pub fn get_music_library_path(&self) -> Option<PathBuf> {
        self.music_library_path.borrow().clone()
    }

    /// Get an interface to the current music library database.
    pub fn get_database(&self) -> Option<Rc<DbThread>> {
        self.database.borrow().clone()
    }

    /// Get an interface to the database and panic if there is none.
    pub fn db(&self) -> Rc<DbThread> {
        self.get_database().unwrap()
    }

    /// Get an interface to the playback service.
    pub fn get_player(&self) -> Option<Rc<Player>> {
        self.player.borrow().clone()
    }

    /// Notify the frontend that the library was changed.
    pub fn library_changed(&self) {
        self.set_state(BackendState::Loading);
        self.set_state(BackendState::Ready);
    }

    /// Get an interface to the player and panic if there is none.
    pub fn pl(&self) -> Rc<Player> {
        self.get_player().unwrap()
    }
}
