use super::schema::persons;
use super::{Database, Result};
use chrono::Utc;
use diesel::prelude::*;
use log::info;

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

impl Database {
    /// Update an existing person or insert a new one.
    pub fn update_person(&self, mut person: Person) -> Result<()> {
        info!("Updating person {:?}", person);
        self.defer_foreign_keys()?;

        person.last_used = Some(Utc::now().timestamp());

        self.connection.lock().unwrap().transaction(|connection| {
            diesel::replace_into(persons::table)
                .values(person)
                .execute(connection)
        })?;

        Ok(())
    }

    /// Get an existing person.
    pub fn get_person(&self, id: &str) -> Result<Option<Person>> {
        let person = persons::table
            .filter(persons::id.eq(id))
            .load::<Person>(&mut *self.connection.lock().unwrap())?
            .into_iter()
            .next();

        Ok(person)
    }

    /// Delete an existing person.
    pub fn delete_person(&self, id: &str) -> Result<()> {
        info!("Deleting person {}", id);
        diesel::delete(persons::table.filter(persons::id.eq(id)))
            .execute(&mut *self.connection.lock().unwrap())?;
        Ok(())
    }

    /// Get all existing persons.
    pub fn get_persons(&self) -> Result<Vec<Person>> {
        let persons = persons::table.load::<Person>(&mut *self.connection.lock().unwrap())?;

        Ok(persons)
    }

    /// Get recently used persons.
    pub fn get_recent_persons(&self) -> Result<Vec<Person>> {
        let persons = persons::table
            .order(persons::last_used.desc())
            .load::<Person>(&mut *self.connection.lock().unwrap())?;

        Ok(persons)
    }
}
