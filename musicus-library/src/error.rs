use std::fmt::{self, Display};

use diesel::result::{DatabaseErrorKind, Error as DieselError};

/// The kind of thing an error is about, so that a message can name it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntityKind {
    Person,
    Role,
    Instrument,
    Tag,
    Work,
    Ensemble,
    Recording,
    Album,
    Track,
}

impl Display for EntityKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            EntityKind::Person => "person",
            EntityKind::Role => "role",
            EntityKind::Instrument => "instrument",
            EntityKind::Tag => "tag",
            EntityKind::Work => "work",
            EntityKind::Ensemble => "ensemble",
            EntityKind::Recording => "recording",
            EntityKind::Album => "album",
            EntityKind::Track => "track",
        };

        f.write_str(name)
    }
}

/// Whether a Diesel error is a foreign key violation.
fn is_foreign_key_violation(error: &DieselError) -> bool {
    match error {
        DieselError::DatabaseError(DatabaseErrorKind::ForeignKeyViolation, _) => true,
        DieselError::DatabaseError(_, info) => info.message().contains("FOREIGN KEY constraint"),
        _ => false,
    }
}

/// The structured error every supported library operation returns.
///
/// Callers can distinguish expected conditions (an entity that no longer
/// exists, a still-referenced deletion, an import that would disturb existing
/// data, a cancellation) without ever parsing error text. Source errors are
/// kept for logging and diagnosis.
#[derive(Debug)]
pub enum LibraryError {
    /// The item cannot be deleted because something else still refers to it.
    StillReferenced(EntityKind),

    /// The entity does not exist.
    NotFound(EntityKind, String),

    /// The input or the data violates an invariant.
    InvalidInput(String),

    /// A file path cannot be used safely (absolute, outside the library,
    /// non-Unicode where SQLite requires it, ...).
    InvalidPath(String),

    /// An import would have to change a local entity to proceed.
    ImportConflict(String),

    /// The database or an import graph is inconsistent.
    Corrupt(String),

    /// The database was written by a newer version of Musicus.
    SchemaTooNew { found: i32, supported: i32 },

    /// The operation was cancelled by the caller.
    Cancelled,

    /// An archive or other serialized data could not be read or written.
    Archive(String),

    /// A file system operation failed.
    Io(std::io::Error),

    /// A database operation failed.
    Database(DieselError),

    /// Anything not covered above; kept for logging, not for matching.
    Other(anyhow::Error),
}

impl LibraryError {
    /// Interpret a Diesel error from deleting `kind`.
    pub(crate) fn from_delete(kind: EntityKind, error: DieselError) -> Self {
        if is_foreign_key_violation(&error) {
            LibraryError::StillReferenced(kind)
        } else {
            LibraryError::Database(error)
        }
    }
}

impl Display for LibraryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LibraryError::StillReferenced(kind) => {
                write!(f, "This {kind} is still used elsewhere in the library.")
            }
            LibraryError::NotFound(kind, id) => {
                write!(f, "No {kind} with ID {id} exists.")
            }
            LibraryError::InvalidInput(message) => write!(f, "{message}"),
            LibraryError::InvalidPath(message) => write!(f, "{message}"),
            LibraryError::ImportConflict(message) => {
                write!(f, "The import conflicts with existing data: {message}")
            }
            LibraryError::Corrupt(message) => write!(f, "{message}"),
            LibraryError::SchemaTooNew { found, supported } => write!(
                f,
                "This library was created by a newer version of Musicus \
                 (library schema version {found}, this version supports {supported}). \
                 Please update Musicus to open it."
            ),
            LibraryError::Cancelled => write!(f, "The operation was cancelled"),
            LibraryError::Archive(message) => write!(f, "{message}"),
            LibraryError::Io(err) => Display::fmt(err, f),
            LibraryError::Database(err) => Display::fmt(err, f),
            LibraryError::Other(err) => Display::fmt(err, f),
        }
    }
}

impl std::error::Error for LibraryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LibraryError::Io(err) => Some(err),
            LibraryError::Database(err) => Some(err),
            LibraryError::Other(err) => err.source(),
            _ => None,
        }
    }
}

impl From<anyhow::Error> for LibraryError {
    fn from(err: anyhow::Error) -> Self {
        LibraryError::Other(err)
    }
}

impl From<DieselError> for LibraryError {
    fn from(err: DieselError) -> Self {
        if is_foreign_key_violation(&err) {
            // A foreign key that fires outside a delete usually means the
            // input refers to an entity that does not exist; callers with a
            // matching entity kind context map it more precisely.
            LibraryError::InvalidInput(format!("A referenced entity is missing: {err}"))
        } else {
            LibraryError::Database(err)
        }
    }
}

impl From<std::io::Error> for LibraryError {
    fn from(err: std::io::Error) -> Self {
        LibraryError::Io(err)
    }
}

impl From<zip::result::ZipError> for LibraryError {
    fn from(err: zip::result::ZipError) -> Self {
        LibraryError::Archive(err.to_string())
    }
}

impl From<serde_json::Error> for LibraryError {
    fn from(err: serde_json::Error) -> Self {
        LibraryError::Archive(err.to_string())
    }
}

pub type Result<T, E = LibraryError> = std::result::Result<T, E>;
