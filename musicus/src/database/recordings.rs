use super::schema::{ensembles, performances, persons, recordings};
use super::{Database, Ensemble, Instrument, Person, Work};
use anyhow::{anyhow, Error, Result};
use diesel::prelude::*;
use diesel::{Insertable, Queryable};
use serde::{Deserialize, Serialize};
use std::convert::TryInto;

/// Database table data for a recording.
#[derive(Insertable, Queryable, Debug, Clone)]
#[table_name = "recordings"]
struct RecordingRow {
    pub id: i64,
    pub work: i64,
    pub comment: String,
}

impl From<Recording> for RecordingRow {
    fn from(recording: Recording) -> Self {
        RecordingRow {
            id: recording.id as i64,
            work: recording.work.id as i64,
            comment: recording.comment,
        }
    }
}

/// Database table data for a performance.
#[derive(Insertable, Queryable, Debug, Clone)]
#[table_name = "performances"]
struct PerformanceRow {
    pub id: i64,
    pub recording: i64,
    pub person: Option<i64>,
    pub ensemble: Option<i64>,
    pub role: Option<i64>,
}

/// How a person or ensemble was involved in a recording.
// TODO: Replace person/ensemble with an enum.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Performance {
    pub person: Option<Person>,
    pub ensemble: Option<Ensemble>,
    pub role: Option<Instrument>,
}

impl Performance {
    /// Get a string representation of the performance.
    // TODO: Replace with impl Display.
    pub fn get_title(&self) -> String {
        let mut text = String::from(if self.is_person() {
            self.unwrap_person().name_fl()
        } else {
            self.unwrap_ensemble().name
        });

        if self.has_role() {
            text = text + " (" + &self.unwrap_role().name + ")";
        }

        text
    }

    pub fn is_person(&self) -> bool {
        self.person.is_some()
    }

    pub fn unwrap_person(&self) -> Person {
        self.person.clone().unwrap()
    }

    pub fn unwrap_ensemble(&self) -> Ensemble {
        self.ensemble.clone().unwrap()
    }

    pub fn has_role(&self) -> bool {
        self.role.clone().is_some()
    }

    pub fn unwrap_role(&self) -> Instrument {
        self.role.clone().unwrap()
    }
}

/// A specific recording of a work.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Recording {
    pub id: u32,
    pub work: Work,
    pub comment: String,
    pub performances: Vec<Performance>,
}

impl Recording {
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

impl Database {
    /// Update an existing recording or insert a new one.
    // TODO: Think about whether to also insert the other items.
    pub fn update_recording(&self, recording: Recording) -> Result<()> {
        self.defer_foreign_keys()?;
        self.connection.transaction::<(), Error, _>(|| {
            self.delete_recording(recording.id)?;

            let recording_id = recording.id as i64;
            let row: RecordingRow = recording.clone().into();
            diesel::insert_into(recordings::table)
                .values(row)
                .execute(&self.connection)?;

            for performance in recording.performances {
                let row = PerformanceRow {
                    id: rand::random(),
                    recording: recording_id,
                    person: performance.person.map(|person| person.id as i64),
                    ensemble: performance.ensemble.map(|ensemble| ensemble.id as i64),
                    role: performance.role.map(|role| role.id as i64),
                };

                diesel::insert_into(performances::table)
                    .values(row)
                    .execute(&self.connection)?;
            }

            Ok(())
        })?;

        Ok(())
    }

    /// Retrieve all available information on a recording from related tables.
    fn get_recording_data(&self, row: RecordingRow) -> Result<Recording> {
        let mut performance_descriptions: Vec<Performance> = Vec::new();

        let performance_rows = performances::table
            .filter(performances::recording.eq(row.id))
            .load::<PerformanceRow>(&self.connection)?;

        for row in performance_rows {
            performance_descriptions.push(Performance {
                person: match row.person {
                    Some(id) => Some(
                        self.get_person(id.try_into()?)?
                            .ok_or(anyhow!("No person with ID: {}", id))?,
                    ),
                    None => None,
                },
                ensemble: match row.ensemble {
                    Some(id) => Some(
                        self.get_ensemble(id.try_into()?)?
                            .ok_or(anyhow!("No ensemble with ID: {}", id))?,
                    ),
                    None => None,
                },
                role: match row.role {
                    Some(id) => Some(
                        self.get_instrument(id.try_into()?)?
                            .ok_or(anyhow!("No instrument with ID: {}", id))?,
                    ),
                    None => None,
                },
            });
        }

        let work_id: u32 = row.work.try_into()?;
        let work = self
            .get_work(work_id)?
            .ok_or(anyhow!("Work doesn't exist: {}", work_id))?;

        let recording_description = Recording {
            id: row.id.try_into()?,
            work,
            comment: row.comment.clone(),
            performances: performance_descriptions,
        };

        Ok(recording_description)
    }

    /// Get all available information on all recordings where a person is performing.
    pub fn get_recordings_for_person(&self, person_id: u32) -> Result<Vec<Recording>> {
        let mut recordings: Vec<Recording> = Vec::new();

        let rows = recordings::table
            .inner_join(performances::table.on(performances::recording.eq(recordings::id)))
            .inner_join(persons::table.on(persons::id.nullable().eq(performances::person)))
            .filter(persons::id.eq(person_id as i64))
            .select(recordings::table::all_columns())
            .load::<RecordingRow>(&self.connection)?;

        for row in rows {
            recordings.push(self.get_recording_data(row)?);
        }

        Ok(recordings)
    }

    /// Get all available information on all recordings where an ensemble is performing.
    pub fn get_recordings_for_ensemble(&self, ensemble_id: u32) -> Result<Vec<Recording>> {
        let mut recordings: Vec<Recording> = Vec::new();

        let rows = recordings::table
            .inner_join(performances::table.on(performances::recording.eq(recordings::id)))
            .inner_join(ensembles::table.on(ensembles::id.nullable().eq(performances::ensemble)))
            .filter(ensembles::id.eq(ensemble_id as i64))
            .select(recordings::table::all_columns())
            .load::<RecordingRow>(&self.connection)?;

        for row in rows {
            recordings.push(self.get_recording_data(row)?);
        }

        Ok(recordings)
    }

    /// Get allavailable information on all recordings of a work.
    pub fn get_recordings_for_work(&self, work_id: u32) -> Result<Vec<Recording>> {
        let mut recordings: Vec<Recording> = Vec::new();

        let rows = recordings::table
            .filter(recordings::work.eq(work_id as i64))
            .load::<RecordingRow>(&self.connection)?;

        for row in rows {
            recordings.push(self.get_recording_data(row)?);
        }

        Ok(recordings)
    }

    /// Delete an existing recording. This will fail if there are still references to this
    /// recording from other tables that are not directly part of the recording data.
    pub fn delete_recording(&self, id: u32) -> Result<()> {
        diesel::delete(recordings::table.filter(recordings::id.eq(id as i64)))
            .execute(&self.connection)?;
        Ok(())
    }
}
