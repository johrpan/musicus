//! This module contains higher-level models combining information from
//! multiple database tables.

use std::{fmt::Display, path::Path};

use anyhow::Result;
use diesel::prelude::*;

use super::{schema::*, tables, TranslatedString};

// Re-exports for tables that don't need additional information.
pub use tables::{Instrument, Person, Role};

#[derive(Clone, Debug)]
pub struct Work {
    pub work_id: String,
    pub name: TranslatedString,
    pub parts: Vec<WorkPart>,
    pub persons: Vec<Composer>,
    pub instruments: Vec<Instrument>,
}

// TODO: Handle part composers.
#[derive(Default, Clone, Debug)]
pub struct WorkPart {
    pub work_id: String,
    pub level: u8,
    pub name: TranslatedString,
}

#[derive(Queryable, Selectable, Clone, Debug)]
pub struct Composer {
    #[diesel(embed)]
    pub person: Person,
    #[diesel(embed)]
    pub role: Role,
}

#[derive(Clone, Debug)]
pub struct Ensemble {
    pub ensemble_id: String,
    pub name: TranslatedString,
    pub persons: Vec<(Person, Instrument)>,
}

#[derive(Clone, Debug)]
pub struct Recording {
    pub recording_id: String,
    pub work: Work,
    pub year: Option<i32>,
    pub persons: Vec<Performer>,
    pub ensembles: Vec<Ensemble>,
    pub tracks: Vec<Track>,
}

#[derive(Clone, Debug)]
pub struct Performer {
    pub person: Person,
    pub role: Role,
    pub instrument: Option<Instrument>,
}

#[derive(Clone, Debug)]
pub struct Track {
    pub track_id: String,
    pub path: String,
    pub works: Vec<Work>,
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

impl Eq for WorkPart {}
impl PartialEq for WorkPart {
    fn eq(&self, other: &Self) -> bool {
        self.work_id == other.work_id
    }
}

impl Work {
    pub fn from_table(data: tables::Work, connection: &mut SqliteConnection) -> Result<Self> {
        fn visit_children(
            work_id: &str,
            level: u8,
            connection: &mut SqliteConnection,
        ) -> Result<Vec<WorkPart>> {
            let mut parts = Vec::new();

            let children: Vec<tables::Work> = works::table
                .filter(works::parent_work_id.eq(work_id))
                .load(connection)?;

            for child in children {
                let mut grand_children = visit_children(&child.work_id, level + 1, connection)?;

                parts.push(WorkPart {
                    work_id: child.work_id,
                    level,
                    name: child.name,
                });

                parts.append(&mut grand_children);
            }

            Ok(parts)
        }

        let parts = visit_children(&data.work_id, 0, connection)?;

        let persons: Vec<Composer> = persons::table
            .inner_join(work_persons::table.inner_join(roles::table))
            .order(work_persons::sequence_number)
            .filter(work_persons::work_id.eq(&data.work_id))
            .select(Composer::as_select())
            .load(connection)?;

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
        })
    }

    pub fn composers_string(&self) -> String {
        // TODO: Include roles except default composer.
        // TODO: Think about works without composers.
        self.persons
            .iter()
            .map(|p| p.person.name.get().to_string())
            .collect::<Vec<String>>()
            .join(", ")
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
        // TODO: Handle works without composers.
        write!(f, "{}: {}", self.composers_string(), self.name)
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
    pub fn from_table(
        data: tables::Recording,
        library_path: &str,
        connection: &mut SqliteConnection,
    ) -> Result<Self> {
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

        let ensembles: Vec<Ensemble> = ensembles::table
            .inner_join(recording_ensembles::table)
            .order(recording_ensembles::sequence_number)
            .filter(recording_ensembles::recording_id.eq(&data.recording_id))
            .select(tables::Ensemble::as_select())
            .load::<tables::Ensemble>(connection)?
            .into_iter()
            .map(|e| Ensemble::from_table(e, connection))
            .collect::<Result<Vec<Ensemble>>>()?;

        let tracks: Vec<Track> = tracks::table
            .order(tracks::sequence_number)
            .filter(tracks::recording_id.eq(&data.recording_id))
            .select(tables::Track::as_select())
            .load::<tables::Track>(connection)?
            .into_iter()
            .map(|t| Track::from_table(t, library_path, connection))
            .collect::<Result<Vec<Track>>>()?;

        Ok(Self {
            recording_id: data.recording_id,
            work,
            year: data.year,
            persons,
            ensembles,
            tracks,
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
                .map(|e| e.name.get().to_string())
                .collect::<Vec<String>>(),
        );

        performers.join(", ")
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

        let role: Role = roles::table
            .filter(roles::role_id.eq(&data.role_id))
            .first(connection)?;

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
        match &self.instrument {
            Some(instrument) => {
                format!("{} ({})", self.person.name.get(), instrument.name.get()).fmt(f)
            }
            None => self.person.name.get().fmt(f),
        }
    }
}

impl Track {
    pub fn from_table(
        data: tables::Track,
        library_path: &str,
        connection: &mut SqliteConnection,
    ) -> Result<Self> {
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
            path: Path::new(library_path)
                .join(&data.path)
                .to_str()
                .unwrap()
                .to_string(),
            works,
        })
    }
}
