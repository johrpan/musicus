use super::schema::ensembles;
use super::DbConn;
use anyhow::Result;
use diesel::prelude::*;
use diesel::{Insertable, Queryable};
use serde::{Deserialize, Serialize};

/// An ensemble that takes part in recordings.
#[derive(Insertable, Queryable, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Ensemble {
    pub id: i64,
    pub name: String,

    #[serde(skip)]
    pub created_by: String,
}

/// A structure representing data on an ensemble.
#[derive(AsChangeset, Deserialize, Debug, Clone)]
#[table_name = "ensembles"]
#[serde(rename_all = "camelCase")]
pub struct EnsembleInsertion {
    pub name: String,
}

/// Insert a new ensemble.
pub fn insert_ensemble(
    conn: &DbConn,
    id: u32,
    data: &EnsembleInsertion,
    created_by: &str,
) -> Result<()> {
    let ensemble = Ensemble {
        id: id as i64,
        name: data.name.clone(),
        created_by: created_by.to_string(),
    };

    diesel::insert_into(ensembles::table)
        .values(ensemble)
        .execute(conn)?;

    Ok(())
}

/// Update an existing ensemble.
pub fn update_ensemble(conn: &DbConn, id: u32, data: &EnsembleInsertion) -> Result<()> {
    diesel::update(ensembles::table)
        .filter(ensembles::id.eq(id as i64))
        .set(data)
        .execute(conn)?;

    Ok(())
}

/// Get an existing ensemble.
pub fn get_ensemble(conn: &DbConn, id: u32) -> Result<Option<Ensemble>> {
    Ok(ensembles::table
        .filter(ensembles::id.eq(id as i64))
        .load::<Ensemble>(conn)?
        .first()
        .cloned())
}

/// Delete an existing ensemble.
pub fn delete_ensemble(conn: &DbConn, id: u32) -> Result<()> {
    diesel::delete(ensembles::table.filter(ensembles::id.eq(id as i64))).execute(conn)?;
    Ok(())
}

/// Get all existing ensembles.
pub fn get_ensembles(conn: &DbConn) -> Result<Vec<Ensemble>> {
    Ok(ensembles::table.load::<Ensemble>(conn)?)
}
