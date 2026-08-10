use std::fmt::{self, Display};

use diesel::result::{DatabaseErrorKind, Error as DieselError};

/// The kind of thing an error is about, so that a message can name it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntityKind {
    Person,
    Role,
    Instrument,
    Work,
    Ensemble,
    Recording,
    Album,
    Medium,
    Track,
}

impl Display for EntityKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            EntityKind::Person => "person",
            EntityKind::Role => "role",
            EntityKind::Instrument => "instrument",
            EntityKind::Work => "work",
            EntityKind::Ensemble => "ensemble",
            EntityKind::Recording => "recording",
            EntityKind::Album => "album",
            EntityKind::Medium => "medium",
            EntityKind::Track => "track",
        };

        f.write_str(name)
    }
}

/// Whether a Diesel error is SQLite refusing to break a foreign key.
///
/// Diesel's SQLite backend does not classify these as
/// [`DatabaseErrorKind::ForeignKeyViolation`] — they arrive as an unspecified
/// database error — so the message has to be checked as well. The kind is still
/// matched first, so this keeps working if Diesel starts classifying them.
fn is_foreign_key_violation(error: &DieselError) -> bool {
    match error {
        DieselError::DatabaseError(DatabaseErrorKind::ForeignKeyViolation, _) => true,
        DieselError::DatabaseError(_, info) => info.message().contains("FOREIGN KEY constraint"),
        _ => false,
    }
}

#[derive(Debug)]
pub enum LibraryError {
    /// The item cannot be deleted because something else still refers to it.
    ///
    /// Deletion relies on the database's foreign keys to refuse this, so
    /// without this variant the user was shown a raw Diesel error string.
    StillReferenced(EntityKind),

    /// The database was written by a newer version of Musicus.
    SchemaTooNew {
        found: i32,
        supported: i32,
    },

    Other(anyhow::Error),
}

impl LibraryError {
    /// Interpret a Diesel error from deleting `kind`.
    pub(crate) fn from_delete(kind: EntityKind, error: DieselError) -> Self {
        if is_foreign_key_violation(&error) {
            LibraryError::StillReferenced(kind)
        } else {
            LibraryError::Other(error.into())
        }
    }
}

impl Display for LibraryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LibraryError::StillReferenced(kind) => {
                write!(f, "This {kind} is still used elsewhere in the library.")
            }
            LibraryError::SchemaTooNew { found, supported } => write!(
                f,
                "This library was created by a newer version of Musicus \
                 (library schema version {found}, this version supports {supported}). \
                 Please update Musicus to open it."
            ),
            LibraryError::Other(err) => Display::fmt(err, f),
        }
    }
}

impl std::error::Error for LibraryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
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
        LibraryError::Other(err.into())
    }
}

pub type Result<T, E = LibraryError> = std::result::Result<T, E>;
