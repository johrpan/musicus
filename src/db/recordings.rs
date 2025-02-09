use chrono::{DateTime, TimeZone, Utc};
use diesel::prelude::*;
use log::info;

use crate::db::{
    defer_foreign_keys, generate_id, get_ensemble, get_instrument, get_person, get_work,
    schema::{ensembles, performances, persons, recordings},
    update_ensemble, update_instrument, update_person, update_work, Ensemble, Error, Instrument,
    Person, Result, Work,
};

/// A specific recording of a work.
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct Recording {
    pub id: String,
    pub work: Work,
    pub comment: String,
    pub performances: Vec<Performance>,
    pub last_used: Option<DateTime<Utc>>,
    pub last_played: Option<DateTime<Utc>>,
}

impl Recording {
    pub fn new(id: String, work: Work, comment: String, performances: Vec<Performance>) -> Self {
        Self {
            id,
            work,
            comment,
            performances,
            last_used: Some(Utc::now()),
            last_played: None,
        }
    }

    /// Initialize a new recording with a work.
    pub fn from_work(work: Work) -> Self {
        Self {
            id: generate_id(),
            work,
            comment: String::new(),
            performances: Vec::new(),
            last_used: Some(Utc::now()),
            last_played: None,
        }
    }

    /// Get a string representation of the performances in this recording.
    // TODO: Maybe replace with impl Display?
    pub fn get_performers(&self) -> String {
        let texts: Vec<String> = self
            .performances
            .iter()
            .map(|performance| performance.get_title())
            .collect();

        texts.join(", ")
    }
}

/// How a person or ensemble was involved in a recording.
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct Performance {
    pub performer: PersonOrEnsemble,
    pub role: Option<Instrument>,
}

impl Performance {
    /// Get a string representation of the performance.
    // TODO: Replace with impl Display.
    pub fn get_title(&self) -> String {
        let performer_title = self.performer.get_title();

        if let Some(role) = &self.role {
            format!("{} ({})", performer_title, role.name)
        } else {
            performer_title
        }
    }
}

/// Either a person or an ensemble.
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum PersonOrEnsemble {
    Person(Person),
    Ensemble(Ensemble),
}

impl PersonOrEnsemble {
    /// Get a short textual representation of the item.
    pub fn get_title(&self) -> String {
        match self {
            PersonOrEnsemble::Person(person) => person.name_lf(),
            PersonOrEnsemble::Ensemble(ensemble) => ensemble.name.clone(),
        }
    }
}

/// Database table data for a recording.
#[derive(Insertable, Queryable, QueryableByName, Debug, Clone)]
#[diesel(table_name = recordings)]
struct RecordingRow {
    pub id: String,
    pub work: String,
    pub comment: String,
    pub last_used: Option<i64>,
    pub last_played: Option<i64>,
}

impl From<Recording> for RecordingRow {
    fn from(recording: Recording) -> Self {
        RecordingRow {
            id: recording.id,
            work: recording.work.id,
            comment: recording.comment,
            last_used: Some(Utc::now().timestamp()),
            last_played: recording.last_played.map(|t| t.timestamp()),
        }
    }
}

/// Database table data for a performance.
#[derive(Insertable, Queryable, Debug, Clone)]
#[diesel(table_name = performances)]
struct PerformanceRow {
    pub id: i64,
    pub recording: String,
    pub person: Option<String>,
    pub ensemble: Option<String>,
    pub role: Option<String>,
}

/// Update an existing recording or insert a new one.
// TODO: Think about whether to also insert the other items.
pub fn update_recording(connection: &mut SqliteConnection, recording: Recording) -> Result<()> {
    info!("Updating recording {:?}", recording);
    defer_foreign_keys(connection)?;

    connection.transaction::<(), Error, _>(|connection| {
        let recording_id = &recording.id;
        delete_recording(connection, recording_id)?;

        // Add associated items from the server, if they don't already exist.

        if get_work(connection, &recording.work.id)?.is_none() {
            update_work(connection, recording.work.clone())?;
        }

        for performance in &recording.performances {
            match &performance.performer {
                PersonOrEnsemble::Person(person) => {
                    if get_person(connection, &person.id)?.is_none() {
                        update_person(connection, person.clone())?;
                    }
                }
                PersonOrEnsemble::Ensemble(ensemble) => {
                    if get_ensemble(connection, &ensemble.id)?.is_none() {
                        update_ensemble(connection, ensemble.clone())?;
                    }
                }
            }

            if let Some(role) = &performance.role {
                if get_instrument(connection, &role.id)?.is_none() {
                    update_instrument(connection, role.clone())?;
                }
            }
        }

        // Add the actual recording.

        let row: RecordingRow = recording.clone().into();
        diesel::insert_into(recordings::table)
            .values(row)
            .execute(connection)?;

        for performance in recording.performances {
            let (person, ensemble) = match performance.performer {
                PersonOrEnsemble::Person(person) => (Some(person.id), None),
                PersonOrEnsemble::Ensemble(ensemble) => (None, Some(ensemble.id)),
            };

            let row = PerformanceRow {
                id: rand::random(),
                recording: recording_id.to_string(),
                person,
                ensemble,
                role: performance.role.map(|role| role.id),
            };

            diesel::insert_into(performances::table)
                .values(row)
                .execute(connection)?;
        }

        Ok(())
    })?;

    Ok(())
}

/// Check whether the database contains a recording.
pub fn recording_exists(connection: &mut SqliteConnection, id: &str) -> Result<bool> {
    let exists = recordings::table
        .filter(recordings::id.eq(id))
        .load::<RecordingRow>(connection)?
        .first()
        .is_some();

    Ok(exists)
}

/// Get an existing recording.
pub fn get_recording(connection: &mut SqliteConnection, id: &str) -> Result<Option<Recording>> {
    let row = recordings::table
        .filter(recordings::id.eq(id))
        .load::<RecordingRow>(connection)?
        .into_iter()
        .next();

    let recording = match row {
        Some(row) => Some(get_recording_data(connection, row)?),
        None => None,
    };

    Ok(recording)
}

/// Get a random recording from the database.
pub fn random_recording(connection: &mut SqliteConnection) -> Result<Recording> {
    let row = diesel::sql_query("SELECT * FROM recordings ORDER BY RANDOM() LIMIT 1")
        .load::<RecordingRow>(connection)?
        .into_iter()
        .next()
        .ok_or(Error::Other("Failed to find random recording."))?;

    get_recording_data(connection, row)
}

/// Retrieve all available information on a recording from related tables.
fn get_recording_data(connection: &mut SqliteConnection, row: RecordingRow) -> Result<Recording> {
    let mut performance_descriptions: Vec<Performance> = Vec::new();

    let performance_rows = performances::table
        .filter(performances::recording.eq(&row.id))
        .load::<PerformanceRow>(connection)?;

    for row in performance_rows {
        performance_descriptions.push(Performance {
            performer: if let Some(id) = row.person {
                PersonOrEnsemble::Person(
                    get_person(connection, &id)?.ok_or(Error::MissingItem("person", id))?,
                )
            } else if let Some(id) = row.ensemble {
                PersonOrEnsemble::Ensemble(
                    get_ensemble(connection, &id)?.ok_or(Error::MissingItem("ensemble", id))?,
                )
            } else {
                return Err(Error::Other("Performance without performer"));
            },
            role: match row.role {
                Some(id) => Some(
                    get_instrument(connection, &id)?.ok_or(Error::MissingItem("instrument", id))?,
                ),
                None => None,
            },
        });
    }

    let work_id = row.work;
    let work = get_work(connection, &work_id)?.ok_or(Error::MissingItem("work", work_id))?;

    let recording_description = Recording {
        id: row.id,
        work,
        comment: row.comment,
        performances: performance_descriptions,
        last_used: row.last_used.map(|t| Utc.timestamp_opt(t, 0).unwrap()),
        last_played: row.last_played.map(|t| Utc.timestamp_opt(t, 0).unwrap()),
    };

    Ok(recording_description)
}

/// Get all available information on all recordings where a person is performing.
pub fn get_recordings_for_person(
    connection: &mut SqliteConnection,
    person_id: &str,
) -> Result<Vec<Recording>> {
    let mut recordings: Vec<Recording> = Vec::new();

    let rows = recordings::table
        .inner_join(performances::table.on(performances::recording.eq(recordings::id)))
        .inner_join(persons::table.on(persons::id.nullable().eq(performances::person)))
        .filter(persons::id.eq(person_id))
        .select(recordings::table::all_columns())
        .load::<RecordingRow>(connection)?;

    for row in rows {
        recordings.push(get_recording_data(connection, row)?);
    }

    Ok(recordings)
}

/// Get all available information on all recordings where an ensemble is performing.
pub fn get_recordings_for_ensemble(
    connection: &mut SqliteConnection,
    ensemble_id: &str,
) -> Result<Vec<Recording>> {
    let mut recordings: Vec<Recording> = Vec::new();

    let rows = recordings::table
        .inner_join(performances::table.on(performances::recording.eq(recordings::id)))
        .inner_join(ensembles::table.on(ensembles::id.nullable().eq(performances::ensemble)))
        .filter(ensembles::id.eq(ensemble_id))
        .select(recordings::table::all_columns())
        .load::<RecordingRow>(connection)?;

    for row in rows {
        recordings.push(get_recording_data(connection, row)?);
    }

    Ok(recordings)
}

/// Get allavailable information on all recordings of a work.
pub fn get_recordings_for_work(
    connection: &mut SqliteConnection,
    work_id: &str,
) -> Result<Vec<Recording>> {
    let mut recordings: Vec<Recording> = Vec::new();

    let rows = recordings::table
        .filter(recordings::work.eq(work_id))
        .load::<RecordingRow>(connection)?;

    for row in rows {
        recordings.push(get_recording_data(connection, row)?);
    }

    Ok(recordings)
}

/// Delete an existing recording. This will fail if there are still references to this
/// recording from other tables that are not directly part of the recording data.
pub fn delete_recording(connection: &mut SqliteConnection, id: &str) -> Result<()> {
    info!("Deleting recording {}", id);
    diesel::delete(recordings::table.filter(recordings::id.eq(id))).execute(connection)?;
    Ok(())
}
