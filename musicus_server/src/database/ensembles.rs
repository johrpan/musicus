use super::schema::ensembles;
use super::{DbConn, User};
use crate::error::ServerError;
use anyhow::{Error, Result};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

/// A ensemble as represented within the API.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Ensemble {
    pub id: u32,
    pub name: String,
}

/// A ensemble as represented in the database.
#[derive(Insertable, Queryable, AsChangeset, Debug, Clone)]
#[table_name = "ensembles"]
struct EnsembleRow {
    pub id: i64,
    pub name: String,
    pub created_by: String,
}

impl From<EnsembleRow> for Ensemble {
    fn from(row: EnsembleRow) -> Ensemble {
        Ensemble {
            id: row.id as u32,
            name: row.name,
        }
    }
}

/// Update an existing ensemble or insert a new one. This will only work, if the provided user is
/// allowed to do that.
pub fn update_ensemble(conn: &DbConn, ensemble: &Ensemble, user: &User) -> Result<()> {
    let old_row = get_ensemble_row(conn, ensemble.id)?;

    let allowed = match old_row {
        Some(row) => user.may_edit(&row.created_by),
        None => user.may_create(),
    };

    if allowed {
        let new_row = EnsembleRow {
            id: ensemble.id as i64,
            name: ensemble.name.clone(),
            created_by: user.username.clone(),
        };

        diesel::insert_into(ensembles::table)
            .values(&new_row)
            .on_conflict(ensembles::id)
            .do_update()
            .set(&new_row)
            .execute(conn)?;

        Ok(())
    } else {
        Err(Error::new(ServerError::Forbidden))
    }
}

/// Get an existing ensemble.
pub fn get_ensemble(conn: &DbConn, id: u32) -> Result<Option<Ensemble>> {
    let row = get_ensemble_row(conn, id)?;
    let ensemble = row.map(|row| row.into());

    Ok(ensemble)
}

/// Delete an existing ensemble. This will only work if the provided user is allowed to do that.
pub fn delete_ensemble(conn: &DbConn, id: u32, user: &User) -> Result<()> {
    if user.may_delete() {
        diesel::delete(ensembles::table.filter(ensembles::id.eq(id as i64))).execute(conn)?;
        Ok(())
    } else {
        Err(Error::new(ServerError::Forbidden))
    }
}

/// Get all existing ensembles.
pub fn get_ensembles(conn: &DbConn) -> Result<Vec<Ensemble>> {
    let rows = ensembles::table.load::<EnsembleRow>(conn)?;
    let ensembles: Vec<Ensemble> = rows.into_iter().map(|row| row.into()).collect();

    Ok(ensembles)
}

/// Get a ensemble row if it exists.
fn get_ensemble_row(conn: &DbConn, id: u32) -> Result<Option<EnsembleRow>> {
    let row = ensembles::table
        .filter(ensembles::id.eq(id as i64))
        .load::<EnsembleRow>(conn)?
        .into_iter()
        .next();

    Ok(row)
}
