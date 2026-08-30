mod db;
pub mod error;
pub mod input;
mod library;
pub mod models;
pub mod path;

pub use db::tables::Source;
pub use db::TranslatedString;
pub use db::{generate_id, set_language};
pub use error::{EntityKind, LibraryError, Result};
pub use library::edit;
pub use library::exchange;
pub use library::list;
pub use library::merge;
pub use library::program;
pub use library::query;
pub use library::search;
pub use library::Library;
pub use path::LibraryTrackPath;

/// Explicitly unstable persistence facilities.
///
/// Normal applications do not use this module; the metadata catalogue tool
/// (`musicus-data`) builds databases from it, and advanced tooling may need
/// the raw rows or the schema version. Everything here may change between
/// releases without notice.
pub mod unstable {
    /// Open (and migrate) a Musicus database file.
    pub use crate::db::connect;

    /// The schema version this build reads and writes.
    pub use crate::db::SCHEMA_VERSION;

    /// The generated Diesel table definitions.
    pub use crate::db::schema;

    /// Raw table row structs, one per table.
    pub use crate::db::tables;

    /// The derived `*_last_played` views.
    pub use crate::db::views;
}

/// Interpolate arguments into an already translated format string, falling
/// back to the untranslated string in case of errors.
///
/// This is meant to wrap [`gettext()`].
#[macro_export]
macro_rules! format_translated {
    ($translated:expr $(, $arg:expr)* $(,)?) => {{
        let template: String = $translated;

        match ::formatx::formatx!(template.clone() $(, $arg)*) {
            Ok(formatted) => formatted,
            Err(err) => {
                ::log::warn!("Malformed format string {template:?}: {err}");
                template
            }
        }
    }};
}
