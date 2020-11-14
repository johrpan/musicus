use super::schema::persons;
use super::DbConn;
use anyhow::Result;
use diesel::prelude::*;
use diesel::{Insertable, Queryable};
use serde::{Deserialize, Serialize};

/// A person that is a composer, an interpret or both.
#[derive(Insertable, Queryable, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    pub id: i64,
    pub first_name: String,
    pub last_name: String,

    #[serde(skip)]
    pub created_by: String,
}

/// A structure representing data on a person.
#[derive(AsChangeset, Deserialize, Debug, Clone)]
#[table_name = "persons"]
#[serde(rename_all = "camelCase")]
pub struct PersonInsertion {
    pub first_name: String,
    pub last_name: String,
}

/// Insert a new person.
pub fn insert_person(
    conn: &DbConn,
    id: u32,
    data: &PersonInsertion,
    created_by: &str,
) -> Result<()> {
    let person = Person {
        id: id as i64,
        first_name: data.first_name.clone(),
        last_name: data.last_name.clone(),
        created_by: created_by.to_string(),
    };

    diesel::insert_into(persons::table)
        .values(person)
        .execute(conn)?;

    Ok(())
}

/// Update an existing person.
pub fn update_person(conn: &DbConn, id: u32, data: &PersonInsertion) -> Result<()> {
    diesel::update(persons::table)
        .filter(persons::id.eq(id as i64))
        .set(data)
        .execute(conn)?;

    Ok(())
}

/// Get an existing person.
pub fn get_person(conn: &DbConn, id: u32) -> Result<Option<Person>> {
    Ok(persons::table
        .filter(persons::id.eq(id as i64))
        .load::<Person>(conn)?
        .first()
        .cloned())
}

/// Delete an existing person.
pub fn delete_person(conn: &DbConn, id: u32) -> Result<()> {
    diesel::delete(persons::table.filter(persons::id.eq(id as i64))).execute(conn)?;
    Ok(())
}

/// Get all existing persons.
pub fn get_persons(conn: &DbConn) -> Result<Vec<Person>> {
    Ok(persons::table.load::<Person>(conn)?)
}
