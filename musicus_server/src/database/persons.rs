use super::schema::persons;
use super::{DbConn, User};
use crate::error::ServerError;
use anyhow::{Error, Result};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

/// A person as represented within the API.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    pub id: u32,
    pub first_name: String,
    pub last_name: String,
}

/// A person as represented in the database.
#[derive(Insertable, Queryable, AsChangeset, Debug, Clone)]
#[table_name = "persons"]
struct PersonRow {
    pub id: i64,
    pub first_name: String,
    pub last_name: String,
    pub created_by: String,
}

impl From<PersonRow> for Person {
    fn from(row: PersonRow) -> Person {
        Person {
            id: row.id as u32,
            first_name: row.first_name,
            last_name: row.last_name,
        }
    }
}

/// Update an existing person or insert a new one. This will only work, if the provided user is
/// allowed to do that.
pub fn update_person(conn: &DbConn, person: &Person, user: &User) -> Result<()> {
    let old_row = get_person_row(conn, person.id)?;

    let allowed = match old_row {
        Some(row) => user.may_edit(&row.created_by),
        None => user.may_create(),
    };

    if allowed {
        let new_row = PersonRow {
            id: person.id as i64,
            first_name: person.first_name.clone(),
            last_name: person.last_name.clone(),
            created_by: user.username.clone(),
        };

        diesel::insert_into(persons::table)
            .values(&new_row)
            .on_conflict(persons::id)
            .do_update()
            .set(&new_row)
            .execute(conn)?;

        Ok(())
    } else {
        Err(Error::new(ServerError::Forbidden))
    }
}

/// Get an existing person.
pub fn get_person(conn: &DbConn, id: u32) -> Result<Option<Person>> {
    let row = get_person_row(conn, id)?;
    let person = row.map(|row| row.into());

    Ok(person)
}

/// Delete an existing person. This will only work if the provided user is allowed to do that.
pub fn delete_person(conn: &DbConn, id: u32, user: &User) -> Result<()> {
    if user.may_delete() {
        diesel::delete(persons::table.filter(persons::id.eq(id as i64))).execute(conn)?;
        Ok(())
    } else {
        Err(Error::new(ServerError::Forbidden))
    }
}

/// Get all existing persons.
pub fn get_persons(conn: &DbConn) -> Result<Vec<Person>> {
    let rows = persons::table.load::<PersonRow>(conn)?;
    let persons: Vec<Person> = rows.into_iter().map(|row| row.into()).collect();

    Ok(persons)
}

/// Get a person row if it exists.
fn get_person_row(conn: &DbConn, id: u32) -> Result<Option<PersonRow>> {
    let row = persons::table
        .filter(persons::id.eq(id as i64))
        .load::<PersonRow>(conn)?
        .into_iter()
        .next();

    Ok(row)
}
