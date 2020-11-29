use super::schema::{instrumentations, work_parts, work_sections, works};
use super::{get_instrument, get_person, update_instrument, update_person};
use super::{DbConn, Instrument, Person, User};
use crate::error::ServerError;
use anyhow::{anyhow, Error, Result};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;

/// A specific work by a composer.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Work {
    pub id: String,
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
    pub id: String,
    pub composer: String,
    pub title: String,
    pub created_by: String,
}

/// Table data for an instrumentation.
#[derive(Insertable, Queryable, Debug, Clone)]
#[table_name = "instrumentations"]
struct InstrumentationRow {
    pub id: i64,
    pub work: String,
    pub instrument: String,
}

/// Table data for a work part.
#[derive(Insertable, Queryable, Debug, Clone)]
#[table_name = "work_parts"]
struct WorkPartRow {
    pub id: i64,
    pub work: String,
    pub part_index: i64,
    pub title: String,
    pub composer: Option<String>,
}

/// Table data for a work section.
#[table_name = "work_sections"]
#[derive(Insertable, Queryable, Debug, Clone)]
struct WorkSectionRow {
    pub id: i64,
    pub work: String,
    pub title: String,
    pub before_index: i64,
}

/// Update an existing work or insert a new one. This will only succeed, if the user is allowed to
/// do that.
pub fn update_work(conn: &DbConn, work: &Work, user: &User) -> Result<()> {
    conn.transaction::<(), Error, _>(|| {
        let old_row = get_work_row(conn, &work.id)?;

        let allowed = match old_row {
            Some(row) => user.may_edit(&row.created_by),
            None => user.may_create(),
        };

        if allowed {
            let id = &work.id;

            // This will also delete rows from associated tables.
            diesel::delete(works::table)
                .filter(works::id.eq(id))
                .execute(conn)?;

            // Add associated items, if they don't already exist.

            if get_person(conn, &work.composer.id)?.is_none() {
                update_person(conn, &work.composer, &user)?;
            }

            for instrument in &work.instruments {
                if get_instrument(conn, &instrument.id)?.is_none() {
                    update_instrument(conn, instrument, &user)?;
                }
            }

            for part in &work.parts {
                if let Some(person) = &part.composer {
                    if get_person(conn, &person.id)?.is_none() {
                        update_person(conn, person, &user)?;
                    }
                }
            }

            // Add the actual work.

            let row = WorkRow {
                id: id.clone(),
                composer: work.composer.id.clone(),
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
                        work: id.clone(),
                        instrument: instrument.id.clone(),
                    })
                    .execute(conn)?;
            }

            for (index, part) in work.parts.iter().enumerate() {
                let row = WorkPartRow {
                    id: rand::random(),
                    work: id.clone(),
                    part_index: index.try_into()?,
                    title: part.title.clone(),
                    composer: part.composer.as_ref().map(|person| person.id.clone()),
                };

                diesel::insert_into(work_parts::table)
                    .values(row)
                    .execute(conn)?;
            }

            for section in &work.sections {
                let row = WorkSectionRow {
                    id: rand::random(),
                    work: id.clone(),
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
pub fn get_work(conn: &DbConn, id: &str) -> Result<Option<Work>> {
    let work = match get_work_row(conn, id)? {
        Some(row) => Some(get_description_for_work_row(conn, &row)?),
        None => None,
    };

    Ok(work)
}

/// Delete an existing work. This will fail if there are still other tables that relate to
/// this work except for the things that are part of the information on the work itself. Also,
/// this will only succeed, if the provided user is allowed to delete the work.
pub fn delete_work(conn: &DbConn, id: &str, user: &User) -> Result<()> {
    if user.may_delete() {
        diesel::delete(works::table.filter(works::id.eq(id))).execute(conn)?;
        Ok(())
    } else {
        Err(Error::new(ServerError::Forbidden))
    }
}

/// Get all existing works by a composer and related information from other tables.
pub fn get_works(conn: &DbConn, composer_id: &str) -> Result<Vec<Work>> {
    let mut works: Vec<Work> = Vec::new();

    let rows = works::table
        .filter(works::composer.eq(composer_id))
        .load::<WorkRow>(conn)?;

    for row in rows {
        works.push(get_description_for_work_row(conn, &row)?);
    }

    Ok(works)
}

/// Get an already existing work without related rows from other tables.
fn get_work_row(conn: &DbConn, id: &str) -> Result<Option<WorkRow>> {
    Ok(works::table
        .filter(works::id.eq(id))
        .load::<WorkRow>(conn)?
        .into_iter()
        .next())
}

/// Retrieve all available information on a work from related tables.
fn get_description_for_work_row(conn: &DbConn, row: &WorkRow) -> Result<Work> {
    let mut instruments: Vec<Instrument> = Vec::new();

    let instrumentations = instrumentations::table
        .filter(instrumentations::work.eq(&row.id))
        .load::<InstrumentationRow>(conn)?;

    for instrumentation in instrumentations {
        let id = instrumentation.instrument.clone();
        instruments
            .push(get_instrument(conn, &id)?.ok_or(anyhow!("No instrument with ID: {}", id))?);
    }

    let mut parts: Vec<WorkPart> = Vec::new();

    let part_rows = work_parts::table
        .filter(work_parts::work.eq(&row.id))
        .load::<WorkPartRow>(conn)?;

    for part_row in part_rows {
        parts.push(WorkPart {
            title: part_row.title,
            composer: match part_row.composer {
                Some(id) => {
                    Some(get_person(conn, &id)?.ok_or(anyhow!("No person with ID: {}", id))?)
                }
                None => None,
            },
        });
    }

    let mut sections: Vec<WorkSection> = Vec::new();

    let section_rows = work_sections::table
        .filter(work_sections::work.eq(&row.id))
        .load::<WorkSectionRow>(conn)?;

    for section in section_rows {
        sections.push(WorkSection {
            title: section.title,
            before_index: section.before_index,
        });
    }

    let id = &row.composer;
    let composer = get_person(conn, id)?.ok_or(anyhow!("No person with ID: {}", id))?;

    Ok(Work {
        id: row.id.clone(),
        composer,
        title: row.title.clone(),
        instruments,
        parts,
        sections,
    })
}
