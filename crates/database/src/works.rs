use chrono::{DateTime, TimeZone, Utc};
use diesel::{prelude::*, Insertable, Queryable};
use log::info;

use crate::{
    defer_foreign_keys, generate_id, get_instrument, get_person,
    schema::{instrumentations, work_parts, works},
    update_instrument, update_person, Error, Instrument, Person, Result,
};

/// Table row data for a work.
#[derive(Insertable, Queryable, Debug, Clone)]
#[diesel(table_name = works)]
struct WorkRow {
    pub id: String,
    pub composer: String,
    pub title: String,
    pub last_used: Option<i64>,
    pub last_played: Option<i64>,
}

impl From<Work> for WorkRow {
    fn from(work: Work) -> Self {
        WorkRow {
            id: work.id,
            composer: work.composer.id,
            title: work.title,
            last_used: Some(Utc::now().timestamp()),
            last_played: work.last_played.map(|t| t.timestamp()),
        }
    }
}

/// Definition that a work uses an instrument.
#[derive(Insertable, Queryable, Debug, Clone)]
#[diesel(table_name = instrumentations)]
struct InstrumentationRow {
    pub id: i64,
    pub work: String,
    pub instrument: String,
}

/// Table row data for a work part.
#[derive(Insertable, Queryable, Debug, Clone)]
#[diesel(table_name = work_parts)]
struct WorkPartRow {
    pub id: i64,
    pub work: String,
    pub part_index: i64,
    pub title: String,
}

/// A concrete work part that can be recorded.
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct WorkPart {
    pub title: String,
}

/// A specific work by a composer.
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct Work {
    pub id: String,
    pub title: String,
    pub composer: Person,
    pub instruments: Vec<Instrument>,
    pub parts: Vec<WorkPart>,
    pub last_used: Option<DateTime<Utc>>,
    pub last_played: Option<DateTime<Utc>>,
}

impl Work {
    pub fn new(
        id: String,
        title: String,
        composer: Person,
        instruments: Vec<Instrument>,
        parts: Vec<WorkPart>,
    ) -> Self {
        Self {
            id,
            title,
            composer,
            instruments,
            parts,
            last_used: Some(Utc::now()),
            last_played: None,
        }
    }

    /// Initialize a new work with a composer.
    pub fn from_composer(composer: Person) -> Self {
        Self {
            id: generate_id(),
            title: String::new(),
            composer,
            instruments: Vec::new(),
            parts: Vec::new(),
            last_used: Some(Utc::now()),
            last_played: None,
        }
    }

    /// Get a string including the composer and title of the work.
    // TODO: Replace with impl Display.
    pub fn get_title(&self) -> String {
        format!("{}: {}", self.composer.name_fl(), self.title)
    }
}

/// Update an existing work or insert a new one.
// TODO: Think about also inserting related items.
pub fn update_work(connection: &mut SqliteConnection, work: Work) -> Result<()> {
    info!("Updating work {:?}", work);
    defer_foreign_keys(connection)?;

    connection.transaction::<(), Error, _>(|connection| {
        let work_id = &work.id;
        delete_work(connection, work_id)?;

        // Add associated items from the server, if they don't already exist.

        if get_person(connection, &work.composer.id)?.is_none() {
            update_person(connection, work.composer.clone())?;
        }

        for instrument in &work.instruments {
            if get_instrument(connection, &instrument.id)?.is_none() {
                update_instrument(connection, instrument.clone())?;
            }
        }

        // Add the actual work.

        let row: WorkRow = work.clone().into();
        diesel::insert_into(works::table)
            .values(row)
            .execute(connection)?;

        let Work {
            instruments, parts, ..
        } = work;

        for instrument in instruments {
            let row = InstrumentationRow {
                id: rand::random(),
                work: work_id.to_string(),
                instrument: instrument.id,
            };

            diesel::insert_into(instrumentations::table)
                .values(row)
                .execute(connection)?;
        }

        for (index, part) in parts.into_iter().enumerate() {
            let row = WorkPartRow {
                id: rand::random(),
                work: work_id.to_string(),
                part_index: index as i64,
                title: part.title,
            };

            diesel::insert_into(work_parts::table)
                .values(row)
                .execute(connection)?;
        }

        Ok(())
    })?;

    Ok(())
}

/// Get an existing work.
pub fn get_work(connection: &mut SqliteConnection, id: &str) -> Result<Option<Work>> {
    let row = works::table
        .filter(works::id.eq(id))
        .load::<WorkRow>(connection)?
        .first()
        .cloned();

    let work = match row {
        Some(row) => Some(get_work_data(connection, row)?),
        None => None,
    };

    Ok(work)
}

/// Retrieve all available information on a work from related tables.
fn get_work_data(connection: &mut SqliteConnection, row: WorkRow) -> Result<Work> {
    let mut instruments: Vec<Instrument> = Vec::new();

    let instrumentations = instrumentations::table
        .filter(instrumentations::work.eq(&row.id))
        .load::<InstrumentationRow>(connection)?;

    for instrumentation in instrumentations {
        let id = instrumentation.instrument;
        instruments
            .push(get_instrument(connection, &id)?.ok_or(Error::MissingItem("instrument", id))?);
    }

    let mut parts: Vec<WorkPart> = Vec::new();

    let part_rows = work_parts::table
        .filter(work_parts::work.eq(&row.id))
        .load::<WorkPartRow>(connection)?;

    for part_row in part_rows {
        parts.push(WorkPart {
            title: part_row.title,
        });
    }

    let person_id = row.composer;
    let person =
        get_person(connection, &person_id)?.ok_or(Error::MissingItem("person", person_id))?;

    Ok(Work {
        id: row.id,
        composer: person,
        title: row.title,
        instruments,
        parts,
        last_used: row.last_used.map(|t| Utc.timestamp_opt(t, 0).unwrap()),
        last_played: row.last_played.map(|t| Utc.timestamp_opt(t, 0).unwrap()),
    })
}

/// Delete an existing work. This will fail if there are still other tables that relate to
/// this work except for the things that are part of the information on the work it
pub fn delete_work(connection: &mut SqliteConnection, id: &str) -> Result<()> {
    info!("Deleting work {}", id);
    diesel::delete(works::table.filter(works::id.eq(id))).execute(connection)?;
    Ok(())
}

/// Get all existing works by a composer and related information from other tables.
pub fn get_works(connection: &mut SqliteConnection, composer_id: &str) -> Result<Vec<Work>> {
    let mut works: Vec<Work> = Vec::new();

    let rows = works::table
        .filter(works::composer.eq(composer_id))
        .load::<WorkRow>(connection)?;

    for row in rows {
        works.push(get_work_data(connection, row)?);
    }

    Ok(works)
}
