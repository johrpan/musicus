use chrono::Utc;
use diesel::prelude::*;
use log::info;

use crate::{defer_foreign_keys, schema::persons, Result};

/// A person that is a composer, an interpret or both.
#[derive(Insertable, Queryable, PartialEq, Eq, Hash, Debug, Clone)]
pub struct Person {
    pub id: String,
    pub first_name: String,
    pub last_name: String,
    pub last_used: Option<i64>,
    pub last_played: Option<i64>,
}

impl Person {
    pub fn new(id: String, first_name: String, last_name: String) -> Self {
        Self {
            id,
            first_name,
            last_name,
            last_used: Some(Utc::now().timestamp()),
            last_played: None,
        }
    }

    /// Get the full name in the form "First Last".
    pub fn name_fl(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
    }

    /// Get the full name in the form "Last, First".
    pub fn name_lf(&self) -> String {
        format!("{}, {}", self.last_name, self.first_name)
    }
}
/// Update an existing person or insert a new one.
pub fn update_person(connection: &mut SqliteConnection, mut person: Person) -> Result<()> {
    info!("Updating person {:?}", person);
    defer_foreign_keys(connection)?;

    person.last_used = Some(Utc::now().timestamp());

    connection.transaction(|connection| {
        diesel::replace_into(persons::table)
            .values(person)
            .execute(connection)
    })?;

    Ok(())
}

/// Get an existing person.
pub fn get_person(connection: &mut SqliteConnection, id: &str) -> Result<Option<Person>> {
    let person = persons::table
        .filter(persons::id.eq(id))
        .load::<Person>(connection)?
        .into_iter()
        .next();

    Ok(person)
}

/// Delete an existing person.
pub fn delete_person(connection: &mut SqliteConnection, id: &str) -> Result<()> {
    info!("Deleting person {}", id);
    diesel::delete(persons::table.filter(persons::id.eq(id))).execute(connection)?;
    Ok(())
}

/// Get all existing persons.
pub fn get_persons(connection: &mut SqliteConnection) -> Result<Vec<Person>> {
    let persons = persons::table.load::<Person>(connection)?;

    Ok(persons)
}

/// Get recently used persons.
pub fn get_recent_persons(connection: &mut SqliteConnection) -> Result<Vec<Person>> {
    let persons = persons::table
        .order(persons::last_used.desc())
        .load::<Person>(connection)?;

    Ok(persons)
}
