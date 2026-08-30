//! Shared entity persistence policy.
//!
//! Every supported mutation agrees on how timestamps are maintained, what a
//! zero-row update means, how foreign-key failures translate into structured
//! errors, and when observers are notified. The helpers in this module are the
//! single place that policy is defined, so that individual CRUD operations do
//! not repeat it.

use chrono::NaiveDateTime;
use diesel::SqliteConnection;

use crate::{
    db,
    error::{EntityKind, LibraryError, Result},
    library::Library,
};

/// The timestamps of a freshly created entity.
///
/// `created_at`, `edited_at` and `last_used_at` all start at the same moment:
/// an entity is created at the same time it is first edited and first used.
pub(super) fn fresh_timestamps() -> (NaiveDateTime, NaiveDateTime, NaiveDateTime) {
    let now = db::now();
    (now, now, now)
}

/// Update the one entity `kind` expects, failing with `NotFound` if the update
/// matches no row.
///
/// `build_update` receives the connection and the fresh `edited_at` timestamp
/// and returns the number of rows it changed; every supported update sets both
/// `edited_at` and `last_used_at` from that single timestamp.
pub(super) fn update_entity<F>(
    connection: &mut SqliteConnection,
    kind: EntityKind,
    id: &str,
    build_update: F,
) -> Result<()>
where
    F: FnOnce(&mut SqliteConnection, NaiveDateTime) -> diesel::QueryResult<usize>,
{
    let edited_at = db::now();
    let updated = build_update(connection, edited_at)?;

    if updated == 0 {
        return Err(LibraryError::NotFound(kind, id.to_owned()).into());
    }

    Ok(())
}

/// Delete the one entity `kind` expects, translating foreign-key failures and
/// failing with `NotFound` if nothing matched.
pub(super) fn delete_entity<F>(
    connection: &mut SqliteConnection,
    kind: EntityKind,
    id: &str,
    delete: F,
) -> Result<()>
where
    F: FnOnce(&mut SqliteConnection) -> diesel::QueryResult<usize>,
{
    let deleted = delete(connection).map_err(|error| LibraryError::from_delete(kind, error))?;

    if deleted == 0 {
        return Err(LibraryError::NotFound(kind, id.to_owned()).into());
    }

    Ok(())
}

impl Library {
    /// Perform a mutation and notify observers exactly once after it succeeds.
    ///
    /// Callers must not call [`Library::changed`] themselves. Read-only
    /// operations, failed mutations and no-op operations never notify through
    /// this helper; operations that may legitimately change nothing notify
    /// only when something actually changed.
    pub(crate) fn commit<T>(
        &self,
        operation: impl FnOnce(&mut SqliteConnection) -> Result<T>,
    ) -> Result<T> {
        let result = operation(&mut *self.conn());

        if result.is_ok() {
            self.changed();
        }

        result
    }
}
