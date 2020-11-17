use super::schema::ensembles;
use super::Database;
use anyhow::{Error, Result};
use diesel::prelude::*;
use diesel::{Insertable, Queryable};
use serde::{Deserialize, Serialize};
use std::convert::{TryFrom, TryInto};

/// Database table data for an ensemble.
#[derive(Insertable, Queryable, Debug, Clone)]
#[table_name = "ensembles"]
struct EnsembleRow {
    pub id: i64,
    pub name: String,
}

impl From<Ensemble> for EnsembleRow {
    fn from(ensemble: Ensemble) -> Self {
        EnsembleRow {
            id: ensemble.id as i64,
            name: ensemble.name,
        }
    }
}

/// An ensemble that takes part in recordings.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Ensemble {
    pub id: u32,
    pub name: String,
}

impl TryFrom<EnsembleRow> for Ensemble {
    type Error = Error;
    fn try_from(row: EnsembleRow) -> Result<Self> {
        let ensemble = Ensemble {
            id: row.id.try_into()?,
            name: row.name,
        };

        Ok(ensemble)
    }
}

impl Database {
    /// Update an existing ensemble or insert a new one.
    pub fn update_ensemble(&self, ensemble: Ensemble) -> Result<()> {
        self.defer_foreign_keys()?;

        self.connection.transaction(|| {
            let row: EnsembleRow = ensemble.into();
            diesel::replace_into(ensembles::table)
                .values(row)
                .execute(&self.connection)
        })?;

        Ok(())
    }

    /// Get an existing ensemble.
    pub fn get_ensemble(&self, id: u32) -> Result<Option<Ensemble>> {
        let row = ensembles::table
            .filter(ensembles::id.eq(id as i64))
            .load::<EnsembleRow>(&self.connection)?
            .first()
            .cloned();

        let ensemble = match row {
            Some(row) => Some(row.try_into()?),
            None => None,
        };

        Ok(ensemble)
    }

    /// Delete an existing ensemble.
    pub fn delete_ensemble(&self, id: u32) -> Result<()> {
        diesel::delete(ensembles::table.filter(ensembles::id.eq(id as i64)))
            .execute(&self.connection)?;

        Ok(())
    }

    /// Get all existing ensembles.
    pub fn get_ensembles(&self) -> Result<Vec<Ensemble>> {
        let mut ensembles = Vec::<Ensemble>::new();

        let rows = ensembles::table.load::<EnsembleRow>(&self.connection)?;
        for row in rows {
            ensembles.push(row.try_into()?);
        }

        Ok(ensembles)
    }
}
