//! This module contains higher-level models combining information from
//! multiple database tables.

use std::{collections::HashSet, fmt::Display, path::PathBuf};

use anyhow::Result;
use diesel::prelude::*;

// Re-exports for tables that don't need additional information.
pub use tables::{Instrument, Person, Role, Tag};

use super::{schema::*, tables, TranslatedString};

#[derive(Clone, Debug)]
pub struct Work {
    pub work_id: String,
    pub name: TranslatedString,
    pub parts: Vec<Work>,
    pub persons: Vec<Composer>,
    pub instruments: Vec<Instrument>,
    pub tags: Vec<TagValue>,
    pub relates_to: Option<Box<Work>>,
    pub enable_updates: bool,
}

/// A tag as assigned to a work or recording.
///
/// `value` is set exactly when the tag's `takes_value` is true; a plain label
/// carries no value.
#[derive(Clone, Debug)]
pub struct TagValue {
    pub tag: Tag,
    pub value: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Composer {
    pub person: Person,
    pub role: Option<Role>,
}

#[derive(Clone, Debug)]
pub struct Ensemble {
    pub ensemble_id: String,
    pub name: TranslatedString,
    pub persons: Vec<Performer>,
    pub enable_updates: bool,
}

#[derive(Clone, Debug)]
pub struct Recording {
    pub recording_id: String,
    pub work: Work,
    pub persons: Vec<Performer>,
    pub ensembles: Vec<EnsemblePerformer>,
    pub tags: Vec<TagValue>,
    pub comment: Option<String>,
    pub enable_updates: bool,
}

#[derive(Clone, Debug)]
pub struct Performer {
    pub person: Person,
    pub role: Option<Role>,
    pub instrument: Option<Instrument>,
}

#[derive(Clone, Debug)]
pub struct EnsemblePerformer {
    pub ensemble: Ensemble,
    pub role: Option<Role>,
}

#[derive(Clone, Debug)]
pub struct Track {
    pub track_id: String,
    pub path: PathBuf,
    pub works: Vec<Work>,
}

#[derive(Clone, Debug)]
pub struct Album {
    pub album_id: String,
    pub name: TranslatedString,
    pub recordings: Vec<Recording>,
    pub enable_updates: bool,
}

impl Eq for Person {}
impl PartialEq for Person {
    fn eq(&self, other: &Self) -> bool {
        self.person_id == other.person_id
    }
}

impl Display for Person {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Display for Instrument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Eq for Instrument {}
impl PartialEq for Instrument {
    fn eq(&self, other: &Self) -> bool {
        self.instrument_id == other.instrument_id
    }
}

impl Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Eq for Role {}
impl PartialEq for Role {
    fn eq(&self, other: &Self) -> bool {
        self.role_id == other.role_id
    }
}

impl Eq for Composer {}
impl PartialEq for Composer {
    fn eq(&self, other: &Self) -> bool {
        self.person == other.person && self.role == other.role
    }
}

impl TagValue {
    pub fn load_for_work(work_id: &str, connection: &mut SqliteConnection) -> Result<Vec<Self>> {
        Ok(work_tags::table
            .inner_join(tags::table)
            .order(work_tags::sequence_number)
            .filter(work_tags::work_id.eq(work_id))
            .select((tables::Tag::as_select(), work_tags::value))
            .load::<(Tag, Option<String>)>(connection)?
            .into_iter()
            .map(|(tag, value)| Self { tag, value })
            .collect())
    }

    pub fn load_for_recording(
        recording_id: &str,
        connection: &mut SqliteConnection,
    ) -> Result<Vec<Self>> {
        Ok(recording_tags::table
            .inner_join(tags::table)
            .order(recording_tags::sequence_number)
            .filter(recording_tags::recording_id.eq(recording_id))
            .select((tables::Tag::as_select(), recording_tags::value))
            .load::<(Tag, Option<String>)>(connection)?
            .into_iter()
            .map(|(tag, value)| Self { tag, value })
            .collect())
    }
}

impl Eq for TagValue {}
impl PartialEq for TagValue {
    fn eq(&self, other: &Self) -> bool {
        self.tag == other.tag && self.value == other.value
    }
}

impl Display for TagValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.value {
            Some(value) => write!(f, "{}: {}", self.tag.name.get(), value),
            None => write!(f, "{}", self.tag.name.get()),
        }
    }
}

impl Eq for Tag {}
impl PartialEq for Tag {
    fn eq(&self, other: &Self) -> bool {
        self.tag_id == other.tag_id
    }
}

impl Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Work {
    pub fn from_table(data: tables::Work, connection: &mut SqliteConnection) -> Result<Self> {
        // Note: Because this calls Work::from_table for each part, this recursively
        // adds all children. It does not check for circularity.
        let parts = works::table
            .order(works::sequence_number)
            .filter(works::parent_work_id.eq(&data.work_id))
            .load::<tables::Work>(connection)?
            .into_iter()
            .map(|w| Work::from_table(w, connection))
            .collect::<Result<Vec<Work>>>()?;

        let persons = work_persons::table
            .order(work_persons::sequence_number)
            .filter(work_persons::work_id.eq(&data.work_id))
            .load::<tables::WorkPerson>(connection)?
            .into_iter()
            .map(|r| Composer::from_table(r, connection))
            .collect::<Result<Vec<Composer>>>()?;

        let instruments: Vec<Instrument> = instruments::table
            .inner_join(work_instruments::table)
            .order(work_instruments::sequence_number)
            .filter(work_instruments::work_id.eq(&data.work_id))
            .select(tables::Instrument::as_select())
            .load(connection)?;

        let tags = TagValue::load_for_work(&data.work_id, connection)?;

        // Note: Loaded the same way as a part, so this recurses through the whole
        // relation chain. Like the part tree above, it does not check for circularity.
        let relates_to = match &data.relates_to {
            Some(relates_to) => Some(Box::new(Work::from_table(
                works::table
                    .filter(works::work_id.eq(relates_to))
                    .first::<tables::Work>(connection)?,
                connection,
            )?)),
            None => None,
        };

        Ok(Self {
            work_id: data.work_id,
            name: data.name,
            parts,
            persons,
            instruments,
            tags,
            relates_to,
            enable_updates: data.enable_updates,
        })
    }

    /// Whether `work_id` is this work itself or one of its parts, at any depth.
    pub fn contains(&self, work_id: &str) -> bool {
        self.work_id == work_id || self.parts.iter().any(|part| part.contains(work_id))
    }

    pub fn composers_string(&self) -> Option<String> {
        let composers_string = self
            .persons
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<String>>()
            .join(", ");

        if composers_string.is_empty() {
            None
        } else {
            Some(composers_string)
        }
    }
}

impl Eq for Work {}
impl PartialEq for Work {
    fn eq(&self, other: &Self) -> bool {
        self.work_id == other.work_id
    }
}

impl Display for Work {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(composers) = self.composers_string() {
            write!(f, "{}: {}", composers, self.name)
        } else {
            write!(f, "{}", self.name)
        }
    }
}

impl Composer {
    pub fn from_table(data: tables::WorkPerson, connection: &mut SqliteConnection) -> Result<Self> {
        let person: Person = persons::table
            .filter(persons::person_id.eq(&data.person_id))
            .first(connection)?;

        let role = match &data.role_id {
            Some(role_id) => Some(
                roles::table
                    .filter(roles::role_id.eq(role_id))
                    .first::<Role>(connection)?,
            ),
            None => None,
        };

        Ok(Self { person, role })
    }
}

impl Display for Composer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.role {
            Some(role) => format!("{} ({})", self.person.name.get(), role.name.get()).fmt(f),
            None => self.person.name.get().fmt(f),
        }
    }
}

impl Ensemble {
    pub fn from_table(data: tables::Ensemble, connection: &mut SqliteConnection) -> Result<Self> {
        let persons = ensemble_persons::table
            .order(ensemble_persons::sequence_number)
            .filter(ensemble_persons::ensemble_id.eq(&data.ensemble_id))
            .load::<tables::EnsemblePerson>(connection)?
            .into_iter()
            .map(|r| Performer::from_ensemble_person(r, connection))
            .collect::<Result<Vec<Performer>>>()?;

        Ok(Self {
            ensemble_id: data.ensemble_id,
            name: data.name,
            persons,
            enable_updates: data.enable_updates,
        })
    }

    pub fn members_string(&self) -> Option<String> {
        let members_string = self
            .persons
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

impl Eq for Ensemble {}
impl PartialEq for Ensemble {
    fn eq(&self, other: &Self) -> bool {
        self.ensemble_id == other.ensemble_id
    }
}

impl Display for Ensemble {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Recording {
    pub fn from_table(data: tables::Recording, connection: &mut SqliteConnection) -> Result<Self> {
        let work = Work::from_table(
            works::table
                .filter(works::work_id.eq(&data.work_id))
                .first::<tables::Work>(connection)?,
            connection,
        )?;

        let persons = recording_persons::table
            .order(recording_persons::sequence_number)
            .filter(recording_persons::recording_id.eq(&data.recording_id))
            .load::<tables::RecordingPerson>(connection)?
            .into_iter()
            .map(|r| Performer::from_table(r, connection))
            .collect::<Result<Vec<Performer>>>()?;

        let ensembles = ensembles::table
            .inner_join(recording_ensembles::table)
            .order(recording_ensembles::sequence_number)
            .filter(recording_ensembles::recording_id.eq(&data.recording_id))
            .select(tables::RecordingEnsemble::as_select())
            .load::<tables::RecordingEnsemble>(connection)?
            .into_iter()
            .map(|e| EnsemblePerformer::from_table(e, connection))
            .collect::<Result<Vec<EnsemblePerformer>>>()?;

        let tags = TagValue::load_for_recording(&data.recording_id, connection)?;

        Ok(Self {
            recording_id: data.recording_id,
            work,
            persons,
            ensembles,
            tags,
            comment: data.comment,
            enable_updates: data.enable_updates,
        })
    }

    pub fn performers_string(&self) -> String {
        let mut performers = self
            .persons
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<String>>();

        performers.append(
            &mut self
                .ensembles
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<String>>(),
        );

        performers.join(", ")
    }
}

impl Eq for Recording {}
impl PartialEq for Recording {
    fn eq(&self, other: &Self) -> bool {
        self.recording_id == other.recording_id
    }
}

impl Display for Recording {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}; {}", self.work, self.performers_string())
    }
}

impl Performer {
    pub fn from_table(
        data: tables::RecordingPerson,
        connection: &mut SqliteConnection,
    ) -> Result<Self> {
        Self::load(
            &data.person_id,
            &data.role_id,
            &data.instrument_id,
            connection,
        )
    }

    /// The same (person, role, instrument) shape, read off an ensemble member row
    /// instead of a recording's own performer row.
    pub fn from_ensemble_person(
        data: tables::EnsemblePerson,
        connection: &mut SqliteConnection,
    ) -> Result<Self> {
        Self::load(
            &data.person_id,
            &data.role_id,
            &data.instrument_id,
            connection,
        )
    }

    fn load(
        person_id: &str,
        role_id: &Option<String>,
        instrument_id: &Option<String>,
        connection: &mut SqliteConnection,
    ) -> Result<Self> {
        let person: Person = persons::table
            .filter(persons::person_id.eq(person_id))
            .first(connection)?;

        let role = match role_id {
            Some(role_id) => Some(
                roles::table
                    .filter(roles::role_id.eq(role_id))
                    .first::<Role>(connection)?,
            ),
            None => None,
        };

        let instrument = match instrument_id {
            Some(instrument_id) => Some(
                instruments::table
                    .filter(instruments::instrument_id.eq(instrument_id))
                    .first::<Instrument>(connection)?,
            ),
            None => None,
        };

        Ok(Self {
            person,
            role,
            instrument,
        })
    }
}

impl Display for Performer {
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

impl EnsemblePerformer {
    pub fn from_table(
        data: tables::RecordingEnsemble,
        connection: &mut SqliteConnection,
    ) -> Result<Self> {
        let ensemble_data = ensembles::table
            .filter(ensembles::ensemble_id.eq(&data.ensemble_id))
            .first::<tables::Ensemble>(connection)?;

        let ensemble = Ensemble::from_table(ensemble_data, connection)?;

        let role = match &data.role_id {
            Some(role_id) => Some(
                roles::table
                    .filter(roles::role_id.eq(role_id))
                    .first::<Role>(connection)?,
            ),
            None => None,
        };

        Ok(Self { ensemble, role })
    }
}

impl Display for EnsemblePerformer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.ensemble.name.get().fmt(f)
    }
}

impl Track {
    pub fn from_table(data: tables::Track, connection: &mut SqliteConnection) -> Result<Self> {
        let works: Vec<Work> = works::table
            .inner_join(track_works::table)
            .order(track_works::sequence_number)
            .filter(track_works::track_id.eq(&data.track_id))
            .select(tables::Work::as_select())
            .load::<tables::Work>(connection)?
            .into_iter()
            .map(|w| Work::from_table(w, connection))
            .collect::<Result<Vec<Work>>>()?;

        Ok(Self {
            track_id: data.track_id,
            path: data.path.0,
            works,
        })
    }
}

impl Album {
    pub fn from_table(data: tables::Album, connection: &mut SqliteConnection) -> Result<Self> {
        let recordings: Vec<Recording> = recordings::table
            .inner_join(album_recordings::table)
            .order(album_recordings::sequence_number)
            .filter(album_recordings::album_id.eq(&data.album_id))
            .select(tables::Recording::as_select())
            .load(connection)?
            .into_iter()
            .map(|r| Recording::from_table(r, connection))
            .collect::<Result<Vec<Recording>>>()?;

        Ok(Self {
            album_id: data.album_id,
            name: data.name,
            recordings,
            enable_updates: data.enable_updates,
        })
    }

    pub fn performers_string(&self) -> String {
        let mut performers = HashSet::new();
        let mut ensembles = HashSet::new();

        for recording in &self.recordings {
            for performer in &recording.persons {
                performers.insert(performer.to_string());
            }

            for ensemble in &recording.ensembles {
                ensembles.insert(ensemble.to_string());
            }
        }

        performers
            .into_iter()
            .chain(ensembles)
            .collect::<Vec<String>>()
            .join(", ")
    }
}

impl Eq for Album {}
impl PartialEq for Album {
    fn eq(&self, other: &Self) -> bool {
        self.album_id == other.album_id
    }
}

impl Display for Album {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}
