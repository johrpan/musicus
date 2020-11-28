use super::schema::instruments;
use super::{DbConn, User};
use crate::error::ServerError;
use anyhow::{Error, Result};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

/// A instrument as represented within the API.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Instrument {
    pub id: String,
    pub name: String,
}

/// A instrument as represented in the database.
#[derive(Insertable, Queryable, AsChangeset, Debug, Clone)]
#[table_name = "instruments"]
struct InstrumentRow {
    pub id: String,
    pub name: String,
    pub created_by: String,
}

impl From<InstrumentRow> for Instrument {
    fn from(row: InstrumentRow) -> Instrument {
        Instrument {
            id: row.id,
            name: row.name,
        }
    }
}

/// Update an existing instrument or insert a new one. This will only work, if the provided user is
/// allowed to do that.
pub fn update_instrument(conn: &DbConn, instrument: &Instrument, user: &User) -> Result<()> {
    let old_row = get_instrument_row(conn, &instrument.id)?;

    let allowed = match old_row {
        Some(row) => user.may_edit(&row.created_by),
        None => user.may_create(),
    };

    if allowed {
        let new_row = InstrumentRow {
            id: instrument.id.clone(),
            name: instrument.name.clone(),
            created_by: user.username.clone(),
        };

        diesel::insert_into(instruments::table)
            .values(&new_row)
            .on_conflict(instruments::id)
            .do_update()
            .set(&new_row)
            .execute(conn)?;

        Ok(())
    } else {
        Err(Error::new(ServerError::Forbidden))
    }
}

/// Get an existing instrument.
pub fn get_instrument(conn: &DbConn, id: &str) -> Result<Option<Instrument>> {
    let row = get_instrument_row(conn, id)?;
    let instrument = row.map(|row| row.into());

    Ok(instrument)
}

/// Delete an existing instrument. This will only work if the provided user is allowed to do that.
pub fn delete_instrument(conn: &DbConn, id: &str, user: &User) -> Result<()> {
    if user.may_delete() {
        diesel::delete(instruments::table.filter(instruments::id.eq(id))).execute(conn)?;
        Ok(())
    } else {
        Err(Error::new(ServerError::Forbidden))
    }
}

/// Get all existing instruments.
pub fn get_instruments(conn: &DbConn) -> Result<Vec<Instrument>> {
    let rows = instruments::table.load::<InstrumentRow>(conn)?;
    let instruments: Vec<Instrument> = rows.into_iter().map(|row| row.into()).collect();

    Ok(instruments)
}

/// Get a instrument row if it exists.
fn get_instrument_row(conn: &DbConn, id: &str) -> Result<Option<InstrumentRow>> {
    let row = instruments::table
        .filter(instruments::id.eq(id))
        .load::<InstrumentRow>(conn)?
        .into_iter()
        .next();

    Ok(row)
}
