use super::schema::instruments;
use super::Database;
use anyhow::{Error, Result};
use diesel::prelude::*;
use diesel::{Insertable, Queryable};
use serde::{Deserialize, Serialize};
use std::convert::{TryFrom, TryInto};

/// Table row data for an instrument.
#[derive(Insertable, Queryable, Debug, Clone)]
#[table_name = "instruments"]
struct InstrumentRow {
    pub id: i64,
    pub name: String,
}

impl From<Instrument> for InstrumentRow {
    fn from(instrument: Instrument) -> Self {
        InstrumentRow {
            id: instrument.id as i64,
            name: instrument.name,
        }
    }
}

/// An instrument or any other possible role within a recording.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Instrument {
    pub id: u32,
    pub name: String,
}

impl TryFrom<InstrumentRow> for Instrument {
    type Error = Error;
    fn try_from(row: InstrumentRow) -> Result<Self> {
        let instrument = Instrument {
            id: row.id.try_into()?,
            name: row.name,
        };

        Ok(instrument)
    }
}

impl Database {
    /// Update an existing instrument or insert a new one.
    pub fn update_instrument(&self, instrument: Instrument) -> Result<()> {
        self.defer_foreign_keys()?;

        self.connection.transaction(|| {
            let row: InstrumentRow = instrument.into();
            diesel::replace_into(instruments::table)
                .values(row)
                .execute(&self.connection)
        })?;

        Ok(())
    }

    /// Get an existing instrument.
    pub fn get_instrument(&self, id: u32) -> Result<Option<Instrument>> {
        let row = instruments::table
            .filter(instruments::id.eq(id as i64))
            .load::<InstrumentRow>(&self.connection)?
            .first()
            .cloned();

        let instrument = match row {
            Some(row) => Some(row.try_into()?),
            None => None,
        };

        Ok(instrument)
    }

    /// Delete an existing instrument.
    pub fn delete_instrument(&self, id: u32) -> Result<()> {
        diesel::delete(instruments::table.filter(instruments::id.eq(id as i64)))
            .execute(&self.connection)?;

        Ok(())
    }

    /// Get all existing instruments.
    pub fn get_instruments(&self) -> Result<Vec<Instrument>> {
        let mut instruments = Vec::<Instrument>::new();

        let rows = instruments::table.load::<InstrumentRow>(&self.connection)?;
        for row in rows {
            instruments.push(row.try_into()?);
        }

        Ok(instruments)
    }
}
