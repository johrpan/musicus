use super::schema::persons;
use super::{Database, Result};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

/// A person that is a composer, an interpret or both.
#[derive(Serialize, Deserialize, Insertable, Queryable, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    pub id: String,
    pub first_name: String,
    pub last_name: String,
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
            diesel::replace_into(persons::table)
                .values(person)
                .execute(&self.connection)
        })?;

        Ok(())
    }

    /// Get an existing person.
    pub fn get_person(&self, id: &str) -> Result<Option<Person>> {
        let person = persons::table
            .filter(persons::id.eq(id))
            .load::<Person>(&self.connection)?
            .into_iter()
            .next();

        Ok(person)
    }

    /// Delete an existing person.
    pub fn delete_person(&self, id: &str) -> Result<()> {
        diesel::delete(persons::table.filter(persons::id.eq(id))).execute(&self.connection)?;

        Ok(())
    }

    /// Get all existing persons.
    pub fn get_persons(&self) -> Result<Vec<Person>> {
        let persons = persons::table.load::<Person>(&self.connection)?;

        Ok(persons)
    }
}
