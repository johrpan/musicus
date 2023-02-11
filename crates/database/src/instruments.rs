use chrono::Utc;
use diesel::prelude::*;
use log::info;

use crate::{defer_foreign_keys, schema::instruments, Result};

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

/// Update an existing instrument or insert a new one.
pub fn update_instrument(
    connection: &mut SqliteConnection,
    mut instrument: Instrument,
) -> Result<()> {
    info!("Updating instrument {:?}", instrument);
    defer_foreign_keys(connection)?;

    instrument.last_used = Some(Utc::now().timestamp());

    connection.transaction(|connection| {
        diesel::replace_into(instruments::table)
            .values(instrument)
            .execute(connection)
    })?;

    Ok(())
}

/// Get an existing instrument.
pub fn get_instrument(connection: &mut SqliteConnection, id: &str) -> Result<Option<Instrument>> {
    let instrument = instruments::table
        .filter(instruments::id.eq(id))
        .load::<Instrument>(connection)?
        .into_iter()
        .next();

    Ok(instrument)
}

/// Delete an existing instrument.
pub fn delete_instrument(connection: &mut SqliteConnection, id: &str) -> Result<()> {
    info!("Deleting instrument {}", id);
    diesel::delete(instruments::table.filter(instruments::id.eq(id))).execute(connection)?;

    Ok(())
}

/// Get all existing instruments.
pub fn get_instruments(connection: &mut SqliteConnection) -> Result<Vec<Instrument>> {
    let instruments = instruments::table.load::<Instrument>(connection)?;

    Ok(instruments)
}

/// Get recently used instruments.
pub fn get_recent_instruments(connection: &mut SqliteConnection) -> Result<Vec<Instrument>> {
    let instruments = instruments::table
        .order(instruments::last_used.desc())
        .load::<Instrument>(connection)?;

    Ok(instruments)
}
