pub mod db;
pub mod error;
pub mod library;

pub use error::{EntityKind, LibraryError};
pub use library::Library;

/// Interpolate arguments into an already translated format string.
///
/// A translation is data supplied by translators, so its placeholders may not
/// match the arguments the code passes. `formatx!(...).unwrap()` turns that into
/// a panic, which means a single bad entry in a `.po` file can crash the app —
/// including on the search hot path. This falls back to showing the
/// uninterpolated translation instead.
///
/// The message id stays inside a `gettext` call at the call site so that
/// `xgettext` keeps finding it:
///
/// ```ignore
/// format_translated!(gettext("Music for {}"), instrument.name.get())
/// ```
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
