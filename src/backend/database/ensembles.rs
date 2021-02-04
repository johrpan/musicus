use super::schema::ensembles;
use super::{Database, DatabaseResult};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

/// An ensemble that takes part in recordings.
#[derive(Serialize, Deserialize, Insertable, Queryable, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Ensemble {
    pub id: String,
    pub name: String,
}

impl Database {
    /// Update an existing ensemble or insert a new one.
    pub fn update_ensemble(&self, ensemble: Ensemble) -> DatabaseResult<()> {
        self.defer_foreign_keys()?;

        self.connection.transaction(|| {
            diesel::replace_into(ensembles::table)
                .values(ensemble)
                .execute(&self.connection)
        })?;

        Ok(())
    }

    /// Get an existing ensemble.
    pub fn get_ensemble(&self, id: &str) -> DatabaseResult<Option<Ensemble>> {
        let ensemble = ensembles::table
            .filter(ensembles::id.eq(id))
            .load::<Ensemble>(&self.connection)?
            .into_iter()
            .next();

        Ok(ensemble)
    }

    /// Delete an existing ensemble.
    pub fn delete_ensemble(&self, id: &str) -> DatabaseResult<()> {
        diesel::delete(ensembles::table.filter(ensembles::id.eq(id))).execute(&self.connection)?;

        Ok(())
    }

    /// Get all existing ensembles.
    pub fn get_ensembles(&self) -> DatabaseResult<Vec<Ensemble>> {
        let ensembles = ensembles::table.load::<Ensemble>(&self.connection)?;

        Ok(ensembles)
    }
}
