//! This module contains structs that are one-to-one representations of the
//! tables in the database schema.

use std::path::{Path, PathBuf};

use super::{schema::*, TranslatedString};
use crate::error::LibraryError;
use anyhow::{anyhow, Result};
use chrono::NaiveDateTime;
use diesel::{
    backend::Backend,
    deserialize::{FromSql, FromSqlRow},
    expression::AsExpression,
    prelude::*,
    serialize::{IsNull, Output, ToSql},
    sql_types::Text,
    sqlite::Sqlite,
};

#[derive(Insertable, Queryable, Selectable, Clone, Debug)]
#[diesel(check_for_backend(Sqlite))]
pub struct Person {
    pub person_id: String,
    pub name: TranslatedString,
    pub source: Source,
    pub enable_updates: bool,
    pub created_at: NaiveDateTime,
    pub edited_at: NaiveDateTime,
    pub last_used_at: NaiveDateTime,
}

#[derive(Insertable, Queryable, Selectable, Clone, Debug)]
#[diesel(check_for_backend(Sqlite))]
pub struct Role {
    pub role_id: String,
    pub name: TranslatedString,
    pub source: Source,
    pub enable_updates: bool,
    pub created_at: NaiveDateTime,
    pub edited_at: NaiveDateTime,
    pub last_used_at: NaiveDateTime,
}

#[derive(Insertable, Queryable, Selectable, Clone, Debug)]
#[diesel(check_for_backend(Sqlite))]
pub struct Instrument {
    pub instrument_id: String,
    pub name: TranslatedString,
    pub source: Source,
    pub enable_updates: bool,
    pub created_at: NaiveDateTime,
    pub edited_at: NaiveDateTime,
    pub last_used_at: NaiveDateTime,
}

#[derive(Insertable, Queryable, Selectable, Clone, Debug)]
#[diesel(check_for_backend(Sqlite))]
pub struct Tag {
    pub tag_id: String,
    pub name: TranslatedString,
    pub takes_value: bool,
    pub source: Source,
    pub enable_updates: bool,
    pub created_at: NaiveDateTime,
    pub edited_at: NaiveDateTime,
    pub last_used_at: NaiveDateTime,
    pub private: bool,
}

#[derive(Insertable, Queryable, Selectable, Clone, Debug)]
#[diesel(check_for_backend(Sqlite))]
pub struct WorkTag {
    pub work_id: String,
    pub tag_id: String,
    pub value: Option<TranslatedString>,
    pub sequence_number: i32,
}

#[derive(Insertable, Queryable, Selectable, Clone, Debug)]
#[diesel(check_for_backend(Sqlite))]
pub struct RecordingTag {
    pub recording_id: String,
    pub tag_id: String,
    pub value: Option<TranslatedString>,
    pub sequence_number: i32,
}

#[derive(Insertable, Queryable, Selectable, Clone, Debug)]
#[diesel(check_for_backend(Sqlite))]
pub struct Work {
    pub work_id: String,
    pub parent_work_id: Option<String>,
    pub sequence_number: Option<i32>,
    pub name: TranslatedString,
    pub source: Source,
    pub enable_updates: bool,
    pub created_at: NaiveDateTime,
    pub edited_at: NaiveDateTime,
    pub last_used_at: NaiveDateTime,
}

#[derive(Insertable, Queryable, Selectable, Clone, Debug)]
#[diesel(check_for_backend(Sqlite))]
pub struct WorkRelation {
    pub work_id: String,
    pub related_work_id: String,
}

#[derive(Insertable, Queryable, Selectable, Clone, Debug)]
#[diesel(check_for_backend(Sqlite))]
pub struct WorkPerson {
    pub work_id: String,
    pub person_id: String,
    pub role_id: Option<String>,
    pub sequence_number: i32,
}

#[derive(Insertable, Queryable, Selectable, Clone, Debug)]
#[diesel(check_for_backend(Sqlite))]
pub struct WorkInstrument {
    pub work_id: String,
    pub instrument_id: String,
    pub sequence_number: i32,
}

#[derive(Insertable, Queryable, Selectable, Clone, Debug)]
#[diesel(check_for_backend(Sqlite))]
pub struct Ensemble {
    pub ensemble_id: String,
    pub name: TranslatedString,
    pub source: Source,
    pub enable_updates: bool,
    pub created_at: NaiveDateTime,
    pub edited_at: NaiveDateTime,
    pub last_used_at: NaiveDateTime,
}

#[derive(Insertable, Queryable, Selectable, Clone, Debug)]
#[diesel(check_for_backend(Sqlite))]
pub struct EnsemblePerson {
    pub ensemble_id: String,
    pub person_id: String,
    pub instrument_id: Option<String>,
    pub sequence_number: i32,
    pub role_id: Option<String>,
}

#[derive(Insertable, Queryable, Selectable, Clone, Debug)]
#[diesel(check_for_backend(Sqlite))]
pub struct Recording {
    pub recording_id: String,
    pub work_id: String,
    pub source: Source,
    pub enable_updates: bool,
    pub created_at: NaiveDateTime,
    pub edited_at: NaiveDateTime,
    pub last_used_at: NaiveDateTime,
    pub comment: Option<String>,
}

#[derive(Insertable, Queryable, Selectable, Clone, Debug)]
#[diesel(check_for_backend(Sqlite))]
pub struct RecordingPerson {
    pub recording_id: String,
    pub person_id: String,
    pub role_id: Option<String>,
    pub instrument_id: Option<String>,
    pub sequence_number: i32,
}

#[derive(Insertable, Queryable, Selectable, Clone, Debug)]
#[diesel(check_for_backend(Sqlite))]
pub struct RecordingEnsemble {
    pub recording_id: String,
    pub ensemble_id: String,
    pub role_id: Option<String>,
    pub sequence_number: i32,
}

#[derive(Insertable, Queryable, Selectable, Clone, Debug)]
#[diesel(check_for_backend(Sqlite))]
pub struct Track {
    pub track_id: String,
    pub recording_id: String,
    pub recording_index: i32,
    pub path: LibraryTrackPath,
    pub created_at: NaiveDateTime,
    pub edited_at: NaiveDateTime,
    pub last_used_at: NaiveDateTime,
}

#[derive(Insertable, Queryable, Selectable, Clone, Debug)]
#[diesel(check_for_backend(Sqlite))]
pub struct Play {
    pub play_id: String,
    pub track_id: Option<String>,
    pub recording_id: String,
    pub played_at: NaiveDateTime,
}

#[derive(Insertable, Queryable, Selectable, Clone, Debug)]
#[diesel(check_for_backend(Sqlite))]
pub struct TrackWork {
    pub track_id: String,
    pub work_id: String,
    pub sequence_number: i32,
}

#[derive(Insertable, Queryable, Selectable, Clone, Debug)]
#[diesel(check_for_backend(Sqlite))]
pub struct Album {
    pub album_id: String,
    pub name: TranslatedString,
    pub source: Source,
    pub enable_updates: bool,
    pub created_at: NaiveDateTime,
    pub edited_at: NaiveDateTime,
    pub last_used_at: NaiveDateTime,
}

#[derive(Insertable, Queryable, Selectable, Clone, Debug)]
#[diesel(check_for_backend(Sqlite))]
pub struct AlbumRecording {
    pub album_id: String,
    pub recording_id: String,
    pub sequence_number: i32,
}

/// The relative subtree under the library root that managed track files live in.
pub const TRACKS_SUBDIR: &str = "tracks";

/// A validated, library-relative path to a track's audio file.
///
/// Stored as a portable, slash-separated relative path. Values are validated
/// on read and never escape the library root: absolute paths, root
/// components, drive prefixes, `.`/`..` and empty components are rejected.
/// Newly imported files live under [`TRACKS_SUBDIR`]; legacy values outside it
/// stay readable until the schema migration rewrites them.
#[derive(AsExpression, FromSqlRow, Clone, Debug, PartialEq, Eq, Hash)]
#[diesel(sql_type = Text)]
pub struct LibraryTrackPath(String);

impl LibraryTrackPath {
    /// Parse and validate a stored relative path, using '/' as the separator.
    pub fn try_from_str(path: &str) -> Result<Self, LibraryError> {
        Self::try_from_components(path.split('/').collect(), false)
    }

    /// Build a validated path inside the managed [`TRACKS_SUBDIR`] subtree.
    pub fn managed(components: &[&str]) -> Result<Self, LibraryError> {
        Self::try_from_components(components.to_vec(), true)
    }

    fn try_from_components(
        components: Vec<&str>,
        require_managed: bool,
    ) -> Result<Self, LibraryError> {
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

        if require_managed && components[0] != TRACKS_SUBDIR {
            return Err(LibraryError::InvalidInput(format!(
                "Track path must be inside the {} directory",
                TRACKS_SUBDIR
            )));
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
    pub fn to_zip_name(&self) -> &str {
        &self.0
    }

    /// Resolve this track path against the library root.
    pub fn resolve(&self, root: &Path) -> PathBuf {
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

impl ToSql<Text, Sqlite> for LibraryTrackPath
where
    String: ToSql<Text, Sqlite>,
{
    fn to_sql(&self, out: &mut Output<Sqlite>) -> diesel::serialize::Result {
        out.set_value(self.0.clone());
        Ok(IsNull::No)
    }
}

impl<DB> FromSql<Text, DB> for LibraryTrackPath
where
    DB: Backend,
    String: FromSql<Text, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        let text = String::from_sql(bytes)?;

        // Legacy rows store a serialized JSON array of host path components
        // (e.g. `["recording_00.mp3"]`) instead of the portable form. Convert
        // them in memory; the schema migration rewrites the stored values.
        if text.trim_start().starts_with('[') {
            let components = serde_json::from_str::<Vec<String>>(&text)?;
            let joined = components.join("/");

            return LibraryTrackPath::try_from_str(&joined).map_err(|err| anyhow!(err).into());
        }

        LibraryTrackPath::try_from_str(&text).map_err(|err| anyhow!(err).into())
    }
}

/// Where an item came from.
#[derive(AsExpression, FromSqlRow, Copy, Clone, Debug, PartialEq, Eq)]
#[diesel(sql_type = Text)]
pub enum Source {
    Metadata,
    User,
    Import,
}

impl Source {
    fn as_str(&self) -> &'static str {
        match self {
            Source::Metadata => "metadata",
            Source::User => "user",
            Source::Import => "import",
        }
    }
}

impl ToSql<Text, Sqlite> for Source {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> diesel::serialize::Result {
        out.set_value(self.as_str());
        Ok(IsNull::No)
    }
}

impl<DB> FromSql<Text, DB> for Source
where
    DB: Backend,
    String: FromSql<Text, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        let value = String::from_sql(bytes)?;

        match value.as_str() {
            "metadata" => Ok(Source::Metadata),
            "user" => Ok(Source::User),
            "import" => Ok(Source::Import),
            other => Err(format!("Unknown item source \"{other}\"").into()),
        }
    }
}
