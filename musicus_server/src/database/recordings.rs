use super::schema::{ensembles, performances, persons, recordings};
use super::{get_ensemble, get_instrument, get_person, get_work};
use super::{DbConn, Ensemble, Instrument, Person, User, Work};
use crate::error::ServerError;
use anyhow::{anyhow, Error, Result};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

/// A specific recording of a work.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Recording {
    pub id: String,
    pub work: Work,
    pub comment: String,
    pub performances: Vec<Performance>,
}

/// How a person or ensemble was involved in a recording.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Performance {
    pub person: Option<Person>,
    pub ensemble: Option<Ensemble>,
    pub role: Option<Instrument>,
}

/// Row data for a recording.
#[derive(Insertable, Queryable, Debug, Clone)]
#[table_name = "recordings"]
struct RecordingRow {
    pub id: String,
    pub work: String,
    pub comment: String,
    pub created_by: String,
}

/// Row data for a performance.
#[derive(Insertable, Queryable, Debug, Clone)]
#[table_name = "performances"]
struct PerformanceRow {
    pub id: i64,
    pub recording: String,
    pub person: Option<String>,
    pub ensemble: Option<String>,
    pub role: Option<String>,
}

/// Update an existing recording or insert a new one. This will only work, if the provided user is
/// allowed to do that.
// TODO: Also add newly created associated items.
pub fn update_recording(conn: &DbConn, recording: &Recording, user: &User) -> Result<()> {
    conn.transaction::<(), Error, _>(|| {
        let old_row = get_recording_row(conn, &recording.id)?;

        let allowed = match old_row {
            Some(row) => user.may_edit(&row.created_by),
            None => user.may_create(),
        };

        if allowed {
            let id = &recording.id;

            // This will also delete the old performances.
            diesel::delete(recordings::table)
                .filter(recordings::id.eq(id))
                .execute(conn)?;

            let row = RecordingRow {
                id: id.clone(),
                work: recording.work.id.clone(),
                comment: recording.comment.clone(),
                created_by: user.username.clone(),
            };

            diesel::insert_into(recordings::table)
                .values(row)
                .execute(conn)?;

            for performance in &recording.performances {
                diesel::insert_into(performances::table)
                    .values(PerformanceRow {
                        id: rand::random(),
                        recording: id.clone(),
                        person: performance.person.as_ref().map(|person| person.id.clone()),
                        ensemble: performance
                            .ensemble
                            .as_ref()
                            .map(|ensemble| ensemble.id.clone()),
                        role: performance.role.as_ref().map(|role| role.id.clone()),
                    })
                    .execute(conn)?;
            }

            Ok(())
        } else {
            Err(Error::new(ServerError::Forbidden))
        }
    })?;

    Ok(())
}

/// Get an existing recording and all available information from related tables.
pub fn get_recording(conn: &DbConn, id: &str) -> Result<Option<Recording>> {
    let recording = match get_recording_row(conn, id)? {
        Some(row) => Some(get_description_for_recording_row(conn, &row)?),
        None => None,
    };

    Ok(recording)
}

/// Get all available information on all recordings where a person is performing.
pub fn get_recordings_for_person(conn: &DbConn, person_id: &str) -> Result<Vec<Recording>> {
    let mut recordings: Vec<Recording> = Vec::new();

    let rows = recordings::table
        .inner_join(performances::table.on(performances::recording.eq(recordings::id)))
        .inner_join(persons::table.on(persons::id.nullable().eq(performances::person)))
        .filter(persons::id.eq(person_id))
        .select(recordings::table::all_columns())
        .load::<RecordingRow>(conn)?;

    for row in rows {
        recordings.push(get_description_for_recording_row(conn, &row)?);
    }

    Ok(recordings)
}

/// Get all available information on all recordings where an ensemble is performing.
pub fn get_recordings_for_ensemble(conn: &DbConn, ensemble_id: &str) -> Result<Vec<Recording>> {
    let mut recordings: Vec<Recording> = Vec::new();

    let rows = recordings::table
        .inner_join(performances::table.on(performances::recording.eq(recordings::id)))
        .inner_join(ensembles::table.on(ensembles::id.nullable().eq(performances::ensemble)))
        .filter(ensembles::id.eq(ensemble_id))
        .select(recordings::table::all_columns())
        .load::<RecordingRow>(conn)?;

    for row in rows {
        recordings.push(get_description_for_recording_row(conn, &row)?);
    }

    Ok(recordings)
}

/// Get allavailable information on all recordings of a work.
pub fn get_recordings_for_work(conn: &DbConn, work_id: &str) -> Result<Vec<Recording>> {
    let mut recordings: Vec<Recording> = Vec::new();

    let rows = recordings::table
        .filter(recordings::work.eq(work_id))
        .load::<RecordingRow>(conn)?;

    for row in rows {
        recordings.push(get_description_for_recording_row(conn, &row)?);
    }

    Ok(recordings)
}

/// Delete an existing recording. This will fail if there are still references to this
/// recording from other tables that are not directly part of the recording data. Also, the
/// provided user has to be allowed to delete the recording.
pub fn delete_recording(conn: &DbConn, id: &str, user: &User) -> Result<()> {
    if user.may_delete() {
        diesel::delete(recordings::table.filter(recordings::id.eq(id))).execute(conn)?;
        Ok(())
    } else {
        Err(Error::new(ServerError::Forbidden))
    }
}

/// Get an existing recording row.
fn get_recording_row(conn: &DbConn, id: &str) -> Result<Option<RecordingRow>> {
    Ok(recordings::table
        .filter(recordings::id.eq(id))
        .load::<RecordingRow>(conn)?
        .into_iter()
        .next())
}

/// Retrieve all available information on a recording from related tables.
fn get_description_for_recording_row(conn: &DbConn, row: &RecordingRow) -> Result<Recording> {
    let mut performances: Vec<Performance> = Vec::new();

    let performance_rows = performances::table
        .filter(performances::recording.eq(&row.id))
        .load::<PerformanceRow>(conn)?;

    for row in performance_rows {
        performances.push(Performance {
            person: match row.person {
                Some(id) => {
                    Some(get_person(conn, &id)?.ok_or(anyhow!("No person with ID: {}", id))?)
                }
                None => None,
            },
            ensemble: match row.ensemble {
                Some(id) => {
                    Some(get_ensemble(conn, &id)?.ok_or(anyhow!("No ensemble with ID: {}", id))?)
                }
                None => None,
            },
            role: match row.role {
                Some(id) => Some(
                    get_instrument(conn, &id)?.ok_or(anyhow!("No instrument with ID: {}", id))?,
                ),
                None => None,
            },
        });
    }

    let work = get_work(conn, &row.work)?.ok_or(anyhow!("No work with ID: {}", &row.work))?;

    let recording = Recording {
        id: row.id.clone(),
        work,
        comment: row.comment.clone(),
        performances,
    };

    Ok(recording)
}
