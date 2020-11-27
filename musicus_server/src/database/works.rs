use super::schema::{instrumentations, work_parts, work_sections, works};
use super::{get_instrument, get_person, DbConn, Instrument, Person, User};
use crate::error::ServerError;
use anyhow::{anyhow, Error, Result};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;

/// A specific work by a composer.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Work {
    pub id: u32,
    pub title: String,
    pub composer: Person,
    pub instruments: Vec<Instrument>,
    pub parts: Vec<WorkPart>,
    pub sections: Vec<WorkSection>,
}

/// A playable part of a work.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WorkPart {
    pub title: String,
    pub composer: Option<Person>,
}

/// A heading within the work structure.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WorkSection {
    pub title: String,
    pub before_index: i64,
}

/// Table data for a work.
#[derive(Insertable, Queryable, Debug, Clone)]
#[table_name = "works"]
struct WorkRow {
    pub id: i64,
    pub composer: i64,
    pub title: String,
    pub created_by: String,
}

/// Table data for an instrumentation.
#[derive(Insertable, Queryable, Debug, Clone)]
#[table_name = "instrumentations"]
struct InstrumentationRow {
    pub id: i64,
    pub work: i64,
    pub instrument: i64,
}

/// Table data for a work part.
#[derive(Insertable, Queryable, Debug, Clone)]
#[table_name = "work_parts"]
struct WorkPartRow {
    pub id: i64,
    pub work: i64,
    pub part_index: i64,
    pub title: String,
    pub composer: Option<i64>,
}

/// Table data for a work section.
#[table_name = "work_sections"]
#[derive(Insertable, Queryable, Debug, Clone)]
struct WorkSectionRow {
    pub id: i64,
    pub work: i64,
    pub title: String,
    pub before_index: i64,
}

/// Update an existing work or insert a new one. This will only succeed, if the user is allowed to
/// do that.
// TODO: Also add newly created associated items.
pub fn update_work(conn: &DbConn, work: &Work, user: &User) -> Result<()> {
    conn.transaction::<(), Error, _>(|| {
        let old_row = get_work_row(conn, work.id)?;

        let allowed = match old_row {
            Some(row) => user.may_edit(&row.created_by),
            None => user.may_create(),
        };

        if allowed {
            let id = work.id as i64;

            // This will also delete rows from associated tables.
            diesel::delete(works::table)
                .filter(works::id.eq(id))
                .execute(conn)?;

            let row = WorkRow {
                id,
                composer: work.composer.id as i64,
                title: work.title.clone(),
                created_by: user.username.clone(),
            };

            diesel::insert_into(works::table)
                .values(row)
                .execute(conn)?;

            for instrument in &work.instruments {
                diesel::insert_into(instrumentations::table)
                    .values(InstrumentationRow {
                        id: rand::random(),
                        work: id,
                        instrument: instrument.id as i64,
                    })
                    .execute(conn)?;
            }

            for (index, part) in work.parts.iter().enumerate() {
                let row = WorkPartRow {
                    id: rand::random(),
                    work: id,
                    part_index: index.try_into()?,
                    title: part.title.clone(),
                    composer: part.composer.as_ref().map(|person| person.id as i64),
                };

                diesel::insert_into(work_parts::table)
                    .values(row)
                    .execute(conn)?;
            }

            for section in &work.sections {
                let row = WorkSectionRow {
                    id: rand::random(),
                    work: id,
                    title: section.title.clone(),
                    before_index: section.before_index,
                };

                diesel::insert_into(work_sections::table)
                    .values(row)
                    .execute(conn)?;
            }

            Ok(())
        } else {
            Err(Error::new(ServerError::Forbidden))
        }
    })?;

    Ok(())
}

/// Get an existing work and all available information from related tables.
pub fn get_work(conn: &DbConn, id: u32) -> Result<Option<Work>> {
    let work = match get_work_row(conn, id)? {
        Some(row) => Some(get_description_for_work_row(conn, &row)?),
        None => None,
    };

    Ok(work)
}

/// Delete an existing work. This will fail if there are still other tables that relate to
/// this work except for the things that are part of the information on the work itself. Also,
/// this will only succeed, if the provided user is allowed to delete the work.
pub fn delete_work(conn: &DbConn, id: u32, user: &User) -> Result<()> {
    if user.may_delete() {
        diesel::delete(works::table.filter(works::id.eq(id as i64))).execute(conn)?;
        Ok(())
    } else {
        Err(Error::new(ServerError::Forbidden))
    }
}

/// Get all existing works by a composer and related information from other tables.
pub fn get_works(conn: &DbConn, composer_id: u32) -> Result<Vec<Work>> {
    let mut works: Vec<Work> = Vec::new();

    let rows = works::table
        .filter(works::composer.eq(composer_id as i64))
        .load::<WorkRow>(conn)?;

    for row in rows {
        works.push(get_description_for_work_row(conn, &row)?);
    }

    Ok(works)
}

/// Get an already existing work without related rows from other tables.
fn get_work_row(conn: &DbConn, id: u32) -> Result<Option<WorkRow>> {
    Ok(works::table
        .filter(works::id.eq(id as i64))
        .load::<WorkRow>(conn)?
        .into_iter()
        .next())
}

/// Retrieve all available information on a work from related tables.
fn get_description_for_work_row(conn: &DbConn, row: &WorkRow) -> Result<Work> {
    let mut instruments: Vec<Instrument> = Vec::new();

    let instrumentations = instrumentations::table
        .filter(instrumentations::work.eq(row.id))
        .load::<InstrumentationRow>(conn)?;

    for instrumentation in instrumentations {
        let id = instrumentation.instrument as u32;
        instruments
            .push(get_instrument(conn, id)?.ok_or(anyhow!("No instrument with ID: {}", id))?);
    }

    let mut parts: Vec<WorkPart> = Vec::new();

    let part_rows = work_parts::table
        .filter(work_parts::work.eq(row.id))
        .load::<WorkPartRow>(conn)?;

    for part_row in part_rows {
        parts.push(WorkPart {
            title: part_row.title,
            composer: match part_row.composer {
                Some(id) => {
                    let id = id as u32;
                    Some(get_person(conn, id)?.ok_or(anyhow!("No person with ID: {}", id))?)
                }
                None => None,
            },
        });
    }

    let mut sections: Vec<WorkSection> = Vec::new();

    let section_rows = work_sections::table
        .filter(work_sections::work.eq(row.id))
        .load::<WorkSectionRow>(conn)?;

    for section in section_rows {
        sections.push(WorkSection {
            title: section.title,
            before_index: section.before_index,
        });
    }

    let id = row.composer as u32;
    let composer = get_person(conn, id)?.ok_or(anyhow!("No person with ID: {}", id))?;

    Ok(Work {
        id: row.id as u32,
        composer,
        title: row.title.clone(),
        instruments,
        parts,
        sections,
    })
}
