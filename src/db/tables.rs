//! This module contains structs that are one-to-one representations of the
//! tables in the database schema.

use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::sqlite::Sqlite;
use gtk::glib::{self, Boxed};

use super::{schema::*, TranslatedString};

#[derive(Boxed, Insertable, Queryable, Selectable, Clone, Debug)]
#[boxed_type(name = "MusicusPerson")]
#[diesel(check_for_backend(Sqlite))]
pub struct Person {
    pub person_id: String,
    pub name: TranslatedString,
    pub created_at: NaiveDateTime,
    pub edited_at: NaiveDateTime,
    pub last_used_at: NaiveDateTime,
    pub last_played_at: Option<NaiveDateTime>,
}

#[derive(Boxed, Insertable, Queryable, Selectable, Clone, Debug)]
#[boxed_type(name = "MusicusRole")]
#[diesel(check_for_backend(Sqlite))]
pub struct Role {
    pub role_id: String,
    pub name: TranslatedString,
    pub created_at: NaiveDateTime,
    pub edited_at: NaiveDateTime,
    pub last_used_at: NaiveDateTime,
}

#[derive(Boxed, Insertable, Queryable, Selectable, Clone, Debug)]
#[boxed_type(name = "MusicusInstrument", nullable)]
#[diesel(check_for_backend(Sqlite))]
pub struct Instrument {
    pub instrument_id: String,
    pub name: TranslatedString,
    pub created_at: NaiveDateTime,
    pub edited_at: NaiveDateTime,
    pub last_used_at: NaiveDateTime,
    pub last_played_at: Option<NaiveDateTime>,
}

#[derive(Insertable, Queryable, Selectable, Clone, Debug)]
#[diesel(check_for_backend(Sqlite))]
pub struct Work {
    pub work_id: String,
    pub parent_work_id: Option<String>,
    pub sequence_number: Option<i32>,
    pub name: TranslatedString,
    pub created_at: NaiveDateTime,
    pub edited_at: NaiveDateTime,
    pub last_used_at: NaiveDateTime,
    pub last_played_at: Option<NaiveDateTime>,
}

#[derive(Insertable, Queryable, Selectable, Clone, Debug)]
#[diesel(check_for_backend(Sqlite))]
pub struct WorkPerson {
    pub work_id: String,
    pub person_id: String,
    pub role_id: String,
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
    pub created_at: NaiveDateTime,
    pub edited_at: NaiveDateTime,
    pub last_used_at: NaiveDateTime,
    pub last_played_at: Option<NaiveDateTime>,
}

#[derive(Insertable, Queryable, Selectable, Clone, Debug)]
#[diesel(check_for_backend(Sqlite))]
pub struct EnsemblePerson {
    pub ensemble_id: String,
    pub person_id: String,
    pub instrument_id: String,
    pub sequence_number: i32,
}

#[derive(Insertable, Queryable, Selectable, Clone, Debug)]
#[diesel(check_for_backend(Sqlite))]
pub struct Recording {
    pub recording_id: String,
    pub work_id: String,
    pub year: Option<i32>,
    pub created_at: NaiveDateTime,
    pub edited_at: NaiveDateTime,
    pub last_used_at: NaiveDateTime,
    pub last_played_at: Option<NaiveDateTime>,
}

#[derive(Insertable, Queryable, Selectable, Clone, Debug)]
#[diesel(check_for_backend(Sqlite))]
pub struct RecordingPerson {
    pub recording_id: String,
    pub person_id: String,
    pub role_id: String,
    pub instrument_id: Option<String>,
    pub sequence_number: i32,
}

#[derive(Insertable, Queryable, Selectable, Clone, Debug)]
#[diesel(check_for_backend(Sqlite))]
pub struct RecordingEnsemble {
    pub recording_id: String,
    pub ensemble_id: String,
    pub role_id: String,
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
    pub path: String,
    pub created_at: NaiveDateTime,
    pub edited_at: NaiveDateTime,
    pub last_used_at: NaiveDateTime,
    pub last_played_at: Option<NaiveDateTime>,
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
    pub created_at: NaiveDateTime,
    pub edited_at: NaiveDateTime,
    pub last_used_at: NaiveDateTime,
    pub last_played_at: Option<NaiveDateTime>,
}

#[derive(Insertable, Queryable, Selectable, Clone, Debug)]
#[diesel(check_for_backend(Sqlite))]
pub struct Album {
    pub album_id: String,
    pub name: TranslatedString,
    pub created_at: NaiveDateTime,
    pub edited_at: NaiveDateTime,
    pub last_used_at: NaiveDateTime,
    pub last_played_at: Option<NaiveDateTime>,
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
