//! This module contains higher-level models combining information from
//! multiple database tables.

use std::{collections::HashSet, fmt::Display, path::PathBuf};

use anyhow::Result;
use diesel::prelude::*;
use gtk::glib::{self, Boxed};
// Re-exports for tables that don't need additional information.
pub use tables::{Instrument, Person, Role};

use super::{schema::*, tables, TranslatedString};

#[derive(Boxed, Clone, Debug)]
#[boxed_type(name = "MusicusWork")]
pub struct Work {
    pub work_id: String,
    pub name: TranslatedString,
    pub parts: Vec<Work>,
    pub persons: Vec<Composer>,
    pub instruments: Vec<Instrument>,
    pub enable_updates: bool,
}

#[derive(Clone, Debug)]
pub struct Composer {
    pub person: Person,
    pub role: Option<Role>,
}

#[derive(Boxed, Clone, Debug)]
#[boxed_type(name = "MusicusEnsemble")]
pub struct Ensemble {
    pub ensemble_id: String,
    pub name: TranslatedString,
    pub persons: Vec<(Person, Instrument)>,
    pub enable_updates: bool,
}

#[derive(Boxed, Clone, Debug)]
#[boxed_type(name = "MusicusRecording")]
pub struct Recording {
    pub recording_id: String,
    pub work: Work,
    pub year: Option<i32>,
    pub persons: Vec<Performer>,
    pub ensembles: Vec<EnsemblePerformer>,
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

#[derive(Boxed, Clone, Debug)]
#[boxed_type(name = "MusicusAlbum")]
pub struct Album {
    pub album_id: String,
    pub name: TranslatedString,
    pub recordings: Vec<Recording>,
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

        Ok(Self {
            work_id: data.work_id,
            name: data.name,
            parts,
            persons,
            instruments,
            enable_updates: data.enable_updates,
        })
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
        let persons: Vec<(Person, Instrument)> = persons::table
            .inner_join(ensemble_persons::table.inner_join(instruments::table))
            .order(ensemble_persons::sequence_number)
            .filter(ensemble_persons::ensemble_id.eq(&data.ensemble_id))
            .select((tables::Person::as_select(), tables::Instrument::as_select()))
            .load(connection)?;

        Ok(Self {
            ensemble_id: data.ensemble_id,
            name: data.name,
            persons,
            enable_updates: data.enable_updates,
        })
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

        Ok(Self {
            recording_id: data.recording_id,
            work,
            year: data.year,
            persons,
            ensembles,
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

        let instrument = match &data.instrument_id {
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
