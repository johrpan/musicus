//! CRUD for library entities.

use anyhow::{bail, Error, Result};
use diesel::{prelude::*, QueryDsl, SqliteConnection};

use crate::{
    db::{self, schema::*, tables, tables::Source, TranslatedString},
    error::{EntityKind, LibraryError},
    library::{load::Loader, Library},
    models::*,
};

impl Library {
    pub fn create_person(&self, name: TranslatedString, enable_updates: bool) -> Result<Person> {
        let connection = &mut *self.conn();

        let person = tables::Person {
            person_id: db::generate_id(),
            name,
            source: Source::User,
            created_at: db::now(),
            edited_at: db::now(),
            last_used_at: db::now(),
            enable_updates,
        };

        diesel::insert_into(persons::table)
            .values(&person)
            .execute(connection)?;

        self.changed();

        Ok(Person::from(person))
    }

    pub fn update_person(
        &self,
        id: &str,
        name: TranslatedString,
        enable_updates: bool,
    ) -> Result<()> {
        let connection = &mut *self.conn();

        let updated = diesel::update(persons::table)
            .filter(persons::person_id.eq(id))
            .set((
                persons::name.eq(name),
                persons::edited_at.eq(db::now()),
                persons::last_used_at.eq(db::now()),
                persons::enable_updates.eq(enable_updates),
            ))
            .execute(connection)?;

        if updated == 0 {
            return Err(LibraryError::NotFound(EntityKind::Person, id.to_owned()).into());
        }

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

        let instrument = tables::Instrument {
            instrument_id: db::generate_id(),
            name,
            source: Source::User,
            created_at: db::now(),
            edited_at: db::now(),
            last_used_at: db::now(),
            enable_updates,
        };

        diesel::insert_into(instruments::table)
            .values(&instrument)
            .execute(connection)?;

        self.changed();

        Ok(Instrument::from(instrument))
    }

    pub fn update_instrument(
        &self,
        id: &str,
        name: TranslatedString,
        enable_updates: bool,
    ) -> Result<()> {
        let connection = &mut *self.conn();

        let updated = diesel::update(instruments::table)
            .filter(instruments::instrument_id.eq(id))
            .set((
                instruments::name.eq(name),
                instruments::edited_at.eq(db::now()),
                instruments::last_used_at.eq(db::now()),
                instruments::enable_updates.eq(enable_updates),
            ))
            .execute(connection)?;

        if updated == 0 {
            return Err(LibraryError::NotFound(EntityKind::Instrument, id.to_owned()).into());
        }

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

        let role = tables::Role {
            role_id: db::generate_id(),
            name,
            source: Source::User,
            created_at: db::now(),
            edited_at: db::now(),
            last_used_at: db::now(),
            enable_updates,
        };

        diesel::insert_into(roles::table)
            .values(&role)
            .execute(connection)?;

        self.changed();

        Ok(Role::from(role))
    }

    pub fn update_role(
        &self,
        id: &str,
        name: TranslatedString,
        enable_updates: bool,
    ) -> Result<()> {
        let connection = &mut *self.conn();

        let updated = diesel::update(roles::table)
            .filter(roles::role_id.eq(id))
            .set((
                roles::name.eq(name),
                roles::edited_at.eq(db::now()),
                roles::last_used_at.eq(db::now()),
                roles::enable_updates.eq(enable_updates),
            ))
            .execute(connection)?;

        if updated == 0 {
            return Err(LibraryError::NotFound(EntityKind::Role, id.to_owned()).into());
        }

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
        persons: Vec<WorkPerson>,
        instruments: Vec<Instrument>,
        tags: Vec<TagValue>,
        related_works: Vec<String>,
        enable_updates: bool,
    ) -> Result<Work> {
        let connection = &mut *self.conn();

        let work_id = connection.transaction::<String, Error, _>(|connection| {
            let work_id = db::generate_id();

            Self::create_work_priv(
                connection,
                work_id.clone(),
                name,
                parts,
                persons,
                instruments,
                tags,
                None,
                None,
                related_works,
                enable_updates,
            )?;

            Ok(work_id)
        })?;

        let loader = Loader::load(connection)?;
        let work = loader.work(&work_id)?;

        self.changed();

        Ok(work)
    }

    #[allow(clippy::too_many_arguments)]
    fn create_work_priv(
        connection: &mut SqliteConnection,
        work_id: String,
        name: TranslatedString,
        parts: Vec<Work>,
        persons: Vec<WorkPerson>,
        instruments: Vec<Instrument>,
        tags: Vec<TagValue>,
        parent_work_id: Option<&str>,
        sequence_number: Option<i32>,
        related_works: Vec<String>,
        enable_updates: bool,
    ) -> Result<()> {
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
        };

        diesel::insert_into(works::table)
            .values(&work_data)
            .execute(connection)?;

        for (index, part) in parts.into_iter().enumerate() {
            let part_related_works = part.related_works.into_iter().map(|w| w.id).collect();

            // A part from an existing library keeps its ID when it is being
            // moved into a new work by the update path; a fresh part gets a new
            // one.
            let part_id = db::generate_id();

            Self::create_work_priv(
                connection,
                part_id,
                part.name,
                part.parts,
                part.work_persons,
                part.instruments,
                part.tags,
                Some(&work_id),
                Some(db::sequence_number(index)?),
                part_related_works,
                enable_updates,
            )?;
        }

        Self::replace_work_relations(connection, &work_id, related_works)?;

        Self::set_work_tags(connection, &work_id, tags)?;

        for (index, work_person) in persons.into_iter().enumerate() {
            let work_person_data = tables::WorkPerson {
                work_id: work_id.clone(),
                person_id: work_person.person.id,
                role_id: work_person.role.map(|r| r.id),
                sequence_number: db::sequence_number(index)?,
            };

            diesel::insert_into(work_persons::table)
                .values(work_person_data)
                .execute(connection)?;
        }

        for (index, instrument) in instruments.into_iter().enumerate() {
            let instrument_data = tables::WorkInstrument {
                work_id: work_id.clone(),
                instrument_id: instrument.id,
                sequence_number: db::sequence_number(index)?,
            };

            diesel::insert_into(work_instruments::table)
                .values(instrument_data)
                .execute(connection)?;
        }

        Ok(())
    }

    pub fn update_work(
        &self,
        work_id: &str,
        name: TranslatedString,
        parts: Vec<Work>,
        persons: Vec<WorkPerson>,
        instruments: Vec<Instrument>,
        tags: Vec<TagValue>,
        related_works: Vec<String>,
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
                related_works,
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
        persons: Vec<WorkPerson>,
        instruments: Vec<Instrument>,
        tags: Vec<TagValue>,
        parent_work_id: Option<&str>,
        sequence_number: Option<i32>,
        related_works: Vec<String>,
        enable_updates: bool,
    ) -> Result<()> {
        let now = db::now();

        let updated = diesel::update(works::table)
            .filter(works::work_id.eq(work_id))
            .set((
                works::parent_work_id.eq(parent_work_id),
                works::sequence_number.eq(sequence_number),
                works::name.eq(name),
                works::edited_at.eq(now),
                works::last_used_at.eq(now),
                works::enable_updates.eq(enable_updates),
            ))
            .execute(connection)?;

        if updated == 0 {
            return Err(LibraryError::NotFound(EntityKind::Work, work_id.to_owned()).into());
        }

        diesel::delete(works::table)
            .filter(
                works::parent_work_id
                    .eq(work_id)
                    .and(works::work_id.ne_all(parts.iter().map(|p| p.id.clone()))),
            )
            .execute(connection)?;

        for (index, part) in parts.into_iter().enumerate() {
            let part_related_works = part.related_works.into_iter().map(|w| w.id).collect();

            if works::table
                .filter(works::work_id.eq(&part.id))
                .first::<tables::Work>(connection)
                .optional()?
                .is_some()
            {
                Self::update_work_priv(
                    connection,
                    &part.id,
                    part.name,
                    part.parts,
                    part.work_persons,
                    part.instruments,
                    part.tags,
                    Some(work_id),
                    Some(db::sequence_number(index)?),
                    part_related_works,
                    enable_updates,
                )?;
            } else {
                Self::create_work_priv(
                    connection,
                    part.id,
                    part.name,
                    part.parts,
                    part.work_persons,
                    part.instruments,
                    part.tags,
                    Some(work_id),
                    Some(db::sequence_number(index)?),
                    part_related_works,
                    enable_updates,
                )?;
            }
        }

        Self::replace_work_relations(connection, work_id, related_works)?;

        diesel::delete(work_persons::table)
            .filter(work_persons::work_id.eq(work_id))
            .execute(connection)?;

        for (index, work_person) in persons.into_iter().enumerate() {
            let work_person_data = tables::WorkPerson {
                work_id: work_id.to_string(),
                person_id: work_person.person.id,
                role_id: work_person.role.map(|r| r.id),
                sequence_number: db::sequence_number(index)?,
            };

            diesel::insert_into(work_persons::table)
                .values(work_person_data)
                .execute(connection)?;
        }

        diesel::delete(work_instruments::table)
            .filter(work_instruments::work_id.eq(work_id))
            .execute(connection)?;

        for (index, instrument) in instruments.into_iter().enumerate() {
            let instrument_data = tables::WorkInstrument {
                work_id: work_id.to_string(),
                instrument_id: instrument.id,
                sequence_number: db::sequence_number(index)?,
            };

            diesel::insert_into(work_instruments::table)
                .values(instrument_data)
                .execute(connection)?;
        }

        Self::set_work_tags(connection, work_id, tags)?;

        Ok(())
    }

    /// Replace a work's outgoing relation rows, validating the new targets.
    pub(super) fn replace_work_relations(
        connection: &mut SqliteConnection,
        work_id: &str,
        related_works: Vec<String>,
    ) -> Result<()> {
        let mut seen = std::collections::HashSet::new();

        for related_work_id in &related_works {
            if related_work_id == work_id {
                bail!("Work {work_id} cannot relate to itself");
            }

            if !seen.insert(related_work_id.clone()) {
                bail!("Work {work_id} relates to {related_work_id} more than once");
            }

            let target_exists = works::table
                .filter(works::work_id.eq(related_work_id))
                .select(works::work_id)
                .first::<String>(connection)
                .optional()?
                .is_some();

            if !target_exists {
                return Err(
                    LibraryError::NotFound(EntityKind::Work, related_work_id.clone()).into(),
                );
            }
        }

        diesel::delete(work_relations::table)
            .filter(work_relations::work_id.eq(work_id))
            .execute(connection)?;

        for related_work_id in related_works {
            let relation = tables::WorkRelation {
                work_id: work_id.to_string(),
                related_work_id,
            };

            diesel::insert_into(work_relations::table)
                .values(relation)
                .execute(connection)?;
        }

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
        persons: Vec<RecordingPerson>,
        enable_updates: bool,
    ) -> Result<Ensemble> {
        let connection = &mut *self.conn();

        let ensemble_id = connection.transaction::<String, Error, _>(|connection| {
            let ensemble_id = db::generate_id();
            let now = db::now();

            let ensemble_data = tables::Ensemble {
                ensemble_id: ensemble_id.clone(),
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

            insert_ensemble_persons(connection, &ensemble_id, persons)?;

            Ok(ensemble_id)
        })?;

        let loader = Loader::load(connection)?;
        let ensemble = loader.ensemble(&ensemble_id)?;

        self.changed();

        Ok(ensemble)
    }

    pub fn update_ensemble(
        &self,
        id: &str,
        name: TranslatedString,
        persons: Vec<RecordingPerson>,
        enable_updates: bool,
    ) -> Result<()> {
        let connection = &mut *self.conn();

        connection.transaction::<(), Error, _>(|connection| {
            let updated = diesel::update(ensembles::table)
                .filter(ensembles::ensemble_id.eq(id))
                .set((
                    ensembles::name.eq(name),
                    ensembles::edited_at.eq(db::now()),
                    ensembles::last_used_at.eq(db::now()),
                    ensembles::enable_updates.eq(enable_updates),
                ))
                .execute(connection)?;

            if updated == 0 {
                return Err(LibraryError::NotFound(EntityKind::Ensemble, id.to_owned()).into());
            }

            diesel::delete(ensemble_persons::table)
                .filter(ensemble_persons::ensemble_id.eq(id))
                .execute(connection)?;

            insert_ensemble_persons(connection, id, persons)?;

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
        performers: Vec<RecordingPerson>,
        ensembles: Vec<RecordingEnsemble>,
        tags: Vec<TagValue>,
        comment: Option<String>,
        enable_updates: bool,
    ) -> Result<Recording> {
        let connection = &mut *self.conn();

        let recording_id = connection.transaction::<String, Error, _>(|connection| {
            let recording_id = db::generate_id();
            let now = db::now();

            let recording_data = tables::Recording {
                recording_id: recording_id.clone(),
                work_id: work.id.clone(),
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

            insert_recording_persons(connection, &recording_id, performers)?;
            insert_recording_ensembles(connection, &recording_id, ensembles)?;

            Self::set_recording_tags(connection, &recording_id, tags)?;

            Ok(recording_id)
        })?;

        let loader = Loader::load(connection)?;
        let recording = loader.recording(&recording_id)?;

        self.changed();

        Ok(recording)
    }

    pub fn update_recording(
        &self,
        recording_id: &str,
        work: Work,
        performers: Vec<RecordingPerson>,
        ensembles: Vec<RecordingEnsemble>,
        tags: Vec<TagValue>,
        comment: Option<String>,
        enable_updates: bool,
    ) -> Result<()> {
        let connection = &mut *self.conn();

        connection.transaction::<(), Error, _>(|connection| {
            let updated = diesel::update(recordings::table)
                .filter(recordings::recording_id.eq(recording_id))
                .set((
                    recordings::work_id.eq(work.id),
                    recordings::edited_at.eq(db::now()),
                    recordings::last_used_at.eq(db::now()),
                    recordings::comment.eq(comment),
                    recordings::enable_updates.eq(enable_updates),
                ))
                .execute(connection)?;

            if updated == 0 {
                return Err(
                    LibraryError::NotFound(EntityKind::Recording, recording_id.to_owned()).into(),
                );
            }

            diesel::delete(recording_persons::table)
                .filter(recording_persons::recording_id.eq(recording_id))
                .execute(connection)?;

            insert_recording_persons(connection, recording_id, performers)?;

            diesel::delete(recording_ensembles::table)
                .filter(recording_ensembles::recording_id.eq(recording_id))
                .execute(connection)?;

            insert_recording_ensembles(connection, recording_id, ensembles)?;

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

        let album_id = connection.transaction::<String, Error, _>(|connection| {
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
                    recording_id: recording.id,
                    sequence_number: db::sequence_number(index)?,
                };

                diesel::insert_into(album_recordings::table)
                    .values(&album_recording_data)
                    .execute(connection)?;
            }

            Ok(album_id)
        })?;

        let loader = Loader::load(connection)?;
        let album = loader.album(&album_id)?;

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
            let updated = diesel::update(albums::table)
                .filter(albums::album_id.eq(album_id))
                .set((
                    albums::name.eq(name),
                    albums::enable_updates.eq(enable_updates),
                    albums::edited_at.eq(db::now()),
                    albums::last_used_at.eq(db::now()),
                ))
                .execute(connection)?;

            if updated == 0 {
                return Err(LibraryError::NotFound(EntityKind::Album, album_id.to_owned()).into());
            }

            diesel::delete(album_recordings::table)
                .filter(album_recordings::album_id.eq(album_id))
                .execute(connection)?;

            for (index, recording) in recordings.into_iter().enumerate() {
                let album_recording_data = tables::AlbumRecording {
                    album_id: album_id.to_owned(),
                    recording_id: recording.id,
                    sequence_number: db::sequence_number(index)?,
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

/// Insert an ensemble's member rows, replacing or creating them in their
/// given order.
fn insert_ensemble_persons(
    connection: &mut SqliteConnection,
    ensemble_id: &str,
    persons: Vec<RecordingPerson>,
) -> Result<()> {
    for (index, member) in persons.into_iter().enumerate() {
        let ensemble_person_data = tables::EnsemblePerson {
            ensemble_id: ensemble_id.to_string(),
            person_id: member.person.id,
            role_id: member.role.map(|r| r.id),
            instrument_id: member.instrument.map(|i| i.id),
            sequence_number: db::sequence_number(index)?,
        };

        diesel::insert_into(ensemble_persons::table)
            .values(&ensemble_person_data)
            .execute(connection)?;
    }

    Ok(())
}

/// Insert a recording's performing-person rows in their given order.
fn insert_recording_persons(
    connection: &mut SqliteConnection,
    recording_id: &str,
    performers: Vec<RecordingPerson>,
) -> Result<()> {
    for (index, performer) in performers.into_iter().enumerate() {
        let recording_person_data = tables::RecordingPerson {
            recording_id: recording_id.to_string(),
            person_id: performer.person.id,
            role_id: performer.role.map(|r| r.id),
            instrument_id: performer.instrument.map(|i| i.id),
            sequence_number: db::sequence_number(index)?,
        };

        diesel::insert_into(recording_persons::table)
            .values(&recording_person_data)
            .execute(connection)?;
    }

    Ok(())
}

/// Insert a recording's performing-ensemble rows in their given order.
fn insert_recording_ensembles(
    connection: &mut SqliteConnection,
    recording_id: &str,
    ensembles: Vec<RecordingEnsemble>,
) -> Result<()> {
    for (index, ensemble) in ensembles.into_iter().enumerate() {
        let recording_ensemble_data = tables::RecordingEnsemble {
            recording_id: recording_id.to_string(),
            ensemble_id: ensemble.ensemble.id,
            role_id: ensemble.role.map(|r| r.id),
            sequence_number: db::sequence_number(index)?,
        };

        diesel::insert_into(recording_ensembles::table)
            .values(&recording_ensemble_data)
            .execute(connection)?;
    }

    Ok(())
}
