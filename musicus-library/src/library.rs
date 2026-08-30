use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
    time::SystemTime,
};

use diesel::{prelude::*, SqliteConnection};

use crate::{
    db::{self, schema::*, tables},
    error::Result,
};

/// Send a change notification to every subscriber, coalescing per subscriber:
/// a subscriber with a pending message is not sent another one, and a
/// subscriber whose receiver was dropped is removed.
fn notify_changed(senders: &Mutex<Vec<async_channel::Sender<()>>>) {
    senders
        .lock()
        .unwrap()
        .retain(|sender| match sender.try_send(()) {
            Ok(()) => true,
            Err(async_channel::TrySendError::Full(())) => true,
            Err(async_channel::TrySendError::Closed(())) => false,
        });
}

pub mod edit;
pub mod exchange;
mod import_graph;
pub mod list;
mod load;
pub mod merge;
mod metadata;
pub mod program;
pub mod query;
mod refresh;
pub mod search;

/// A music library backed by a SQLite database in a given folder.
pub struct Library {
    folder: PathBuf,
    connection: Arc<Mutex<SqliteConnection>>,

    /// The current metadata database connection including its database files'
    /// modification time as the cache key.
    metadata_connection: RefCell<Option<(Option<SystemTime>, Arc<Mutex<SqliteConnection>>)>>,

    /// Directory for cache files.
    cache_dir: PathBuf,

    /// Change notification senders, one per subscriber.
    ///
    /// Shared through an [`Arc`] so that synchronous exchange operations on
    /// worker threads can notify through it too; every channel is bounded with
    /// capacity one, so a slow subscriber never accumulates messages.
    changed_senders: Arc<Mutex<Vec<async_channel::Sender<()>>>>,
}

impl Library {
    /// Open (and if necessary create/migrate) the library database in `path`.
    ///
    /// `cache_dir` is used for the metadata database file.
    pub fn new(path: impl AsRef<Path>, cache_dir: impl Into<PathBuf>) -> Result<Self> {
        let folder = path.as_ref().to_owned();
        let db_path = folder.join("musicus.musdb");
        let connection = db::connect(&db_path)?;

        Ok(Self {
            folder,
            connection: Arc::new(Mutex::new(connection)),
            metadata_connection: RefCell::new(None),
            cache_dir: cache_dir.into(),
            changed_senders: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// The folder the library's database and track files live in.
    pub fn folder(&self) -> &Path {
        &self.folder
    }

    /// Resolve a validated library-relative track path to an absolute path.
    pub fn track_file_path(&self, path: &crate::path::LibraryTrackPath) -> PathBuf {
        path.resolve(&self.folder)
    }

    fn conn(&self) -> MutexGuard<'_, SqliteConnection> {
        db::lock_connection(&self.connection)
    }

    /// Subscribe to change notifications. A message is sent on the returned
    /// receiver every time library data changes.
    ///
    /// The channel is bounded with capacity one: at most one pending change
    /// message exists per subscriber, and a slow subscriber cannot cause
    /// unbounded memory growth.
    pub fn subscribe_changed(&self) -> async_channel::Receiver<()> {
        let (sender, receiver) = async_channel::bounded(1);
        self.changed_senders.lock().unwrap().push(sender);
        receiver
    }

    /// Notify all receivers that the library has changed.
    ///
    /// Private: every notification originates from a committed library
    /// operation, never from the application.
    fn changed(&self) {
        notify_changed(&self.changed_senders);
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
