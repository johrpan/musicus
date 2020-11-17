use super::schema::persons;
use super::Database;
use anyhow::{Error, Result};
use diesel::prelude::*;
use diesel::{Insertable, Queryable};
use serde::{Deserialize, Serialize};
use std::convert::{TryFrom, TryInto};

/// Database table data for a person.
#[derive(Insertable, Queryable, Debug, Clone)]
#[table_name = "persons"]
struct PersonRow {
    pub id: i64,
    pub first_name: String,
    pub last_name: String,
}

impl From<Person> for PersonRow {
    fn from(person: Person) -> Self {
        PersonRow {
            id: person.id as i64,
            first_name: person.first_name,
            last_name: person.last_name,
        }
    }
}

/// A person that is a composer, an interpret or both.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    pub id: u32,
    pub first_name: String,
    pub last_name: String,
}

impl TryFrom<PersonRow> for Person {
    type Error = Error;
    fn try_from(row: PersonRow) -> Result<Self> {
        let person = Person {
            id: row.id.try_into()?,
            first_name: row.first_name,
            last_name: row.last_name,
        };

        Ok(person)
    }
}

impl Person {
    /// Get the full name in the form "First Last".
    pub fn name_fl(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
    }

    /// Get the full name in the form "Last, First".
    pub fn name_lf(&self) -> String {
        format!("{}, {}", self.last_name, self.first_name)
    }
}

impl Database {
    /// Update an existing person or insert a new one.
    pub fn update_person(&self, person: Person) -> Result<()> {
        self.defer_foreign_keys()?;

        self.connection.transaction(|| {
            let row: PersonRow = person.into();
            diesel::replace_into(persons::table)
                .values(row)
                .execute(&self.connection)
        })?;

        Ok(())
    }

    /// Get an existing person.
    pub fn get_person(&self, id: u32) -> Result<Option<Person>> {
        let row = persons::table
            .filter(persons::id.eq(id as i64))
            .load::<PersonRow>(&self.connection)?
            .first()
            .cloned();

        let person = match row {
            Some(row) => Some(row.try_into()?),
            None => None,
        };

        Ok(person)
    }

    /// Delete an existing person.
    pub fn delete_person(&self, id: u32) -> Result<()> {
        diesel::delete(persons::table.filter(persons::id.eq(id as i64)))
            .execute(&self.connection)?;
        Ok(())
    }

    /// Get all existing persons.
    pub fn get_persons(&self) -> Result<Vec<Person>> {
        let mut persons = Vec::<Person>::new();

        let rows = persons::table.load::<PersonRow>(&self.connection)?;
        for row in rows {
            persons.push(row.try_into()?);
        }

        Ok(persons)
    }
}
