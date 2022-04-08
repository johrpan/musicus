use super::generate_id;
use super::schema::{ensembles, performances, persons, recordings};
use super::{Database, Ensemble, Error, Instrument, Person, Result, Work};
use diesel::prelude::*;
use log::info;

/// A specific recording of a work.
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct Recording {
    pub id: String,
    pub work: Work,
    pub comment: String,
    pub performances: Vec<Performance>,
}

impl Recording {
    /// Initialize a new recording with a work.
    pub fn new(work: Work) -> Self {
        Self {
            id: generate_id(),
            work,
            comment: String::new(),
            performances: Vec::new(),
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
#[table_name = "recordings"]
struct RecordingRow {
    pub id: String,
    pub work: String,
    pub comment: String,
}

impl From<Recording> for RecordingRow {
    fn from(recording: Recording) -> Self {
        RecordingRow {
            id: recording.id,
            work: recording.work.id,
            comment: recording.comment,
        }
    }
}

/// Database table data for a performance.
#[derive(Insertable, Queryable, Debug, Clone)]
#[table_name = "performances"]
struct PerformanceRow {
    pub id: i64,
    pub recording: String,
    pub person: Option<String>,
    pub ensemble: Option<String>,
    pub role: Option<String>,
}

impl Database {
    /// Update an existing recording or insert a new one.
    // TODO: Think about whether to also insert the other items.
    pub fn update_recording(&self, recording: Recording) -> Result<()> {
        info!("Updating recording {:?}", recording);
        self.defer_foreign_keys()?;
        self.connection.transaction::<(), Error, _>(|| {
            let recording_id = &recording.id;
            self.delete_recording(recording_id)?;

            // Add associated items from the server, if they don't already exist.

            if self.get_work(&recording.work.id)?.is_none() {
                self.update_work(recording.work.clone())?;
            }

            for performance in &recording.performances {
                match &performance.performer {
                    PersonOrEnsemble::Person(person) => {
                        if self.get_person(&person.id)?.is_none() {
                            self.update_person(person.clone())?;
                        }
                    }
                    PersonOrEnsemble::Ensemble(ensemble) => {
                        if self.get_ensemble(&ensemble.id)?.is_none() {
                            self.update_ensemble(ensemble.clone())?;
                        }
                    }
                }

                if let Some(role) = &performance.role {
                    if self.get_instrument(&role.id)?.is_none() {
                        self.update_instrument(role.clone())?;
                    }
                }
            }

            // Add the actual recording.

            let row: RecordingRow = recording.clone().into();
            diesel::insert_into(recordings::table)
                .values(row)
                .execute(&self.connection)?;

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
                    .execute(&self.connection)?;
            }

            Ok(())
        })?;

        Ok(())
    }

    /// Check whether the database contains a recording.
    pub fn recording_exists(&self, id: &str) -> Result<bool> {
        let exists = recordings::table
            .filter(recordings::id.eq(id))
            .load::<RecordingRow>(&self.connection)?
            .first()
            .is_some();

        Ok(exists)
    }

    /// Get an existing recording.
    pub fn get_recording(&self, id: &str) -> Result<Option<Recording>> {
        let row = recordings::table
            .filter(recordings::id.eq(id))
            .load::<RecordingRow>(&self.connection)?
            .into_iter()
            .next();

        let recording = match row {
            Some(row) => Some(self.get_recording_data(row)?),
            None => None,
        };

        Ok(recording)
    }

    /// Get a random recording from the database.
    pub fn random_recording(&self) -> Result<Recording> {
        let row = diesel::sql_query("SELECT * FROM recordings ORDER BY RANDOM() LIMIT 1")
            .load::<RecordingRow>(&self.connection)?
            .into_iter()
            .next()
            .ok_or(Error::Other("Failed to find random recording."))?;

        self.get_recording_data(row)
    }

    /// Retrieve all available information on a recording from related tables.
    fn get_recording_data(&self, row: RecordingRow) -> Result<Recording> {
        let mut performance_descriptions: Vec<Performance> = Vec::new();

        let performance_rows = performances::table
            .filter(performances::recording.eq(&row.id))
            .load::<PerformanceRow>(&self.connection)?;

        for row in performance_rows {
            performance_descriptions.push(Performance {
                performer: if let Some(id) = row.person {
                    PersonOrEnsemble::Person(
                        self.get_person(&id)?
                            .ok_or(Error::MissingItem("person", id))?,
                    )
                } else if let Some(id) = row.ensemble {
                    PersonOrEnsemble::Ensemble(
                        self.get_ensemble(&id)?
                            .ok_or(Error::MissingItem("ensemble", id))?,
                    )
                } else {
                    return Err(Error::Other("Performance without performer"));
                },
                role: match row.role {
                    Some(id) => Some(
                        self.get_instrument(&id)?
                            .ok_or(Error::MissingItem("instrument", id))?,
                    ),
                    None => None,
                },
            });
        }

        let work_id = row.work;
        let work = self
            .get_work(&work_id)?
            .ok_or(Error::MissingItem("work", work_id))?;

        let recording_description = Recording {
            id: row.id,
            work,
            comment: row.comment,
            performances: performance_descriptions,
        };

        Ok(recording_description)
    }

    /// Get all available information on all recordings where a person is performing.
    pub fn get_recordings_for_person(&self, person_id: &str) -> Result<Vec<Recording>> {
        let mut recordings: Vec<Recording> = Vec::new();

        let rows = recordings::table
            .inner_join(performances::table.on(performances::recording.eq(recordings::id)))
            .inner_join(persons::table.on(persons::id.nullable().eq(performances::person)))
            .filter(persons::id.eq(person_id))
            .select(recordings::table::all_columns())
            .load::<RecordingRow>(&self.connection)?;

        for row in rows {
            recordings.push(self.get_recording_data(row)?);
        }

        Ok(recordings)
    }

    /// Get all available information on all recordings where an ensemble is performing.
    pub fn get_recordings_for_ensemble(&self, ensemble_id: &str) -> Result<Vec<Recording>> {
        let mut recordings: Vec<Recording> = Vec::new();

        let rows = recordings::table
            .inner_join(performances::table.on(performances::recording.eq(recordings::id)))
            .inner_join(ensembles::table.on(ensembles::id.nullable().eq(performances::ensemble)))
            .filter(ensembles::id.eq(ensemble_id))
            .select(recordings::table::all_columns())
            .load::<RecordingRow>(&self.connection)?;

        for row in rows {
            recordings.push(self.get_recording_data(row)?);
        }

        Ok(recordings)
    }

    /// Get allavailable information on all recordings of a work.
    pub fn get_recordings_for_work(&self, work_id: &str) -> Result<Vec<Recording>> {
        let mut recordings: Vec<Recording> = Vec::new();

        let rows = recordings::table
            .filter(recordings::work.eq(work_id))
            .load::<RecordingRow>(&self.connection)?;

        for row in rows {
            recordings.push(self.get_recording_data(row)?);
        }

        Ok(recordings)
    }

    /// Delete an existing recording. This will fail if there are still references to this
    /// recording from other tables that are not directly part of the recording data.
    pub fn delete_recording(&self, id: &str) -> Result<()> {
        info!("Deleting recording {}", id);
        diesel::delete(recordings::table.filter(recordings::id.eq(id)))
            .execute(&self.connection)?;
        Ok(())
    }
}
