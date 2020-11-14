use super::schema::instruments;
use super::DbConn;
use anyhow::Result;
use diesel::prelude::*;
use diesel::{Insertable, Queryable};
use serde::{Deserialize, Serialize};

/// An instrument or any other possible role within a recording.
#[derive(Insertable, Queryable, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Instrument {
    pub id: i64,
    pub name: String,

    #[serde(skip)]
    pub created_by: String,
}

/// A structure representing data on an instrument.
#[derive(AsChangeset, Deserialize, Debug, Clone)]
#[table_name = "instruments"]
#[serde(rename_all = "camelCase")]
pub struct InstrumentInsertion {
    pub name: String,
}

/// Insert a new instrument.
pub fn insert_instrument(
    conn: &DbConn,
    id: u32,
    data: &InstrumentInsertion,
    created_by: &str,
) -> Result<()> {
    let instrument = Instrument {
        id: id as i64,
        name: data.name.clone(),
        created_by: created_by.to_string(),
    };

    diesel::insert_into(instruments::table)
        .values(instrument)
        .execute(conn)?;

    Ok(())
}

/// Update an existing instrument.
pub fn update_instrument(conn: &DbConn, id: u32, data: &InstrumentInsertion) -> Result<()> {
    diesel::update(instruments::table)
        .filter(instruments::id.eq(id as i64))
        .set(data)
        .execute(conn)?;

    Ok(())
}

/// Get an existing instrument.
pub fn get_instrument(conn: &DbConn, id: u32) -> Result<Option<Instrument>> {
    Ok(instruments::table
        .filter(instruments::id.eq(id as i64))
        .load::<Instrument>(conn)?
        .first()
        .cloned())
}

/// Delete an existing instrument.
pub fn delete_instrument(conn: &DbConn, id: u32) -> Result<()> {
    diesel::delete(instruments::table.filter(instruments::id.eq(id as i64))).execute(conn)?;
    Ok(())
}

/// Get all existing instruments.
pub fn get_instruments(conn: &DbConn) -> Result<Vec<Instrument>> {
    Ok(instruments::table.load::<Instrument>(conn)?)
}