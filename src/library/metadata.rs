use std::sync::{Arc, Mutex};

use adw::subclass::prelude::*;
use anyhow::{anyhow, Result};
use chrono::prelude::*;
use diesel::{prelude::*, SqliteConnection};

use super::{exchange, Library};
use crate::db::{
    self,
    models::*,
    schema::*,
    tables::{self, Source},
};

/// A search result item that is either already part of the library or only available from the
/// separate metadata database.
#[derive(Clone, Debug)]
pub struct SearchItem<T> {
    pub item: T,
    pub in_library: bool,
}

impl Library {
    pub fn metadata_connection(&self) -> Option<Arc<Mutex<SqliteConnection>>> {
        let mut metadata_connection = self.imp().metadata_connection.borrow_mut();

        if metadata_connection.is_none() {
            let path = exchange::metadata_file_path();

            if path.exists() {
                match db::connect(path.to_str()?) {
                    Ok(connection) => {
                        *metadata_connection = Some(Arc::new(Mutex::new(connection)));
                    }
                    Err(err) => {
                        log::error!("Failed to open metadata database: {err:?}");
                    }
                }
            }
        }

        metadata_connection.clone()
    }

    pub fn import_metadata_person(&self, person_id: &str) -> Result<Person> {
        let metadata_connection = self
            .metadata_connection()
            .ok_or_else(|| anyhow!("No metadata database available"))?;
        let metadata_connection = &mut *db::lock_connection(&metadata_connection);
        let connection = &mut *self.conn();

        copy_person(metadata_connection, connection, person_id)?;

        let person = persons::table
            .filter(persons::person_id.eq(person_id))
            .first(connection)?;

        self.changed();

        Ok(person)
    }

    pub fn import_metadata_role(&self, role_id: &str) -> Result<Role> {
        let metadata_connection = self
            .metadata_connection()
            .ok_or_else(|| anyhow!("No metadata database available"))?;
        let metadata_connection = &mut *db::lock_connection(&metadata_connection);
        let connection = &mut *self.conn();

        copy_role(metadata_connection, connection, role_id)?;

        let role = roles::table
            .filter(roles::role_id.eq(role_id))
            .first(connection)?;

        self.changed();

        Ok(role)
    }

    pub fn import_metadata_instrument(&self, instrument_id: &str) -> Result<Instrument> {
        let metadata_connection = self
            .metadata_connection()
            .ok_or_else(|| anyhow!("No metadata database available"))?;
        let metadata_connection = &mut *db::lock_connection(&metadata_connection);
        let connection = &mut *self.conn();

        copy_instrument(metadata_connection, connection, instrument_id)?;

        let instrument = instruments::table
            .filter(instruments::instrument_id.eq(instrument_id))
            .first(connection)?;

        self.changed();

        Ok(instrument)
    }

    pub fn import_metadata_ensemble(&self, ensemble_id: &str) -> Result<Ensemble> {
        let metadata_connection = self
            .metadata_connection()
            .ok_or_else(|| anyhow!("No metadata database available"))?;
        let metadata_connection = &mut *db::lock_connection(&metadata_connection);
        let connection = &mut *self.conn();

        copy_ensemble(metadata_connection, connection, ensemble_id)?;

        let row = ensembles::table
            .filter(ensembles::ensemble_id.eq(ensemble_id))
            .first::<tables::Ensemble>(connection)?;

        let ensemble = Ensemble::from_table(row, connection)?;

        self.changed();

        Ok(ensemble)
    }

    pub fn import_metadata_work(&self, work_id: &str) -> Result<Work> {
        let metadata_connection = self
            .metadata_connection()
            .ok_or_else(|| anyhow!("No metadata database available"))?;
        let metadata_connection = &mut *db::lock_connection(&metadata_connection);
        let connection = &mut *self.conn();

        copy_work(metadata_connection, connection, work_id)?;

        let row = works::table
            .filter(works::work_id.eq(work_id))
            .first::<tables::Work>(connection)?;

        let work = Work::from_table(row, connection)?;

        self.changed();

        Ok(work)
    }

    pub fn import_metadata_recording(&self, recording_id: &str) -> Result<Recording> {
        let metadata_connection = self
            .metadata_connection()
            .ok_or_else(|| anyhow!("No metadata database available"))?;
        let metadata_connection = &mut *db::lock_connection(&metadata_connection);
        let connection = &mut *self.conn();

        copy_recording(metadata_connection, connection, recording_id)?;

        let row = recordings::table
            .filter(recordings::recording_id.eq(recording_id))
            .first::<tables::Recording>(connection)?;

        let recording = Recording::from_table(row, connection)?;

        self.changed();

        Ok(recording)
    }
}

fn copy_person(
    from: &mut SqliteConnection,
    to: &mut SqliteConnection,
    person_id: &str,
) -> Result<()> {
    let now = Local::now().naive_local();
    let mut person = persons::table
        .filter(persons::person_id.eq(person_id))
        .first::<tables::Person>(from)?;

    person.source = Source::Metadata;
    person.created_at = now;
    person.edited_at = now;
    person.last_used_at = now;
    person.last_played_at = None;

    diesel::insert_into(persons::table)
        .values(person)
        .on_conflict_do_nothing()
        .execute(to)?;

    Ok(())
}

fn copy_role(from: &mut SqliteConnection, to: &mut SqliteConnection, role_id: &str) -> Result<()> {
    let now = Local::now().naive_local();
    let mut role = roles::table
        .filter(roles::role_id.eq(role_id))
        .first::<tables::Role>(from)?;

    role.source = Source::Metadata;
    role.created_at = now;
    role.edited_at = now;
    role.last_used_at = now;

    diesel::insert_into(roles::table)
        .values(role)
        .on_conflict_do_nothing()
        .execute(to)?;

    Ok(())
}

fn copy_instrument(
    from: &mut SqliteConnection,
    to: &mut SqliteConnection,
    instrument_id: &str,
) -> Result<()> {
    let now = Local::now().naive_local();
    let mut instrument = instruments::table
        .filter(instruments::instrument_id.eq(instrument_id))
        .first::<tables::Instrument>(from)?;

    instrument.source = Source::Metadata;
    instrument.created_at = now;
    instrument.edited_at = now;
    instrument.last_used_at = now;
    instrument.last_played_at = None;

    diesel::insert_into(instruments::table)
        .values(instrument)
        .on_conflict_do_nothing()
        .execute(to)?;

    Ok(())
}

fn copy_work(from: &mut SqliteConnection, to: &mut SqliteConnection, work_id: &str) -> Result<()> {
    let now = Local::now().naive_local();
    let mut work = works::table
        .filter(works::work_id.eq(work_id))
        .first::<tables::Work>(from)?;

    if let Some(parent_work_id) = work.parent_work_id.clone() {
        copy_work(from, to, &parent_work_id)?;
    }

    work.source = Source::Metadata;
    work.created_at = now;
    work.edited_at = now;
    work.last_used_at = now;
    work.last_played_at = None;

    diesel::insert_into(works::table)
        .values(&work)
        .on_conflict_do_nothing()
        .execute(to)?;

    let work_persons = work_persons::table
        .filter(work_persons::work_id.eq(work_id))
        .load::<tables::WorkPerson>(from)?;

    for work_person in work_persons {
        copy_person(from, to, &work_person.person_id)?;

        if let Some(role_id) = &work_person.role_id {
            copy_role(from, to, role_id)?;
        }

        diesel::insert_into(work_persons::table)
            .values(work_person)
            .on_conflict_do_nothing()
            .execute(to)?;
    }

    let work_instruments = work_instruments::table
        .filter(work_instruments::work_id.eq(work_id))
        .load::<tables::WorkInstrument>(from)?;

    for work_instrument in work_instruments {
        copy_instrument(from, to, &work_instrument.instrument_id)?;

        diesel::insert_into(work_instruments::table)
            .values(work_instrument)
            .on_conflict_do_nothing()
            .execute(to)?;
    }

    Ok(())
}

fn copy_ensemble(
    from: &mut SqliteConnection,
    to: &mut SqliteConnection,
    ensemble_id: &str,
) -> Result<()> {
    let now = Local::now().naive_local();
    let mut ensemble = ensembles::table
        .filter(ensembles::ensemble_id.eq(ensemble_id))
        .first::<tables::Ensemble>(from)?;

    ensemble.source = Source::Metadata;
    ensemble.created_at = now;
    ensemble.edited_at = now;
    ensemble.last_used_at = now;
    ensemble.last_played_at = None;

    diesel::insert_into(ensembles::table)
        .values(&ensemble)
        .on_conflict_do_nothing()
        .execute(to)?;

    let ensemble_persons = ensemble_persons::table
        .filter(ensemble_persons::ensemble_id.eq(ensemble_id))
        .load::<tables::EnsemblePerson>(from)?;

    for ensemble_person in ensemble_persons {
        copy_person(from, to, &ensemble_person.person_id)?;

        if let Some(instrument_id) = &ensemble_person.instrument_id {
            copy_instrument(from, to, instrument_id)?;
        }

        diesel::insert_into(ensemble_persons::table)
            .values(ensemble_person)
            .on_conflict_do_nothing()
            .execute(to)?;
    }

    Ok(())
}

fn copy_recording(
    from: &mut SqliteConnection,
    to: &mut SqliteConnection,
    recording_id: &str,
) -> Result<()> {
    let now = Local::now().naive_local();
    let mut recording = recordings::table
        .filter(recordings::recording_id.eq(recording_id))
        .first::<tables::Recording>(from)?;

    copy_work(from, to, &recording.work_id)?;

    recording.source = Source::Metadata;
    recording.created_at = now;
    recording.edited_at = now;
    recording.last_used_at = now;
    recording.last_played_at = None;

    diesel::insert_into(recordings::table)
        .values(&recording)
        .on_conflict_do_nothing()
        .execute(to)?;

    let recording_persons = recording_persons::table
        .filter(recording_persons::recording_id.eq(recording_id))
        .load::<tables::RecordingPerson>(from)?;

    for recording_person in recording_persons {
        copy_person(from, to, &recording_person.person_id)?;

        if let Some(role_id) = &recording_person.role_id {
            copy_role(from, to, role_id)?;
        }

        if let Some(instrument_id) = &recording_person.instrument_id {
            copy_instrument(from, to, instrument_id)?;
        }

        diesel::insert_into(recording_persons::table)
            .values(recording_person)
            .on_conflict_do_nothing()
            .execute(to)?;
    }

    let recording_ensembles = recording_ensembles::table
        .filter(recording_ensembles::recording_id.eq(recording_id))
        .load::<tables::RecordingEnsemble>(from)?;

    for recording_ensemble in recording_ensembles {
        copy_ensemble(from, to, &recording_ensemble.ensemble_id)?;

        if let Some(role_id) = &recording_ensemble.role_id {
            copy_role(from, to, role_id)?;
        }

        diesel::insert_into(recording_ensembles::table)
            .values(recording_ensemble)
            .on_conflict_do_nothing()
            .execute(to)?;
    }

    Ok(())
}
