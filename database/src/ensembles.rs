use super::schema::ensembles;
use super::{Database, Result};
use diesel::prelude::*;
use log::info;
use serde::{Deserialize, Serialize};

/// An ensemble that takes part in recordings.
#[derive(Serialize, Deserialize, Insertable, Queryable, PartialEq, Eq, Hash, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Ensemble {
    pub id: String,
    pub name: String,
}

impl Database {
    /// Update an existing ensemble or insert a new one.
    pub fn update_ensemble(&self, ensemble: Ensemble) -> Result<()> {
        info!("Updating ensemble {:?}", ensemble);
        self.defer_foreign_keys()?;

        self.connection.transaction(|| {
            diesel::replace_into(ensembles::table)
                .values(ensemble)
                .execute(&self.connection)
        })?;

        Ok(())
    }

    /// Get an existing ensemble.
    pub fn get_ensemble(&self, id: &str) -> Result<Option<Ensemble>> {
        let ensemble = ensembles::table
            .filter(ensembles::id.eq(id))
            .load::<Ensemble>(&self.connection)?
            .into_iter()
            .next();

        Ok(ensemble)
    }

    /// Delete an existing ensemble.
    pub fn delete_ensemble(&self, id: &str) -> Result<()> {
        info!("Deleting ensemble {}", id);
        diesel::delete(ensembles::table.filter(ensembles::id.eq(id))).execute(&self.connection)?;
        Ok(())
    }

    /// Get all existing ensembles.
    pub fn get_ensembles(&self) -> Result<Vec<Ensemble>> {
        let ensembles = ensembles::table.load::<Ensemble>(&self.connection)?;
        Ok(ensembles)
    }
}
