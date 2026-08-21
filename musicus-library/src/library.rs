use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
    time::SystemTime,
};

use anyhow::{anyhow, Context, Result};
use diesel::{prelude::*, SqliteConnection};

use crate::db::{self, schema::*, tables};
pub use edit::TrackUpdate;
pub use generate::GenerateRecordingParams;
pub use manage::{EnsembleListItem, RecordingListItem, WorkListItem};
pub use merge::EntityUsage;
pub use metadata::SearchItem;
pub use pattern::Patterns;
pub use query::{Facet, LibraryQuery};
pub mod audio_tags;
pub mod edit;
pub mod exchange;
pub mod filenames;
pub mod generate;
pub mod manage;
pub mod merge;
pub mod metadata;
pub mod pattern;
pub mod query;
pub mod reorganize;

/// An open metadata database remembered together with the modification time of
/// the file it came from.
///
/// Downloading a new metadata database replaces that file, which invalidates the connection.
type CachedMetadataConnection = (Option<SystemTime>, Arc<Mutex<SqliteConnection>>);

/// A music library backed by a SQLite database in a given folder.
pub struct Library {
    folder: String,
    connection: Arc<Mutex<SqliteConnection>>,
    metadata_connection: RefCell<Option<CachedMetadataConnection>>,
    metadata_cache_dir: PathBuf,
    changed_senders: RefCell<Vec<async_channel::Sender<()>>>,
    patterns: RefCell<Patterns>,
}

impl Library {
    /// Open (and if necessary create/migrate) the library database in `path`.
    ///
    /// `metadata_cache_dir` is the directory used to cache the downloaded metadata
    /// database (see [`exchange`]).
    pub fn new(path: impl AsRef<Path>, metadata_cache_dir: impl Into<PathBuf>) -> Result<Self> {
        let folder = path
            .as_ref()
            .to_str()
            .ok_or_else(|| anyhow!("Failed to convert library path to string"))?
            .to_owned();

        let db_path = PathBuf::from(&folder).join("musicus.db");
        let connection = db::connect(
            db_path
                .to_str()
                .ok_or_else(|| anyhow!("Failed to convert library path to string"))?,
        )
        .context("Failed to connect to music library database")?;

        Ok(Self {
            folder,
            connection: Arc::new(Mutex::new(connection)),
            metadata_connection: RefCell::new(None),
            metadata_cache_dir: metadata_cache_dir.into(),
            changed_senders: RefCell::new(Vec::new()),
            patterns: RefCell::new(Patterns::default()),
        })
    }

    pub fn folder(&self) -> &str {
        &self.folder
    }

    /// The patterns used to name and tag the files of newly imported tracks.
    pub fn patterns(&self) -> Patterns {
        self.patterns.borrow().clone()
    }

    /// Set the patterns used to name and tag the files of newly imported
    /// tracks.
    pub fn set_patterns(&self, patterns: &Patterns) {
        self.patterns.replace(patterns.clone());
    }

    /// The pattern used to name the files of newly imported tracks.
    pub fn filename_pattern(&self) -> String {
        self.patterns.borrow().filename.clone()
    }

    /// Set the pattern used to name the files of newly imported tracks.
    pub fn set_filename_pattern(&self, pattern: &str) {
        self.patterns.borrow_mut().filename = pattern.to_owned();
    }

    fn conn(&self) -> MutexGuard<'_, SqliteConnection> {
        db::lock_connection(&self.connection)
    }

    /// Subscribe to change notifications. A message is sent on the returned receiver
    /// every time library data changes.
    pub fn subscribe_changed(&self) -> async_channel::Receiver<()> {
        let (sender, receiver) = async_channel::unbounded();
        self.changed_senders.borrow_mut().push(sender);
        receiver
    }

    pub fn changed(&self) {
        self.changed_senders
            .borrow_mut()
            .retain(|sender| sender.try_send(()).is_ok());
    }

    /// Whether this library is empty. The library is considered empty, if
    /// there are no tracks.
    pub fn is_empty(&self) -> Result<bool> {
        let connection = &mut *self.conn();
        Ok(tracks::table
            .first::<tables::Track>(connection)
            .optional()?
            .is_none())
    }
}
