pub mod catalog;
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
pub use library::catalog_export;
pub use library::catalog_import;
pub use library::edit;
pub use library::exchange;
pub use library::list;
pub use library::merge;
pub use library::program;
pub use library::query;
pub use library::search;
pub use library::Library;
pub use path::LibraryTrackPath;

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
