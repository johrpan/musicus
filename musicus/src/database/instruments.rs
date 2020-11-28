use super::schema::instruments;
use super::Database;
use anyhow::Result;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

/// An instrument or any other possible role within a recording.
#[derive(Serialize, Deserialize, Insertable, Queryable, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Instrument {
    pub id: String,
    pub name: String,
}

impl Database {
    /// Update an existing instrument or insert a new one.
    pub fn update_instrument(&self, instrument: Instrument) -> Result<()> {
        self.defer_foreign_keys()?;

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
