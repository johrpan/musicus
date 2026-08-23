//! CRUD for library entities.

use anyhow::{Error, Result};
use diesel::{prelude::*, QueryDsl, SqliteConnection};

use crate::db::{
    self,
    models::*,
    schema::*,
    tables::{self, Source},
    TranslatedString,
};
use crate::error::{EntityKind, LibraryError};
use crate::library::Library;

impl Library {
    pub fn create_person(&self, name: TranslatedString, enable_updates: bool) -> Result<Person> {
        let connection = &mut *self.conn();

        let now = db::now();

        let person = Person {
            person_id: db::generate_id(),
            name,
            source: Source::User,
            created_at: now,
            edited_at: now,
            last_used_at: now,
            enable_updates,
        };

        diesel::insert_into(persons::table)
            .values(&person)
            .execute(connection)?;

        self.changed();

        Ok(person)
    }

    pub fn update_person(
        &self,
        id: &str,
        name: TranslatedString,
        enable_updates: bool,
    ) -> Result<()> {
        let connection = &mut *self.conn();

        let now = db::now();

        diesel::update(persons::table)
            .filter(persons::person_id.eq(id))
            .set((
                persons::name.eq(name),
                persons::edited_at.eq(now),
                persons::last_used_at.eq(now),
                persons::enable_updates.eq(enable_updates),
            ))
            .execute(connection)?;

        self.changed();

        Ok(())
    }

    pub fn delete_person(&self, person_id: &str) -> Result<(), LibraryError> {
        let connection = &mut *self.conn();

        diesel::delete(persons::table)
            .filter(persons::person_id.eq(person_id))
            .execute(connection)
            .map_err(|err| LibraryError::from_delete(EntityKind::Person, err))?;

        self.changed();

        Ok(())
    }

    pub fn create_instrument(
        &self,
        name: TranslatedString,
        enable_updates: bool,
    ) -> Result<Instrument> {
        let connection = &mut *self.conn();

        let now = db::now();

        let instrument = Instrument {
            instrument_id: db::generate_id(),
            name,
            source: Source::User,
            created_at: now,
            edited_at: now,
            last_used_at: now,
            enable_updates,
        };

        diesel::insert_into(instruments::table)
            .values(&instrument)
            .execute(connection)?;

        self.changed();

        Ok(instrument)
    }

    pub fn update_instrument(
        &self,
        id: &str,
        name: TranslatedString,
        enable_updates: bool,
    ) -> Result<()> {
        let connection = &mut *self.conn();

        let now = db::now();

        diesel::update(instruments::table)
            .filter(instruments::instrument_id.eq(id))
            .set((
                instruments::name.eq(name),
                instruments::edited_at.eq(now),
                instruments::last_used_at.eq(now),
                instruments::enable_updates.eq(enable_updates),
            ))
            .execute(connection)?;

        self.changed();

        Ok(())
    }

    pub fn delete_instrument(&self, instrument_id: &str) -> Result<(), LibraryError> {
        let connection = &mut *self.conn();

        diesel::delete(instruments::table)
            .filter(instruments::instrument_id.eq(instrument_id))
            .execute(connection)
            .map_err(|err| LibraryError::from_delete(EntityKind::Instrument, err))?;

        self.changed();

        Ok(())
    }

    pub fn create_role(&self, name: TranslatedString, enable_updates: bool) -> Result<Role> {
        let connection = &mut *self.conn();

        let now = db::now();

        let role = Role {
            role_id: db::generate_id(),
            name,
            source: Source::User,
            created_at: now,
            edited_at: now,
            last_used_at: now,
            enable_updates,
        };

        diesel::insert_into(roles::table)
            .values(&role)
            .execute(connection)?;

        self.changed();

        Ok(role)
    }

    pub fn update_role(
        &self,
        id: &str,
        name: TranslatedString,
        enable_updates: bool,
    ) -> Result<()> {
        let connection = &mut *self.conn();

        let now = db::now();

        diesel::update(roles::table)
            .filter(roles::role_id.eq(id))
            .set((
                roles::name.eq(name),
                roles::edited_at.eq(now),
                roles::last_used_at.eq(now),
                roles::enable_updates.eq(enable_updates),
            ))
            .execute(connection)?;

        self.changed();

        Ok(())
    }

    pub fn delete_role(&self, role_id: &str) -> Result<(), LibraryError> {
        let connection = &mut *self.conn();

        diesel::delete(roles::table)
            .filter(roles::role_id.eq(role_id))
            .execute(connection)
            .map_err(|err| LibraryError::from_delete(EntityKind::Role, err))?;

        self.changed();

        Ok(())
    }

    pub fn create_work(
        &self,
        name: TranslatedString,
        parts: Vec<Work>,
        persons: Vec<Composer>,
        instruments: Vec<Instrument>,
        tags: Vec<TagValue>,
        relates_to: Option<Work>,
        enable_updates: bool,
    ) -> Result<Work> {
        let connection = &mut *self.conn();

        let work = connection.transaction::<Work, Error, _>(|connection| {
            Self::create_work_priv(
                connection,
                name,
                parts,
                persons,
                instruments,
                tags,
                None,
                None,
                relates_to.map(|w| w.work_id),
                enable_updates,
            )
        })?;

        self.changed();

        Ok(work)
    }

    #[allow(clippy::too_many_arguments)]
    fn create_work_priv(
        connection: &mut SqliteConnection,
        name: TranslatedString,
        parts: Vec<Work>,
        persons: Vec<Composer>,
        instruments: Vec<Instrument>,
        tags: Vec<TagValue>,
        parent_work_id: Option<&str>,
        sequence_number: Option<i32>,
        relates_to: Option<String>,
        enable_updates: bool,
    ) -> Result<Work> {
        let work_id = db::generate_id();
        let now = db::now();

        let work_data = tables::Work {
            work_id: work_id.clone(),
            parent_work_id: parent_work_id.map(|w| w.to_string()),
            sequence_number,
            name,
            source: Source::User,
            created_at: now,
            edited_at: now,
            last_used_at: now,
            enable_updates,
            relates_to,
        };

        diesel::insert_into(works::table)
            .values(&work_data)
            .execute(connection)?;

        for (index, part) in parts.into_iter().enumerate() {
            let part_relates_to = part.relates_to.map(|w| w.work_id);

            Self::create_work_priv(
                connection,
                part.name,
                part.parts,
                part.persons,
                part.instruments,
                part.tags,
                Some(&work_id),
                Some(index as i32),
                part_relates_to,
                enable_updates,
            )?;
        }

        Self::set_work_tags(connection, &work_id, tags)?;

        for (index, composer) in persons.into_iter().enumerate() {
            let composer_data = tables::WorkPerson {
                work_id: work_id.clone(),
                person_id: composer.person.person_id,
                role_id: composer.role.map(|r| r.role_id),
                sequence_number: index as i32,
            };

            diesel::insert_into(work_persons::table)
                .values(composer_data)
                .execute(connection)?;
        }

        for (index, instrument) in instruments.into_iter().enumerate() {
            let instrument_data = tables::WorkInstrument {
                work_id: work_id.clone(),
                instrument_id: instrument.instrument_id,
                sequence_number: index as i32,
            };

            diesel::insert_into(work_instruments::table)
                .values(instrument_data)
                .execute(connection)?;
        }

        let work = Work::from_table(work_data, connection)?;

        Ok(work)
    }

    pub fn update_work(
        &self,
        work_id: &str,
        name: TranslatedString,
        parts: Vec<Work>,
        persons: Vec<Composer>,
        instruments: Vec<Instrument>,
        tags: Vec<TagValue>,
        relates_to: Option<Work>,
        enable_updates: bool,
    ) -> Result<()> {
        let connection = &mut *self.conn();

        connection.transaction::<(), Error, _>(|connection| {
            Self::update_work_priv(
                connection,
                work_id,
                name,
                parts,
                persons,
                instruments,
                tags,
                None,
                None,
                relates_to.map(|w| w.work_id),
                enable_updates,
            )
        })?;

        self.changed();

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn update_work_priv(
        connection: &mut SqliteConnection,
        work_id: &str,
        name: TranslatedString,
        parts: Vec<Work>,
        persons: Vec<Composer>,
        instruments: Vec<Instrument>,
        tags: Vec<TagValue>,
        parent_work_id: Option<&str>,
        sequence_number: Option<i32>,
        relates_to: Option<String>,
        enable_updates: bool,
    ) -> Result<()> {
        let now = db::now();

        diesel::update(works::table)
            .filter(works::work_id.eq(work_id))
            .set((
                works::parent_work_id.eq(parent_work_id),
                works::sequence_number.eq(sequence_number),
                works::name.eq(name),
                works::edited_at.eq(now),
                works::last_used_at.eq(now),
                works::enable_updates.eq(enable_updates),
                works::relates_to.eq(relates_to),
            ))
            .execute(connection)?;

        diesel::delete(works::table)
            .filter(
                works::parent_work_id
                    .eq(work_id)
                    .and(works::work_id.ne_all(parts.iter().map(|p| p.work_id.clone()))),
            )
            .execute(connection)?;

        for (index, part) in parts.into_iter().enumerate() {
            let part_relates_to = part.relates_to.map(|w| w.work_id);

            if works::table
                .filter(works::work_id.eq(&part.work_id))
                .first::<tables::Work>(connection)
                .optional()?
                .is_some()
            {
                Self::update_work_priv(
                    connection,
                    &part.work_id,
                    part.name,
                    part.parts,
                    part.persons,
                    part.instruments,
                    part.tags,
                    Some(work_id),
                    Some(index as i32),
                    part_relates_to,
                    enable_updates,
                )?;
            } else {
                // Note: The previously used ID is discarded. This should be OK, because
                // at this point, the part ID should not have been used anywhere.
                Self::create_work_priv(
                    connection,
                    part.name,
                    part.parts,
                    part.persons,
                    part.instruments,
                    part.tags,
                    Some(work_id),
                    Some(index as i32),
                    part_relates_to,
                    enable_updates,
                )?;
            }
        }

        diesel::delete(work_persons::table)
            .filter(work_persons::work_id.eq(work_id))
            .execute(connection)?;

        for (index, composer) in persons.into_iter().enumerate() {
            let composer_data = tables::WorkPerson {
                work_id: work_id.to_string(),
                person_id: composer.person.person_id,
                role_id: composer.role.map(|r| r.role_id),
                sequence_number: index as i32,
            };

            diesel::insert_into(work_persons::table)
                .values(composer_data)
                .execute(connection)?;
        }

        diesel::delete(work_instruments::table)
            .filter(work_instruments::work_id.eq(work_id))
            .execute(connection)?;

        for (index, instrument) in instruments.into_iter().enumerate() {
            let instrument_data = tables::WorkInstrument {
                work_id: work_id.to_string(),
                instrument_id: instrument.instrument_id,
                sequence_number: index as i32,
            };

            diesel::insert_into(work_instruments::table)
                .values(instrument_data)
                .execute(connection)?;
        }

        Self::set_work_tags(connection, work_id, tags)?;

        Ok(())
    }

    pub fn delete_work(&self, work_id: &str) -> Result<(), LibraryError> {
        let connection = &mut *self.conn();

        diesel::delete(works::table)
            .filter(works::work_id.eq(work_id))
            .execute(connection)
            .map_err(|err| LibraryError::from_delete(EntityKind::Work, err))?;

        self.changed();

        Ok(())
    }

    pub fn create_ensemble(
        &self,
        name: TranslatedString,
        persons: Vec<Performer>,
        enable_updates: bool,
    ) -> Result<Ensemble> {
        let connection = &mut *self.conn();

        let ensemble = connection.transaction::<Ensemble, Error, _>(|connection| {
            let now = db::now();

            let ensemble_data = tables::Ensemble {
                ensemble_id: db::generate_id(),
                name,
                source: Source::User,
                created_at: now,
                edited_at: now,
                last_used_at: now,
                enable_updates,
            };

            diesel::insert_into(ensembles::table)
                .values(&ensemble_data)
                .execute(connection)?;

            for (index, member) in persons.into_iter().enumerate() {
                let ensemble_person_data = tables::EnsemblePerson {
                    ensemble_id: ensemble_data.ensemble_id.clone(),
                    person_id: member.person.person_id,
                    role_id: member.role.map(|r| r.role_id),
                    instrument_id: member.instrument.map(|i| i.instrument_id),
                    sequence_number: index as i32,
                };

                diesel::insert_into(ensemble_persons::table)
                    .values(&ensemble_person_data)
                    .execute(connection)?;
            }

            Ensemble::from_table(ensemble_data, connection)
        })?;

        self.changed();

        Ok(ensemble)
    }

    pub fn update_ensemble(
        &self,
        id: &str,
        name: TranslatedString,
        persons: Vec<Performer>,
        enable_updates: bool,
    ) -> Result<()> {
        let connection = &mut *self.conn();

        connection.transaction::<(), Error, _>(|connection| {
            let now = db::now();

            diesel::update(ensembles::table)
                .filter(ensembles::ensemble_id.eq(id))
                .set((
                    ensembles::name.eq(name),
                    ensembles::edited_at.eq(now),
                    ensembles::last_used_at.eq(now),
                    ensembles::enable_updates.eq(enable_updates),
                ))
                .execute(connection)?;

            diesel::delete(ensemble_persons::table)
                .filter(ensemble_persons::ensemble_id.eq(id))
                .execute(connection)?;

            for (index, member) in persons.into_iter().enumerate() {
                let ensemble_person_data = tables::EnsemblePerson {
                    ensemble_id: id.to_string(),
                    person_id: member.person.person_id,
                    role_id: member.role.map(|r| r.role_id),
                    instrument_id: member.instrument.map(|i| i.instrument_id),
                    sequence_number: index as i32,
                };

                diesel::insert_into(ensemble_persons::table)
                    .values(&ensemble_person_data)
                    .execute(connection)?;
            }

            Ok(())
        })?;

        self.changed();

        Ok(())
    }

    pub fn delete_ensemble(&self, ensemble_id: &str) -> Result<(), LibraryError> {
        let connection = &mut *self.conn();

        diesel::delete(ensembles::table)
            .filter(ensembles::ensemble_id.eq(ensemble_id))
            .execute(connection)
            .map_err(|err| LibraryError::from_delete(EntityKind::Ensemble, err))?;

        self.changed();

        Ok(())
    }

    pub fn create_recording(
        &self,
        work: Work,
        performers: Vec<Performer>,
        ensembles: Vec<EnsemblePerformer>,
        tags: Vec<TagValue>,
        comment: Option<String>,
        enable_updates: bool,
    ) -> Result<Recording> {
        let connection = &mut *self.conn();

        let recording = connection.transaction::<Recording, Error, _>(|connection| {
            let recording_id = db::generate_id();
            let now = db::now();

            let recording_data = tables::Recording {
                recording_id: recording_id.clone(),
                work_id: work.work_id.clone(),
                source: Source::User,
                created_at: now,
                edited_at: now,
                last_used_at: now,
                comment,
                enable_updates,
            };

            diesel::insert_into(recordings::table)
                .values(&recording_data)
                .execute(connection)?;

            for (index, performer) in performers.into_iter().enumerate() {
                let recording_person_data = tables::RecordingPerson {
                    recording_id: recording_id.clone(),
                    person_id: performer.person.person_id,
                    role_id: performer.role.map(|r| r.role_id),
                    instrument_id: performer.instrument.map(|i| i.instrument_id),
                    sequence_number: index as i32,
                };

                diesel::insert_into(recording_persons::table)
                    .values(&recording_person_data)
                    .execute(connection)?;
            }

            for (index, ensemble) in ensembles.into_iter().enumerate() {
                let recording_ensemble_data = tables::RecordingEnsemble {
                    recording_id: recording_id.clone(),
                    ensemble_id: ensemble.ensemble.ensemble_id,
                    role_id: ensemble.role.map(|r| r.role_id),
                    sequence_number: index as i32,
                };

                diesel::insert_into(recording_ensembles::table)
                    .values(&recording_ensemble_data)
                    .execute(connection)?;
            }

            Self::set_recording_tags(connection, &recording_id, tags)?;

            Recording::from_table(recording_data, connection)
        })?;

        self.changed();

        Ok(recording)
    }

    pub fn update_recording(
        &self,
        recording_id: &str,
        work: Work,
        performers: Vec<Performer>,
        ensembles: Vec<EnsemblePerformer>,
        tags: Vec<TagValue>,
        comment: Option<String>,
        enable_updates: bool,
    ) -> Result<()> {
        let connection = &mut *self.conn();

        connection.transaction::<(), Error, _>(|connection| {
            let now = db::now();

            diesel::update(recordings::table)
                .filter(recordings::recording_id.eq(recording_id))
                .set((
                    recordings::work_id.eq(work.work_id),
                    recordings::edited_at.eq(now),
                    recordings::last_used_at.eq(now),
                    recordings::comment.eq(comment),
                    recordings::enable_updates.eq(enable_updates),
                ))
                .execute(connection)?;

            diesel::delete(recording_persons::table)
                .filter(recording_persons::recording_id.eq(recording_id))
                .execute(connection)?;

            for (index, performer) in performers.into_iter().enumerate() {
                let recording_person_data = tables::RecordingPerson {
                    recording_id: recording_id.to_string(),
                    person_id: performer.person.person_id,
                    role_id: performer.role.map(|r| r.role_id),
                    instrument_id: performer.instrument.map(|i| i.instrument_id),
                    sequence_number: index as i32,
                };

                diesel::insert_into(recording_persons::table)
                    .values(&recording_person_data)
                    .execute(connection)?;
            }

            diesel::delete(recording_ensembles::table)
                .filter(recording_ensembles::recording_id.eq(recording_id))
                .execute(connection)?;

            for (index, ensemble) in ensembles.into_iter().enumerate() {
                let recording_ensemble_data = tables::RecordingEnsemble {
                    recording_id: recording_id.to_string(),
                    ensemble_id: ensemble.ensemble.ensemble_id,
                    role_id: ensemble.role.map(|r| r.role_id),
                    sequence_number: index as i32,
                };

                diesel::insert_into(recording_ensembles::table)
                    .values(&recording_ensemble_data)
                    .execute(connection)?;
            }

            Self::set_recording_tags(connection, recording_id, tags)?;

            Ok(())
        })?;

        self.changed();

        Ok(())
    }

    pub fn delete_recording(&self, recording_id: &str) -> Result<(), LibraryError> {
        let connection = &mut *self.conn();

        diesel::delete(recordings::table)
            .filter(recordings::recording_id.eq(recording_id))
            .execute(connection)
            .map_err(|err| LibraryError::from_delete(EntityKind::Recording, err))?;

        self.changed();

        Ok(())
    }

    pub fn create_album(
        &self,
        name: TranslatedString,
        recordings: Vec<Recording>,
        enable_updates: bool,
    ) -> Result<Album> {
        let connection = &mut *self.conn();

        let album = connection.transaction::<Album, Error, _>(|connection| {
            let album_id = db::generate_id();
            let now = db::now();

            let album_data = tables::Album {
                album_id: album_id.clone(),
                name,
                source: Source::User,
                enable_updates,
                created_at: now,
                edited_at: now,
                last_used_at: now,
            };

            diesel::insert_into(albums::table)
                .values(&album_data)
                .execute(connection)?;

            for (index, recording) in recordings.into_iter().enumerate() {
                let album_recording_data = tables::AlbumRecording {
                    album_id: album_id.clone(),
                    recording_id: recording.recording_id,
                    sequence_number: index as i32,
                };

                diesel::insert_into(album_recordings::table)
                    .values(&album_recording_data)
                    .execute(connection)?;
            }

            Album::from_table(album_data, connection)
        })?;

        self.changed();

        Ok(album)
    }

    pub fn update_album(
        &self,
        album_id: &str,
        name: TranslatedString,
        recordings: Vec<Recording>,
        enable_updates: bool,
    ) -> Result<()> {
        let connection = &mut *self.conn();

        connection.transaction::<(), Error, _>(|connection| {
            let now = db::now();

            diesel::update(albums::table)
                .filter(albums::album_id.eq(album_id))
                .set((
                    albums::name.eq(name),
                    albums::enable_updates.eq(enable_updates),
                    albums::edited_at.eq(now),
                    albums::last_used_at.eq(now),
                ))
                .execute(connection)?;

            diesel::delete(album_recordings::table)
                .filter(album_recordings::album_id.eq(album_id))
                .execute(connection)?;

            for (index, recording) in recordings.into_iter().enumerate() {
                let album_recording_data = tables::AlbumRecording {
                    album_id: album_id.to_owned(),
                    recording_id: recording.recording_id,
                    sequence_number: index as i32,
                };

                diesel::insert_into(album_recordings::table)
                    .values(&album_recording_data)
                    .execute(connection)?;
            }

            Ok(())
        })?;

        self.changed();

        Ok(())
    }

    pub fn delete_album(&self, album_id: &str) -> Result<(), LibraryError> {
        let connection = &mut *self.conn();

        diesel::delete(albums::table)
            .filter(albums::album_id.eq(album_id))
            .execute(connection)
            .map_err(|err| LibraryError::from_delete(EntityKind::Album, err))?;

        self.changed();

        Ok(())
    }
}
