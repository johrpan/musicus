//! Merging duplicate entities.
//!
//! A merge repoints all references to `from` in the library to `into`, then
//! discards `from`, including its owned rows from other tables. Recordings are not
//! mergable, because they own tracks on disk.

use std::collections::HashSet;

use anyhow::{bail, Result};
use diesel::prelude::*;

use super::Library;
use crate::db::schema::*;

/// Reference counts within the library for one entity.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EntityUsage {
    pub works: usize,
    pub recordings: usize,
    pub ensembles: usize,
    pub parts: usize,
    pub tracks: usize,
}

impl EntityUsage {
    pub fn total(&self) -> usize {
        self.works + self.recordings + self.ensembles + self.parts + self.tracks
    }
}

impl Library {
    /// Merge one person into another.
    pub fn merge_persons(&self, from: &str, into: &str) -> Result<()> {
        if from == into {
            bail!("cannot merge a person into itself");
        }

        let connection = &mut *self.conn();

        connection.transaction::<_, diesel::result::Error, _>(|connection| {
            diesel::update(work_persons::table.filter(work_persons::person_id.eq(from)))
                .set(work_persons::person_id.eq(into))
                .execute(connection)?;

            diesel::update(ensemble_persons::table.filter(ensemble_persons::person_id.eq(from)))
                .set(ensemble_persons::person_id.eq(into))
                .execute(connection)?;

            diesel::update(recording_persons::table.filter(recording_persons::person_id.eq(from)))
                .set(recording_persons::person_id.eq(into))
                .execute(connection)?;

            diesel::delete(persons::table.filter(persons::person_id.eq(from)))
                .execute(connection)?;

            Ok(())
        })?;

        self.changed();

        Ok(())
    }

    /// Merge one role into another.
    pub fn merge_roles(&self, from: &str, into: &str) -> Result<()> {
        if from == into {
            bail!("cannot merge a role into itself");
        }

        let connection = &mut *self.conn();

        connection.transaction::<_, diesel::result::Error, _>(|connection| {
            diesel::update(work_persons::table.filter(work_persons::role_id.eq(from)))
                .set(work_persons::role_id.eq(into))
                .execute(connection)?;

            diesel::update(ensemble_persons::table.filter(ensemble_persons::role_id.eq(from)))
                .set(ensemble_persons::role_id.eq(into))
                .execute(connection)?;

            diesel::update(recording_persons::table.filter(recording_persons::role_id.eq(from)))
                .set(recording_persons::role_id.eq(into))
                .execute(connection)?;

            diesel::update(
                recording_ensembles::table.filter(recording_ensembles::role_id.eq(from)),
            )
            .set(recording_ensembles::role_id.eq(into))
            .execute(connection)?;

            diesel::delete(roles::table.filter(roles::role_id.eq(from))).execute(connection)?;

            Ok(())
        })?;

        self.changed();

        Ok(())
    }

    /// Merge one instrument into another.
    pub fn merge_instruments(&self, from: &str, into: &str) -> Result<()> {
        if from == into {
            bail!("cannot merge an instrument into itself");
        }

        let connection = &mut *self.conn();

        connection.transaction::<_, diesel::result::Error, _>(|connection| {
            diesel::update(
                work_instruments::table.filter(work_instruments::instrument_id.eq(from)),
            )
            .set(work_instruments::instrument_id.eq(into))
            .execute(connection)?;

            diesel::update(
                ensemble_persons::table.filter(ensemble_persons::instrument_id.eq(from)),
            )
            .set(ensemble_persons::instrument_id.eq(into))
            .execute(connection)?;

            diesel::update(
                recording_persons::table.filter(recording_persons::instrument_id.eq(from)),
            )
            .set(recording_persons::instrument_id.eq(into))
            .execute(connection)?;

            diesel::delete(instruments::table.filter(instruments::instrument_id.eq(from)))
                .execute(connection)?;

            Ok(())
        })?;

        self.changed();

        Ok(())
    }

    /// Merge one tag into another.
    ///
    /// Meging a value-taking tag into a plain tag means that all values are
    /// effectively discarded.
    pub fn merge_tags(&self, from: &str, into: &str) -> Result<()> {
        if from == into {
            bail!("cannot merge a tag into itself");
        }

        let connection = &mut *self.conn();

        connection.transaction::<_, diesel::result::Error, _>(|connection| {
            let takes_value = tags::table
                .filter(tags::tag_id.eq(into))
                .select(tags::takes_value)
                .first::<bool>(connection)?;

            diesel::update(work_tags::table.filter(work_tags::tag_id.eq(from)))
                .set(work_tags::tag_id.eq(into))
                .execute(connection)?;

            diesel::update(recording_tags::table.filter(recording_tags::tag_id.eq(from)))
                .set(recording_tags::tag_id.eq(into))
                .execute(connection)?;

            if !takes_value {
                diesel::update(work_tags::table.filter(work_tags::tag_id.eq(into)))
                    .set(work_tags::value.eq(None::<String>))
                    .execute(connection)?;

                diesel::update(recording_tags::table.filter(recording_tags::tag_id.eq(into)))
                    .set(recording_tags::value.eq(None::<String>))
                    .execute(connection)?;
            }

            diesel::delete(tags::table.filter(tags::tag_id.eq(from))).execute(connection)?;

            Ok(())
        })?;

        self.changed();

        Ok(())
    }

    /// Merge one ensemble into another.
    pub fn merge_ensembles(&self, from: &str, into: &str) -> Result<()> {
        if from == into {
            bail!("cannot merge an ensemble into itself");
        }

        let connection = &mut *self.conn();

        connection.transaction::<_, diesel::result::Error, _>(|connection| {
            diesel::update(
                recording_ensembles::table.filter(recording_ensembles::ensemble_id.eq(from)),
            )
            .set(recording_ensembles::ensemble_id.eq(into))
            .execute(connection)?;

            diesel::delete(ensembles::table.filter(ensembles::ensemble_id.eq(from)))
                .execute(connection)?;

            Ok(())
        })?;

        self.changed();

        Ok(())
    }

    /// Merge one work into another.
    ///
    /// If `into` has work parts itself, work parts of `from` are discarded.
    /// Only if `into` does not have any children, they are moved to point to
    /// `into` instead.
    /// 
    /// Merging a work into its own children will fail to avoid circular
    /// references.
    pub fn merge_works(&self, from: &str, into: &str) -> Result<()> {
        if from == into {
            bail!("cannot merge a work into itself");
        }

        let connection = &mut *self.conn();

        connection.transaction::<_, anyhow::Error, _>(|connection| {
            let mut ancestor = works::table
                .filter(works::work_id.eq(into))
                .select(works::parent_work_id)
                .first::<Option<String>>(connection)?;
            let mut seen = HashSet::new();

            while let Some(id) = ancestor {
                if id == from {
                    bail!("cannot merge a work into one of its own descendants");
                }

                if !seen.insert(id.clone()) {
                    break;
                }

                ancestor = works::table
                    .filter(works::work_id.eq(&id))
                    .select(works::parent_work_id)
                    .first::<Option<String>>(connection)?;
            }

            // Check if `into` has any work parts (children)
            let into_has_children = works::table
                .filter(works::parent_work_id.eq(into))
                .select(works::work_id)
                .first::<String>(connection)
                .optional()?
                .is_some();

            // Only move work parts of `from` to `into` if `into` has no work parts
            if !into_has_children {
                diesel::update(works::table.filter(works::parent_work_id.eq(from)))
                    .set(works::parent_work_id.eq(into))
                    .execute(connection)?;
            }

            diesel::update(recordings::table.filter(recordings::work_id.eq(from)))
                .set(recordings::work_id.eq(into))
                .execute(connection)?;

            diesel::update(track_works::table.filter(track_works::work_id.eq(from)))
                .set(track_works::work_id.eq(into))
                .execute(connection)?;

            diesel::delete(works::table.filter(works::work_id.eq(from))).execute(connection)?;

            Ok(())
        })?;

        self.changed();

        Ok(())
    }

    /// How much else in the library refers to this person.
    pub fn usage_of_person(&self, id: &str) -> Result<EntityUsage> {
        let connection = &mut *self.conn();

        Ok(EntityUsage {
            works: as_usize(
                work_persons::table
                    .filter(work_persons::person_id.eq(id))
                    .count()
                    .get_result(connection),
            )?,
            ensembles: as_usize(
                ensemble_persons::table
                    .filter(ensemble_persons::person_id.eq(id))
                    .count()
                    .get_result(connection),
            )?,
            recordings: as_usize(
                recording_persons::table
                    .filter(recording_persons::person_id.eq(id))
                    .count()
                    .get_result(connection),
            )?,
            ..EntityUsage::default()
        })
    }

    /// How much else in the library refers to this role.
    pub fn usage_of_role(&self, id: &str) -> Result<EntityUsage> {
        let connection = &mut *self.conn();

        Ok(EntityUsage {
            works: as_usize(
                work_persons::table
                    .filter(work_persons::role_id.eq(id))
                    .count()
                    .get_result(connection),
            )?,
            ensembles: as_usize(
                ensemble_persons::table
                    .filter(ensemble_persons::role_id.eq(id))
                    .count()
                    .get_result(connection),
            )?,
            recordings: as_usize(
                recording_persons::table
                    .filter(recording_persons::role_id.eq(id))
                    .count()
                    .get_result(connection),
            )? + as_usize(
                recording_ensembles::table
                    .filter(recording_ensembles::role_id.eq(id))
                    .count()
                    .get_result(connection),
            )?,
            ..EntityUsage::default()
        })
    }

    /// How much else in the library refers to this instrument.
    pub fn usage_of_instrument(&self, id: &str) -> Result<EntityUsage> {
        let connection = &mut *self.conn();

        Ok(EntityUsage {
            works: as_usize(
                work_instruments::table
                    .filter(work_instruments::instrument_id.eq(id))
                    .count()
                    .get_result(connection),
            )?,
            ensembles: as_usize(
                ensemble_persons::table
                    .filter(ensemble_persons::instrument_id.eq(id))
                    .count()
                    .get_result(connection),
            )?,
            recordings: as_usize(
                recording_persons::table
                    .filter(recording_persons::instrument_id.eq(id))
                    .count()
                    .get_result(connection),
            )?,
            ..EntityUsage::default()
        })
    }

    /// How much else in the library refers to this tag.
    pub fn usage_of_tag(&self, id: &str) -> Result<EntityUsage> {
        let connection = &mut *self.conn();

        Ok(EntityUsage {
            works: as_usize(
                work_tags::table
                    .filter(work_tags::tag_id.eq(id))
                    .count()
                    .get_result(connection),
            )?,
            recordings: as_usize(
                recording_tags::table
                    .filter(recording_tags::tag_id.eq(id))
                    .count()
                    .get_result(connection),
            )?,
            ..EntityUsage::default()
        })
    }

    /// How much else in the library refers to this ensemble.
    pub fn usage_of_ensemble(&self, id: &str) -> Result<EntityUsage> {
        let connection = &mut *self.conn();

        Ok(EntityUsage {
            recordings: as_usize(
                recording_ensembles::table
                    .filter(recording_ensembles::ensemble_id.eq(id))
                    .count()
                    .get_result(connection),
            )?,
            ..EntityUsage::default()
        })
    }

    /// How much else in the library refers to this work.
    pub fn usage_of_work(&self, id: &str) -> Result<EntityUsage> {
        let connection = &mut *self.conn();

        Ok(EntityUsage {
            parts: as_usize(
                works::table
                    .filter(works::parent_work_id.eq(id))
                    .count()
                    .get_result(connection),
            )?,
            recordings: as_usize(
                recordings::table
                    .filter(recordings::work_id.eq(id))
                    .count()
                    .get_result(connection),
            )?,
            tracks: as_usize(
                track_works::table
                    .filter(track_works::work_id.eq(id))
                    .count()
                    .get_result(connection),
            )?,
            ..EntityUsage::default()
        })
    }
}

fn as_usize(count: std::result::Result<i64, diesel::result::Error>) -> Result<usize> {
    Ok(count?.max(0) as usize)
}