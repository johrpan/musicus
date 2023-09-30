use chrono::Utc;
use diesel::prelude::*;
use log::info;

use crate::db::{defer_foreign_keys, schema::ensembles, Result};

/// An ensemble that takes part in recordings.
#[derive(Insertable, Queryable, PartialEq, Eq, Hash, Debug, Clone)]
pub struct Ensemble {
    pub id: String,
    pub name: String,
    pub last_used: Option<i64>,
    pub last_played: Option<i64>,
}

impl Ensemble {
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            last_used: Some(Utc::now().timestamp()),
            last_played: None,
        }
    }
}

/// Update an existing ensemble or insert a new one.
pub fn update_ensemble(connection: &mut SqliteConnection, mut ensemble: Ensemble) -> Result<()> {
    info!("Updating ensemble {:?}", ensemble);
    defer_foreign_keys(connection)?;

    ensemble.last_used = Some(Utc::now().timestamp());

    connection.transaction(|connection| {
        diesel::replace_into(ensembles::table)
            .values(ensemble)
            .execute(connection)
    })?;

    Ok(())
}

/// Get an existing ensemble.
pub fn get_ensemble(connection: &mut SqliteConnection, id: &str) -> Result<Option<Ensemble>> {
    let ensemble = ensembles::table
        .filter(ensembles::id.eq(id))
        .load::<Ensemble>(connection)?
        .into_iter()
        .next();

    Ok(ensemble)
}

/// Delete an existing ensemble.
pub fn delete_ensemble(connection: &mut SqliteConnection, id: &str) -> Result<()> {
    info!("Deleting ensemble {}", id);
    diesel::delete(ensembles::table.filter(ensembles::id.eq(id))).execute(connection)?;
    Ok(())
}

/// Get all existing ensembles.
pub fn get_ensembles(connection: &mut SqliteConnection) -> Result<Vec<Ensemble>> {
    let ensembles = ensembles::table.load::<Ensemble>(connection)?;
    Ok(ensembles)
}

/// Get recently used ensembles.
pub fn get_recent_ensembles(connection: &mut SqliteConnection) -> Result<Vec<Ensemble>> {
    let ensembles = ensembles::table
        .order(ensembles::last_used.desc())
        .load::<Ensemble>(connection)?;

    Ok(ensembles)
}
