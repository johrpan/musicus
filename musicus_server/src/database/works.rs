use super::schema::{instrumentations, instruments, persons, work_parts, work_sections, works};
use super::{get_person, DbConn, Instrument, Person};
use anyhow::{anyhow, Error, Result};
use diesel::prelude::*;
use diesel::{Insertable, Queryable};
use serde::{Deserialize, Serialize};
use std::convert::TryInto;

/// A composition by a composer.
#[derive(Insertable, Queryable, Debug, Clone)]
pub struct Work {
    pub id: i64,
    pub composer: i64,
    pub title: String,
    pub created_by: String,
}

/// Definition that a work uses an instrument.
#[derive(Insertable, Queryable, Debug, Clone)]
pub struct Instrumentation {
    pub id: i64,
    pub work: i64,
    pub instrument: i64,
}

/// A concrete work part that can be recorded.
#[derive(Insertable, Queryable, Debug, Clone)]
pub struct WorkPart {
    pub id: i64,
    pub work: i64,
    pub part_index: i64,
    pub title: String,
    pub composer: Option<i64>,
}

/// A heading between work parts.
#[derive(Insertable, Queryable, Debug, Clone)]
pub struct WorkSection {
    pub id: i64,
    pub work: i64,
    pub title: String,
    pub before_index: i64,
}
/// A structure for collecting all available information on a work part.
#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WorkPartDescription {
    pub title: String,
    pub composer: Option<Person>,
}

/// A structure for collecting all available information on a work section.
#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WorkSectionDescription {
    pub title: String,
    pub before_index: i64,
}

/// A structure for collecting all available information on a work.
#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WorkDescription {
    pub id: i64,
    pub title: String,
    pub composer: Person,
    pub instruments: Vec<Instrument>,
    pub parts: Vec<WorkPartDescription>,
    pub sections: Vec<WorkSectionDescription>,
}

/// A structure representing data on a work part.
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WorkPartInsertion {
    pub title: String,
    pub composer: Option<i64>,
}

/// A structure representing data on a work section.
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WorkSectionInsertion {
    pub title: String,
    pub before_index: i64,
}

/// A structure representing data on a work.
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WorkInsertion {
    pub composer: i64,
    pub title: String,
    pub instruments: Vec<i64>,
    pub parts: Vec<WorkPartInsertion>,
    pub sections: Vec<WorkSectionInsertion>,
}

/// Insert a new work.
pub fn insert_work(conn: &DbConn, id: u32, data: &WorkInsertion, created_by: &str) -> Result<()> {
    conn.transaction::<(), Error, _>(|| {
        let id = id as i64;

        diesel::insert_into(works::table)
            .values(Work {
                id,
                composer: data.composer.clone(),
                title: data.title.clone(),
                created_by: created_by.to_string(),
            })
            .execute(conn)?;

        insert_work_data(conn, id, data)?;

        Ok(())
    })?;

    Ok(())
}

/// Update an existing work.
pub fn update_work(conn: &DbConn, id: u32, data: &WorkInsertion) -> Result<()> {
    conn.transaction::<(), Error, _>(|| {
        let id = id as i64;

        diesel::delete(instrumentations::table)
            .filter(instrumentations::work.eq(id))
            .execute(conn)?;

        diesel::delete(work_parts::table)
            .filter(work_parts::work.eq(id))
            .execute(conn)?;

        diesel::delete(work_sections::table)
            .filter(work_sections::work.eq(id))
            .execute(conn)?;

        diesel::update(works::table)
            .filter(works::id.eq(id))
            .set((
                works::composer.eq(data.composer),
                works::title.eq(data.title.clone()),
            ))
            .execute(conn)?;

        insert_work_data(conn, id, data)?;

        Ok(())
    })?;

    Ok(())
}

/// Helper method to populate tables related to a work.
fn insert_work_data(conn: &DbConn, id: i64, data: &WorkInsertion) -> Result<()> {
    for instrument in &data.instruments {
        diesel::insert_into(instrumentations::table)
            .values(Instrumentation {
                id: rand::random(),
                work: id,
                instrument: *instrument,
            })
            .execute(conn)?;
    }

    for (index, part) in data.parts.iter().enumerate() {
        let part = WorkPart {
            id: rand::random(),
            work: id,
            part_index: index.try_into()?,
            title: part.title.clone(),
            composer: part.composer,
        };

        diesel::insert_into(work_parts::table)
            .values(part)
            .execute(conn)?;
    }

    for section in &data.sections {
        let section = WorkSection {
            id: rand::random(),
            work: id,
            title: section.title.clone(),
            before_index: section.before_index,
        };

        diesel::insert_into(work_sections::table)
            .values(section)
            .execute(conn)?;
    }

    Ok(())
}

/// Get an already existing work without related rows from other tables.
fn get_work(conn: &DbConn, id: u32) -> Result<Option<Work>> {
    Ok(works::table
        .filter(works::id.eq(id as i64))
        .load::<Work>(conn)?
        .first()
        .cloned())
}

/// Retrieve all available information on a work from related tables.
fn get_description_for_work(conn: &DbConn, work: &Work) -> Result<WorkDescription> {
    let mut instruments: Vec<Instrument> = Vec::new();

    let instrumentations = instrumentations::table
        .filter(instrumentations::work.eq(work.id))
        .load::<Instrumentation>(conn)?;

    for instrumentation in instrumentations {
        instruments.push(
            instruments::table
                .filter(instruments::id.eq(instrumentation.instrument))
                .load::<Instrument>(conn)?
                .first()
                .cloned()
                .ok_or(anyhow!(
                    "No instrument with ID: {}",
                    instrumentation.instrument
                ))?,
        );
    }

    let mut part_descriptions: Vec<WorkPartDescription> = Vec::new();

    let work_parts = work_parts::table
        .filter(work_parts::work.eq(work.id))
        .load::<WorkPart>(conn)?;

    for work_part in work_parts {
        part_descriptions.push(WorkPartDescription {
            title: work_part.title,
            composer: match work_part.composer {
                Some(composer) => Some(
                    persons::table
                        .filter(persons::id.eq(composer))
                        .load::<Person>(conn)?
                        .first()
                        .cloned()
                        .ok_or(anyhow!("No person with ID: {}", composer))?,
                ),
                None => None,
            },
        });
    }

    let mut section_descriptions: Vec<WorkSectionDescription> = Vec::new();

    let sections = work_sections::table
        .filter(work_sections::work.eq(work.id))
        .load::<WorkSection>(conn)?;

    for section in sections {
        section_descriptions.push(WorkSectionDescription {
            title: section.title,
            before_index: section.before_index,
        });
    }

    let person_id = work.composer.try_into()?;
    let person =
        get_person(conn, person_id)?.ok_or(anyhow!("Person doesn't exist: {}", person_id))?;

    Ok(WorkDescription {
        id: work.id,
        composer: person,
        title: work.title.clone(),
        instruments,
        parts: part_descriptions,
        sections: section_descriptions,
    })
}

/// Get an existing work and all available information from related tables.
pub fn get_work_description(conn: &DbConn, id: u32) -> Result<Option<WorkDescription>> {
    let work_description = match get_work(conn, id)? {
        Some(work) => Some(get_description_for_work(conn, &work)?),
        None => None,
    };

    Ok(work_description)
}

/// Delete an existing work. This will fail if there are still other tables that relate to
/// this work except for the things that are part of the information on the work it
pub fn delete_work(conn: &DbConn, id: u32) -> Result<()> {
    diesel::delete(works::table.filter(works::id.eq(id as i64))).execute(conn)?;
    Ok(())
}

/// Get all existing works by a composer and related information from other tables.
pub fn get_work_descriptions(conn: &DbConn, composer_id: u32) -> Result<Vec<WorkDescription>> {
    let mut work_descriptions: Vec<WorkDescription> = Vec::new();

    let works = works::table
        .filter(works::composer.eq(composer_id as i64))
        .load::<Work>(conn)?;

    for work in works {
        work_descriptions.push(get_description_for_work(conn, &work)?);
    }

    Ok(work_descriptions)
}
