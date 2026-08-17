//! This module contains structs that are one-to-one representations of the
//! tables in the database schema.

use std::path::{Path, PathBuf};

use super::{schema::*, TranslatedString};
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

/// A label that can be assigned to works and recordings.
///
/// `takes_value` distinguishes the two kinds of tag: a plain label such as
/// "Baroque" is shared by many items and is offered as a search facet, while a
/// tag like "Catalogue" names a property whose value ("BWV 1043") is stored on
/// the assignment itself.
///
/// A `private` tag is personal to this library: it works like any other tag
/// locally, but neither it nor the assignments referring to it leave the
/// library in an export.
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
    pub value: Option<String>,
    pub sequence_number: i32,
}

#[derive(Insertable, Queryable, Selectable, Clone, Debug)]
#[diesel(check_for_backend(Sqlite))]
pub struct RecordingTag {
    pub recording_id: String,
    pub tag_id: String,
    pub value: Option<String>,
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
    pub relates_to: Option<String>,
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
    pub medium_id: Option<String>,
    pub medium_index: Option<i32>,
    pub path: PathBufWrapper,
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
pub struct Medium {
    pub medium_id: String,
    pub discid: String,
    pub source: Source,
    pub enable_updates: bool,
    pub created_at: NaiveDateTime,
    pub edited_at: NaiveDateTime,
    pub last_used_at: NaiveDateTime,
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

#[derive(Insertable, Queryable, Selectable, Clone, Debug)]
#[diesel(check_for_backend(Sqlite))]
pub struct AlbumMedium {
    pub album_id: String,
    pub medium_id: String,
    pub sequence_number: i32,
}

#[derive(AsExpression, FromSqlRow, Clone, Debug)]
#[diesel(sql_type = Text)]
pub struct PathBufWrapper(pub PathBuf);

impl ToSql<Text, Sqlite> for PathBufWrapper
where
    String: ToSql<Text, Sqlite>,
{
    fn to_sql(&self, out: &mut Output<Sqlite>) -> diesel::serialize::Result {
        out.set_value(serde_json::to_string(
            &self
                .0
                .iter()
                .map(|p| {
                    p.to_str()
                        .ok_or_else(|| anyhow!("Path contains invalid UTF-8"))
                })
                .collect::<Result<Vec<&str>>>()?,
        )?);

        Ok(IsNull::No)
    }
}

impl<DB> FromSql<Text, DB> for PathBufWrapper
where
    DB: Backend,
    String: FromSql<Text, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        Ok(PathBufWrapper(
            serde_json::from_str::<Vec<String>>(&String::from_sql(bytes)?)?
                .into_iter()
                .collect(),
        ))
    }
}

impl From<PathBuf> for PathBufWrapper {
    fn from(value: PathBuf) -> Self {
        PathBufWrapper(value)
    }
}

impl From<PathBufWrapper> for PathBuf {
    fn from(value: PathBufWrapper) -> Self {
        value.0
    }
}

impl AsRef<Path> for PathBufWrapper {
    fn as_ref(&self) -> &Path {
        self.0.as_ref()
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
