use super::schema::{ensembles, instruments, performances, persons, recordings};
use super::{get_work_description, DbConn, Ensemble, Instrument, Person, WorkDescription};
use anyhow::{anyhow, Error, Result};
use diesel::prelude::*;
use diesel::{Insertable, Queryable};
use serde::{Deserialize, Serialize};
use std::convert::TryInto;

/// A specific recording of a work.
#[derive(Insertable, Queryable, Debug, Clone)]
pub struct Recording {
    pub id: i64,
    pub work: i64,
    pub comment: String,
    pub created_by: String,
}

/// How a person or ensemble was involved in a recording.
#[derive(Insertable, Queryable, Debug, Clone)]
pub struct Performance {
    pub id: i64,
    pub recording: i64,
    pub person: Option<i64>,
    pub ensemble: Option<i64>,
    pub role: Option<i64>,
}

/// A structure for collecting all available information on a performance.
#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceDescription {
    pub person: Option<Person>,
    pub ensemble: Option<Ensemble>,
    pub role: Option<Instrument>,
}

/// A structure for collecting all available information on a recording.
#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RecordingDescription {
    pub id: i64,
    pub work: WorkDescription,
    pub comment: String,
    pub performances: Vec<PerformanceDescription>,
}

/// A structure representing data on a performance.
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceInsertion {
    pub person: Option<i64>,
    pub ensemble: Option<i64>,
    pub role: Option<i64>,
}

/// A bundle of everything needed for adding a new recording to the database.
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RecordingInsertion {
    pub work: i64,
    pub comment: String,
    pub performances: Vec<PerformanceInsertion>,
}

/// Insert a new recording.
pub fn insert_recording(
    conn: &DbConn,
    id: u32,
    data: &RecordingInsertion,
    created_by: &str,
) -> Result<()> {
    conn.transaction::<(), Error, _>(|| {
        let id = id as i64;

        diesel::insert_into(recordings::table)
            .values(Recording {
                id,
                work: data.work,
                comment: data.comment.clone(),
                created_by: created_by.to_string(),
            })
            .execute(conn)?;

        insert_recording_data(conn, id, data)?;

        Ok(())
    })?;

    Ok(())
}

/// Update an existing recording.
pub fn update_recording(conn: &DbConn, id: u32, data: &RecordingInsertion) -> Result<()> {
    conn.transaction::<(), Error, _>(|| {
        let id = id as i64;

        diesel::delete(performances::table)
            .filter(performances::recording.eq(id))
            .execute(conn)?;

        diesel::update(recordings::table)
            .filter(recordings::id.eq(id))
            .set((
                recordings::work.eq(data.work),
                recordings::comment.eq(data.comment.clone()),
            ))
            .execute(conn)?;

        insert_recording_data(conn, id, data)?;

        Ok(())
    })?;

    Ok(())
}

/// Helper method to populate other tables related to a recording.
fn insert_recording_data(conn: &DbConn, id: i64, data: &RecordingInsertion) -> Result<()> {
    for performance in &data.performances {
        diesel::insert_into(performances::table)
            .values(Performance {
                id: rand::random(),
                recording: id,
                person: performance.person,
                ensemble: performance.ensemble,
                role: performance.role,
            })
            .execute(conn)?;
    }

    Ok(())
}

/// Get an existing recording.
pub fn get_recording(conn: &DbConn, id: u32) -> Result<Option<Recording>> {
    Ok(recordings::table
        .filter(recordings::id.eq(id as i64))
        .load::<Recording>(conn)?
        .first()
        .cloned())
}

/// Retrieve all available information on a recording from related tables.
pub fn get_description_for_recording(
    conn: &DbConn,
    recording: &Recording,
) -> Result<RecordingDescription> {
    let mut performance_descriptions: Vec<PerformanceDescription> = Vec::new();

    let performances = performances::table
        .filter(performances::recording.eq(recording.id))
        .load::<Performance>(conn)?;

    for performance in performances {
        performance_descriptions.push(PerformanceDescription {
            person: match performance.person {
                Some(id) => Some(
                    persons::table
                        .filter(persons::id.eq(id as i64))
                        .load::<Person>(conn)?
                        .first()
                        .cloned()
                        .ok_or(anyhow!("No person with ID: {}", id))?,
                ),
                None => None,
            },
            ensemble: match performance.ensemble {
                Some(id) => Some(
                    ensembles::table
                        .filter(ensembles::id.eq(id as i64))
                        .load::<Ensemble>(conn)?
                        .first()
                        .cloned()
                        .ok_or(anyhow!("No ensemble with ID: {}", id))?,
                ),
                None => None,
            },
            role: match performance.role {
                Some(id) => Some(
                    instruments::table
                        .filter(instruments::id.eq(id as i64))
                        .load::<Instrument>(conn)?
                        .first()
                        .cloned()
                        .ok_or(anyhow!("No instrument with ID: {}", id))?,
                ),
                None => None,
            },
        });
    }

    let work_id = recording.work.try_into()?;
    let work =
        get_work_description(conn, work_id)?.ok_or(anyhow!("Work doesn't exist: {}", work_id))?;

    let recording_description = RecordingDescription {
        id: recording.id,
        work,
        comment: recording.comment.clone(),
        performances: performance_descriptions,
    };

    Ok(recording_description)
}

/// Get an existing recording and all available information from related tables.
pub fn get_recording_description(conn: &DbConn, id: u32) -> Result<Option<RecordingDescription>> {
    let recording_description = match get_recording(conn, id)? {
        Some(recording) => Some(get_description_for_recording(conn, &recording)?),
        None => None,
    };

    Ok(recording_description)
}

/// Get all available information on all recordings where a person is performing.
pub fn get_recordings_for_person(
    conn: &DbConn,
    person_id: u32,
) -> Result<Vec<RecordingDescription>> {
    let mut recording_descriptions: Vec<RecordingDescription> = Vec::new();

    let recordings = recordings::table
        .inner_join(performances::table.on(performances::recording.eq(recordings::id)))
        .inner_join(persons::table.on(persons::id.nullable().eq(performances::person)))
        .filter(persons::id.eq(person_id as i64))
        .select(recordings::table::all_columns())
        .load::<Recording>(conn)?;

    for recording in recordings {
        recording_descriptions.push(get_description_for_recording(conn, &recording)?);
    }

    Ok(recording_descriptions)
}

/// Get all available information on all recordings where an ensemble is performing.
pub fn get_recordings_for_ensemble(
    conn: &DbConn,
    ensemble_id: u32,
) -> Result<Vec<RecordingDescription>> {
    let mut recording_descriptions: Vec<RecordingDescription> = Vec::new();

    let recordings = recordings::table
        .inner_join(performances::table.on(performances::recording.eq(recordings::id)))
        .inner_join(ensembles::table.on(ensembles::id.nullable().eq(performances::ensemble)))
        .filter(ensembles::id.eq(ensemble_id as i64))
        .select(recordings::table::all_columns())
        .load::<Recording>(conn)?;

    for recording in recordings {
        recording_descriptions.push(get_description_for_recording(conn, &recording)?);
    }

    Ok(recording_descriptions)
}

/// Get allavailable information on all recordings of a work.
pub fn get_recordings_for_work(conn: &DbConn, work_id: u32) -> Result<Vec<RecordingDescription>> {
    let mut recording_descriptions: Vec<RecordingDescription> = Vec::new();

    let recordings = recordings::table
        .filter(recordings::work.eq(work_id as i64))
        .load::<Recording>(conn)?;

    for recording in recordings {
        recording_descriptions.push(get_description_for_recording(conn, &recording)?);
    }

    Ok(recording_descriptions)
}

/// Delete an existing recording. This will fail if there are still references to this
/// recording from other tables that are not directly part of the recording data.
pub fn delete_recording(conn: &DbConn, id: u32) -> Result<()> {
    diesel::delete(recordings::table.filter(recordings::id.eq(id as i64))).execute(conn)?;
    Ok(())
}
