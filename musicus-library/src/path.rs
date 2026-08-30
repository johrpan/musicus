//! Library-relative track paths.
//!
//! The supported, storage-independent value type for a track's audio file
//! path. This module defines the domain value and its validation only; the
//! database serialization lives in the private database layer.

use std::path::{Path, PathBuf};

use crate::error::LibraryError;

/// A validated, library-relative path to a track's audio file.
///
/// Stored as a portable, slash-separated relative path. Values are validated
/// on construction and never escape the library root: absolute paths, root
/// components, drive prefixes, `.`/`..` and empty components are rejected.
/// Database serialization is implemented entirely in the private persistence
/// layer.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LibraryTrackPath(String);

impl LibraryTrackPath {
    /// Parse and validate a stored relative path, using '/' as the separator.
    pub fn try_from_str(path: &str) -> crate::Result<Self> {
        Self::try_from_components(path.split('/').collect())
    }

    fn try_from_components(components: Vec<&str>) -> crate::Result<Self> {
        if components.is_empty() {
            return Err(LibraryError::InvalidInput(
                "Track paths must not be empty".to_owned(),
            ));
        }

        for component in &components {
            if component.is_empty() {
                return Err(LibraryError::InvalidInput(format!(
                    "Track path {components:?} contains an empty component"
                )));
            }

            if *component == "." || *component == ".." {
                return Err(LibraryError::InvalidInput(format!(
                    "Track path {components:?} contains a \".\" or \"..\" component"
                )));
            }

            if component.contains('\\') {
                return Err(LibraryError::InvalidInput(format!(
                    "Track path {components:?} contains a backslash"
                )));
            }
        }

        Ok(Self(components.join("/")))
    }

    /// The stored path, in portable slash-separated form.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The file name of this track, if any.
    pub fn file_name(&self) -> Option<&str> {
        self.0.rsplit('/').next().filter(|name| !name.is_empty())
    }

    /// The path relative to the library root that a ZIP entry uses.
    pub(crate) fn to_zip_name(&self) -> &str {
        &self.0
    }

    /// Resolve this track path against the library root.
    pub(crate) fn resolve(&self, root: &Path) -> PathBuf {
        root.join(&self.0)
    }
}

impl AsRef<Path> for LibraryTrackPath {
    fn as_ref(&self) -> &Path {
        Path::new(&self.0)
    }
}

impl std::fmt::Display for LibraryTrackPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
