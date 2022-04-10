use super::schema::instruments;
use super::{Database, Result};
use chrono::Utc;
use diesel::prelude::*;
use log::info;

/// An instrument or any other possible role within a recording.
#[derive(Insertable, Queryable, PartialEq, Eq, Hash, Debug, Clone)]
pub struct Instrument {
    pub id: String,
    pub name: String,
    pub last_used: Option<i64>,
    pub last_played: Option<i64>,
}

impl Instrument {
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            last_used: Some(Utc::now().timestamp()),
            last_played: None,
        }
    }
}

impl Database {
    /// Update an existing instrument or insert a new one.
    pub fn update_instrument(&self, mut instrument: Instrument) -> Result<()> {
        info!("Updating instrument {:?}", instrument);
        self.defer_foreign_keys()?;

        instrument.last_used = Some(Utc::now().timestamp());

        self.connection.transaction(|| {
            diesel::replace_into(instruments::table)
                .values(instrument)
                .execute(&self.connection)
        })?;

        Ok(())
    }

    /// Get an existing instrument.
    pub fn get_instrument(&self, id: &str) -> Result<Option<Instrument>> {
        let instrument = instruments::table
            .filter(instruments::id.eq(id))
            .load::<Instrument>(&self.connection)?
            .into_iter()
            .next();

        Ok(instrument)
    }

    /// Delete an existing instrument.
    pub fn delete_instrument(&self, id: &str) -> Result<()> {
        info!("Deleting instrument {}", id);
        diesel::delete(instruments::table.filter(instruments::id.eq(id)))
            .execute(&self.connection)?;

        Ok(())
    }

    /// Get all existing instruments.
    pub fn get_instruments(&self) -> Result<Vec<Instrument>> {
        let instruments = instruments::table.load::<Instrument>(&self.connection)?;

        Ok(instruments)
    }
}
