//! Supported domain models.
//!
//! These types describe the library's data independently of how it is stored.
//! They contain no Diesel types and never touch a database connection; the
//! `db` layer loads rows and assembles these models in memory.

use std::fmt::Display;

use crate::{
    db::{tables, TranslatedString},
    path::LibraryTrackPath,
    Source,
};

/// A person credited as a work person (composer) or recording person
/// (performer).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Person {
    pub id: String,
    pub name: TranslatedString,
    pub source: Source,
    pub last_used_at: chrono::NaiveDateTime,
    pub enable_updates: bool,
}

impl Display for Person {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl From<tables::Person> for Person {
    fn from(row: tables::Person) -> Self {
        Self {
            id: row.person_id,
            name: row.name,
            source: row.source,
            last_used_at: row.last_used_at,
            enable_updates: row.enable_updates,
        }
    }
}

/// A credit role.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Role {
    pub id: String,
    pub name: TranslatedString,
    pub source: Source,
    pub last_used_at: chrono::NaiveDateTime,
    pub enable_updates: bool,
}

impl Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl From<tables::Role> for Role {
    fn from(row: tables::Role) -> Self {
        Self {
            id: row.role_id,
            name: row.name,
            source: row.source,
            last_used_at: row.last_used_at,
            enable_updates: row.enable_updates,
        }
    }
}

/// An instrument.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Instrument {
    pub id: String,
    pub name: TranslatedString,
    pub source: Source,
    pub last_used_at: chrono::NaiveDateTime,
    pub enable_updates: bool,
}

impl Display for Instrument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl From<tables::Instrument> for Instrument {
    fn from(row: tables::Instrument) -> Self {
        Self {
            id: row.instrument_id,
            name: row.name,
            source: row.source,
            last_used_at: row.last_used_at,
            enable_updates: row.enable_updates,
        }
    }
}

/// A tag, possibly carrying a value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tag {
    pub id: String,
    pub name: TranslatedString,
    pub takes_value: bool,
    pub source: Source,
    pub last_used_at: chrono::NaiveDateTime,
    pub enable_updates: bool,
    pub private: bool,
}

impl Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl From<tables::Tag> for Tag {
    fn from(row: tables::Tag) -> Self {
        Self {
            id: row.tag_id,
            name: row.name,
            takes_value: row.takes_value,
            source: row.source,
            last_used_at: row.last_used_at,
            enable_updates: row.enable_updates,
            private: row.private,
        }
    }
}

/// A person credited on a work, with an optional role.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkPerson {
    pub person: Person,
    pub role: Option<Role>,
}

impl Display for WorkPerson {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.role {
            Some(role) => format!("{} ({})", self.person.name.get(), role.name.get()).fmt(f),
            None => self.person.name.get().fmt(f),
        }
    }
}

/// A tag as assigned to a work or recording.
///
/// `value` is set exactly when the tag's `takes_value` is true; a plain label
/// carries no value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TagValue {
    pub tag: Tag,
    pub value: Option<TranslatedString>,
}

impl Display for TagValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.value {
            Some(value) => write!(f, "{}: {}", self.tag.name.get(), value.get()),
            None => write!(f, "{}", self.tag.name.get()),
        }
    }
}

/// A work related to another one, without the related work's own parts or
/// relations: the relation graph may cycle, so it must not be loaded
/// recursively.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelatedWork {
    pub id: String,
    pub name: TranslatedString,
    pub work_persons: Vec<WorkPerson>,
}

impl RelatedWork {
    pub fn work_persons_string(&self) -> Option<String> {
        let work_persons_string = self
            .work_persons
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<String>>()
            .join(", ");

        if work_persons_string.is_empty() {
            None
        } else {
            Some(work_persons_string)
        }
    }
}

/// A work, with its parts loaded recursively and its related works as
/// non-recursive summaries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Work {
    pub id: String,
    pub name: TranslatedString,
    pub source: Source,
    pub last_used_at: chrono::NaiveDateTime,
    pub enable_updates: bool,
    pub parts: Vec<Work>,
    pub work_persons: Vec<WorkPerson>,
    pub instruments: Vec<Instrument>,
    pub tags: Vec<TagValue>,
    pub related_works: Vec<RelatedWork>,
}

impl Work {
    /// Whether `work_id` is this work itself or one of its parts, at any depth.
    pub fn contains(&self, work_id: &str) -> bool {
        self.id == work_id || self.parts.iter().any(|part| part.contains(work_id))
    }

    pub fn work_persons_string(&self) -> Option<String> {
        let work_persons_string = self
            .work_persons
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<String>>()
            .join(", ");

        if work_persons_string.is_empty() {
            None
        } else {
            Some(work_persons_string)
        }
    }
}

impl Display for Work {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(work_persons) = self.work_persons_string() {
            write!(f, "{}: {}", work_persons, self.name)
        } else {
            write!(f, "{}", self.name)
        }
    }
}

/// A person performing in a recording or playing in an ensemble, with its
/// role and instrument.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordingPerson {
    pub person: Person,
    pub role: Option<Role>,
    pub instrument: Option<Instrument>,
}

impl Display for RecordingPerson {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (&self.role, &self.instrument) {
            (_, Some(instrument)) => {
                format!("{} ({})", self.person.name.get(), instrument.name.get()).fmt(f)
            }
            (Some(role), _) => format!("{} ({})", self.person.name.get(), role.name.get()).fmt(f),
            (None, None) => self.person.name.get().fmt(f),
        }
    }
}

/// An ensemble performing in a recording, with an optional role.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordingEnsemble {
    pub ensemble: Ensemble,
    pub role: Option<Role>,
}

impl Display for RecordingEnsemble {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.ensemble.name.get().fmt(f)
    }
}

/// An ensemble with its members.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ensemble {
    pub id: String,
    pub name: TranslatedString,
    pub source: Source,
    pub last_used_at: chrono::NaiveDateTime,
    pub enable_updates: bool,
    pub members: Vec<RecordingPerson>,
}

impl Ensemble {
    pub fn members_string(&self) -> Option<String> {
        let members_string = self
            .members
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<String>>()
            .join(", ");

        if members_string.is_empty() {
            None
        } else {
            Some(members_string)
        }
    }
}

impl Display for Ensemble {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// A recording of a work, with its performing persons and ensembles.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recording {
    pub id: String,
    pub work: Work,
    pub source: Source,
    pub last_used_at: chrono::NaiveDateTime,
    pub enable_updates: bool,
    pub recording_persons: Vec<RecordingPerson>,
    pub recording_ensembles: Vec<RecordingEnsemble>,
    pub tags: Vec<TagValue>,
    pub comment: Option<String>,
}

impl Recording {
    pub fn recording_persons_string(&self) -> String {
        let mut recording_persons = self
            .recording_persons
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<String>>();

        recording_persons.append(
            &mut self
                .recording_ensembles
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<String>>(),
        );

        recording_persons.join(", ")
    }
}

impl Display for Recording {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}; {}", self.work, self.recording_persons_string())
    }
}

/// A track of a recording.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Track {
    pub id: String,
    pub recording_id: String,
    pub recording_index: i32,
    pub path: LibraryTrackPath,
    pub works: Vec<Work>,
}

/// An album: an ordered collection of recordings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Album {
    pub id: String,
    pub name: TranslatedString,
    pub source: Source,
    pub last_used_at: chrono::NaiveDateTime,
    pub enable_updates: bool,
    pub recordings: Vec<Recording>,
}

impl Album {
    pub fn recording_persons_string(&self) -> String {
        let mut seen = std::collections::HashSet::new();
        let mut names = Vec::new();

        for recording in &self.recordings {
            for recording_person in &recording.recording_persons {
                let name = recording_person.to_string();
                if seen.insert(name.clone()) {
                    names.push(name);
                }
            }

            for recording_ensemble in &recording.recording_ensembles {
                let name = recording_ensemble.to_string();
                if seen.insert(name.clone()) {
                    names.push(name);
                }
            }
        }

        names.join(", ")
    }
}

impl Display for Album {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}
