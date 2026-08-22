pub mod db;
pub mod error;
pub mod library;

pub use error::{EntityKind, LibraryError};
pub use library::Library;

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
