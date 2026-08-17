use std::{
    ffi::{OsStr, OsString},
    fs::{self},
    path::{Path, PathBuf},
};

use anyhow::{bail, Error, Result};
use diesel::{dsl::exists, prelude::*, QueryDsl, SqliteConnection};

use super::Library;
use crate::db::{
    self,
    models::*,
    schema::*,
    tables::{self, Source},
    TranslatedString,
};
use crate::error::{EntityKind, LibraryError};

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

    pub fn create_tag(
        &self,
        name: TranslatedString,
        takes_value: bool,
        private: bool,
        enable_updates: bool,
    ) -> Result<Tag> {
        let connection = &mut *self.conn();

        let now = db::now();

        let tag = Tag {
            tag_id: db::generate_id(),
            name,
            takes_value,
            source: Source::User,
            created_at: now,
            edited_at: now,
            last_used_at: now,
            enable_updates,
            private,
        };

        diesel::insert_into(tags::table)
            .values(&tag)
            .execute(connection)?;

        self.changed();

        Ok(tag)
    }

    /// Whether any work or recording is tagged with this tag.
    pub fn tag_is_in_use(&self, tag_id: &str) -> Result<bool> {
        let connection = &mut *self.conn();
        Self::tag_is_in_use_priv(connection, tag_id)
    }

    /// The body of [`Library::tag_is_in_use`], for callers that already hold the
    /// connection. Taking it twice would deadlock: the guard lives until the end
    /// of the enclosing function, and dropping the `&mut` to it does not release
    /// it.
    fn tag_is_in_use_priv(connection: &mut SqliteConnection, tag_id: &str) -> Result<bool> {
        Ok(diesel::select(exists(
            work_tags::table.filter(work_tags::tag_id.eq(tag_id)),
        ))
        .get_result::<bool>(connection)?
            || diesel::select(exists(
                recording_tags::table.filter(recording_tags::tag_id.eq(tag_id)),
            ))
            .get_result::<bool>(connection)?)
    }

    /// Update a tag.
    ///
    /// Whether a tag takes a value cannot be changed once anything is tagged
    /// with it. It decides what the existing assignments mean and how they are
    /// found: dropping the value would discard every value already recorded,
    /// and adding one would leave every existing assignment without the value
    /// that a valued tag is searched by. A tag nothing uses yet can still be
    /// corrected.
    pub fn update_tag(
        &self,
        id: &str,
        name: TranslatedString,
        takes_value: bool,
        private: bool,
        enable_updates: bool,
    ) -> Result<(), LibraryError> {
        let connection = &mut *self.conn();

        let previous = tags::table
            .filter(tags::tag_id.eq(id))
            .select(tags::takes_value)
            .first::<bool>(connection)
            .map_err(anyhow::Error::from)?;

        if previous != takes_value && Self::tag_is_in_use_priv(connection, id)? {
            return Err(LibraryError::StillReferenced(EntityKind::Tag));
        }

        let now = db::now();

        diesel::update(tags::table)
            .filter(tags::tag_id.eq(id))
            .set((
                tags::name.eq(name),
                tags::takes_value.eq(takes_value),
                tags::private.eq(private),
                tags::edited_at.eq(now),
                tags::last_used_at.eq(now),
                tags::enable_updates.eq(enable_updates),
            ))
            .execute(connection)
            .map_err(anyhow::Error::from)?;

        self.changed();

        Ok(())
    }

    pub fn delete_tag(&self, tag_id: &str) -> Result<(), LibraryError> {
        let connection = &mut *self.conn();

        diesel::delete(tags::table)
            .filter(tags::tag_id.eq(tag_id))
            .execute(connection)
            .map_err(|err| LibraryError::from_delete(EntityKind::Tag, err))?;

        self.changed();

        Ok(())
    }

    /// Replace a work's tag assignments.
    ///
    /// Assignments are keyed by `(work_id, sequence_number)`, so they are
    /// rewritten wholesale rather than diffed, matching how the other ordered
    /// relations of a work are updated.
    fn set_work_tags(
        connection: &mut SqliteConnection,
        work_id: &str,
        tags: Vec<TagValue>,
    ) -> Result<()> {
        diesel::delete(work_tags::table)
            .filter(work_tags::work_id.eq(work_id))
            .execute(connection)?;

        for (index, tag_value) in tags.into_iter().enumerate() {
            let work_tag_data = tables::WorkTag {
                work_id: work_id.to_string(),
                tag_id: tag_value.tag.tag_id,
                value: tag_value.value.filter(|_| tag_value.tag.takes_value),
                sequence_number: index as i32,
            };

            diesel::insert_into(work_tags::table)
                .values(&work_tag_data)
                .execute(connection)?;
        }

        Ok(())
    }

    /// Replace a recording's tag assignments. See [`Library::set_work_tags`].
    fn set_recording_tags(
        connection: &mut SqliteConnection,
        recording_id: &str,
        tags: Vec<TagValue>,
    ) -> Result<()> {
        diesel::delete(recording_tags::table)
            .filter(recording_tags::recording_id.eq(recording_id))
            .execute(connection)?;

        for (index, tag_value) in tags.into_iter().enumerate() {
            let recording_tag_data = tables::RecordingTag {
                recording_id: recording_id.to_string(),
                tag_id: tag_value.tag.tag_id,
                value: tag_value.value.filter(|_| tag_value.tag.takes_value),
                sequence_number: index as i32,
            };

            diesel::insert_into(recording_tags::table)
                .values(&recording_tag_data)
                .execute(connection)?;
        }

        Ok(())
    }

    pub fn create_work(
        &self,
        name: TranslatedString,
        parts: Vec<Work>,
        persons: Vec<Composer>,
        instruments: Vec<Instrument>,
        tags: Vec<TagValue>,
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
                enable_updates,
            )
        })?;

        self.changed();

        Ok(work)
    }

    fn create_work_priv(
        connection: &mut SqliteConnection,
        name: TranslatedString,
        parts: Vec<Work>,
        persons: Vec<Composer>,
        instruments: Vec<Instrument>,
        tags: Vec<TagValue>,
        parent_work_id: Option<&str>,
        sequence_number: Option<i32>,
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
            relates_to: None,
        };

        diesel::insert_into(works::table)
            .values(&work_data)
            .execute(connection)?;

        for (index, part) in parts.into_iter().enumerate() {
            Self::create_work_priv(
                connection,
                part.name,
                part.parts,
                part.persons,
                part.instruments,
                part.tags,
                Some(&work_id),
                Some(index as i32),
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
                enable_updates,
            )
        })?;

        self.changed();

        Ok(())
    }

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
        persons: Vec<(Person, Option<Instrument>)>,
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

            for (index, (person, instrument)) in persons.into_iter().enumerate() {
                let ensemble_person_data = tables::EnsemblePerson {
                    ensemble_id: ensemble_data.ensemble_id.clone(),
                    person_id: person.person_id,
                    instrument_id: instrument.map(|i| i.instrument_id),
                    sequence_number: index as i32,
                    role_id: None,
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
        persons: Vec<(Person, Option<Instrument>)>,
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

            for (index, (person, instrument)) in persons.into_iter().enumerate() {
                let ensemble_person_data = tables::EnsemblePerson {
                    ensemble_id: id.to_string(),
                    person_id: person.person_id,
                    instrument_id: instrument.map(|i| i.instrument_id),
                    sequence_number: index as i32,
                    role_id: None,
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

    pub fn delete_recording_and_tracks(&self, recording_id: &str) -> Result<()> {
        let connection = &mut *self.conn();

        // Delete from library first to avoid orphan tracks in case of file
        // system related errors. The track list is read inside the transaction
        // so that it cannot go stale between reading and deleting.

        let tracks = connection.transaction::<Vec<tables::Track>, Error, _>(|connection| {
            let tracks = tracks::table
                .filter(tracks::recording_id.eq(recording_id))
                .load::<tables::Track>(connection)?;

            for track in &tracks {
                diesel::delete(track_works::table)
                    .filter(track_works::track_id.eq(&track.track_id))
                    .execute(connection)?;

                diesel::delete(tracks::table)
                    .filter(tracks::track_id.eq(&track.track_id))
                    .execute(connection)?;
            }

            diesel::delete(recordings::table)
                .filter(recordings::recording_id.eq(recording_id))
                .execute(connection)?;

            Ok(tracks)
        })?;

        // The database no longer references these files. A failure to remove one
        // of them leaves an unreferenced file behind, which must not fail the
        // operation or prevent the remaining files from being removed.
        let library_path = PathBuf::from(self.folder());
        for track in tracks {
            let path = library_path.join(&track.path);
            if let Err(err) = fs::remove_file(&path) {
                log::warn!("Failed to remove track file {}: {err}", path.display());
            }
        }

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

    /// Save the complete track list of `recording_id` in one transaction.
    ///
    /// `tracks` is the track list as it should be afterwards, in order; each
    /// track's `recording_index` is its position in that list, so a save can
    /// never leave duplicate or gapped indices behind. `deleted_tracks` are the
    /// tracks the caller removed from the recording.
    ///
    /// Either all of the deletions, updates and imports are applied or none of
    /// them are, so a failed save can simply be retried.
    // TODO: Support mediums, think about albums.
    pub fn set_recording_tracks(
        &self,
        recording_id: &str,
        tracks: Vec<TrackUpdate>,
        deleted_tracks: &[Track],
    ) -> Result<()> {
        let tracks = tracks
            .into_iter()
            .enumerate()
            .map(|(index, track)| (index as i32, track))
            .collect();

        self.apply_track_changes(Some(recording_id), tracks, deleted_tracks)
    }

    /// Import a track into the music library.
    // TODO: Support mediums.
    pub fn import_track(
        &self,
        path: impl AsRef<Path>,
        recording_id: &str,
        recording_index: i32,
        works: Vec<Work>,
    ) -> Result<()> {
        let track = TrackUpdate::New {
            path: path.as_ref().to_owned(),
            works,
        };

        self.apply_track_changes(Some(recording_id), vec![(recording_index, track)], &[])
    }

    // TODO: Support mediums, think about albums.
    pub fn delete_track(&self, track: &Track) -> Result<()> {
        // No recording is needed: without an import there is no file to name.
        self.apply_track_changes(None, Vec::new(), std::slice::from_ref(track))
    }

    // TODO: Support mediums, think about albums.
    pub fn update_track(
        &self,
        track_id: &str,
        recording_index: i32,
        works: Vec<Work>,
    ) -> Result<()> {
        let track = TrackUpdate::Existing {
            track_id: track_id.to_owned(),
            works,
        };

        self.apply_track_changes(None, vec![(recording_index, track)], &[])
    }

    /// Record that a track was played.
    pub fn track_played(&self, track_id: &str) -> Result<()> {
        let connection = &mut *self.conn();

        connection.transaction::<(), diesel::result::Error, _>(|connection| {
            let recording_id = tracks::table
                .filter(tracks::track_id.eq(track_id))
                .select(tracks::recording_id)
                .first::<String>(connection)?;

            diesel::insert_into(plays::table)
                .values(tables::Play {
                    play_id: db::generate_id(),
                    track_id: Some(track_id.to_owned()),
                    recording_id,
                    played_at: db::now(),
                })
                .execute(connection)?;

            Ok(())
        })?;

        self.changed();

        Ok(())
    }

    /// Apply a batch of track changes in a single transaction.
    ///
    /// Each entry of `tracks` carries the `recording_index` its track should end
    /// up with. `recording_id` is only needed to name the files of newly
    /// imported tracks and may be `None` for a batch that imports nothing.
    ///
    /// This is the only place that writes to the track tables, so that every
    /// track change gets the same all-or-nothing guarantee.
    fn apply_track_changes(
        &self,
        recording_id: Option<&str>,
        tracks: Vec<(i32, TrackUpdate)>,
        deleted_tracks: &[Track],
    ) -> Result<()> {
        let folder = PathBuf::from(self.folder());

        // Copy the file of every new track next to its destination before
        // touching the database. Nothing is moved into place until the
        // transaction is about to commit, so a failure can leave neither a
        // track row pointing at a missing file nor an unreferenced file in the
        // library folder.
        let mut staged = Vec::new();
        let mut prepared = Vec::with_capacity(tracks.len());

        for (recording_index, track) in tracks {
            match track {
                TrackUpdate::Existing { track_id, works } => {
                    prepared.push((recording_index, PreparedTrack::Existing { track_id, works }));
                }
                TrackUpdate::New { path, works } => {
                    let Some(recording_id) = recording_id else {
                        clean_up_staged(&staged, 0);
                        bail!("Cannot import a track without the recording it belongs to");
                    };

                    // TODO: Human interpretable filenames?
                    let library_path = unused_track_path(
                        &folder,
                        recording_id,
                        recording_index,
                        path.extension(),
                        &staged,
                    );

                    let to_path = folder.join(&library_path);
                    let mut tmp_path = to_path.clone();
                    tmp_path.as_mut_os_string().push(".part");

                    if let Err(err) = fs::copy(&path, &tmp_path) {
                        clean_up_staged(&staged, 0);
                        return Err(err.into());
                    }

                    staged.push(StagedFile { tmp_path, to_path });

                    prepared.push((
                        recording_index,
                        PreparedTrack::New {
                            track_id: db::generate_id(),
                            library_path,
                            works,
                        },
                    ));
                }
            }
        }

        let connection = &mut *self.conn();
        let now = db::now();
        let mut renamed = 0;

        let result = connection.transaction::<(), Error, _>(|connection| {
            for track in deleted_tracks {
                diesel::delete(track_works::table)
                    .filter(track_works::track_id.eq(&track.track_id))
                    .execute(connection)?;

                diesel::delete(tracks::table)
                    .filter(tracks::track_id.eq(&track.track_id))
                    .execute(connection)?;
            }

            for (recording_index, track) in prepared {
                let (track_id, works) = match track {
                    PreparedTrack::Existing { track_id, works } => {
                        diesel::update(tracks::table)
                            .filter(tracks::track_id.eq(&track_id))
                            .set((
                                tracks::recording_index.eq(recording_index),
                                tracks::edited_at.eq(now),
                                tracks::last_used_at.eq(now),
                            ))
                            .execute(connection)?;

                        diesel::delete(track_works::table)
                            .filter(track_works::track_id.eq(&track_id))
                            .execute(connection)?;

                        (track_id, works)
                    }
                    PreparedTrack::New {
                        track_id,
                        library_path,
                        works,
                    } => {
                        let track_data = tables::Track {
                            track_id: track_id.clone(),
                            // A batch only contains new tracks if it knows the
                            // recording they belong to.
                            recording_id: recording_id.unwrap_or_default().to_owned(),
                            recording_index,
                            medium_id: None,
                            medium_index: None,
                            path: library_path.into(),
                            created_at: now,
                            edited_at: now,
                            last_used_at: now,
                        };

                        diesel::insert_into(tracks::table)
                            .values(&track_data)
                            .execute(connection)?;

                        (track_id, works)
                    }
                };

                for (index, work) in works.into_iter().enumerate() {
                    let track_work_data = tables::TrackWork {
                        track_id: track_id.clone(),
                        work_id: work.work_id,
                        sequence_number: index as i32,
                    };

                    diesel::insert_into(track_works::table)
                        .values(&track_work_data)
                        .execute(connection)?;
                }
            }

            // Moving the files into place is the last fallible step before the
            // commit, so that a failure here rolls the database back instead of
            // leaving track rows pointing at missing files.
            for file in &staged {
                fs::rename(&file.tmp_path, &file.to_path)?;
                renamed += 1;
            }

            Ok(())
        });

        if let Err(err) = result {
            clean_up_staged(&staged, renamed);
            return Err(err);
        }

        // The database no longer references the deleted tracks' files. A failure
        // to remove one of them leaves an unreferenced file behind, which must
        // not fail the operation or prevent the remaining files from being
        // removed.
        for track in deleted_tracks {
            let path = folder.join(&track.path);
            if let Err(err) = fs::remove_file(&path) {
                log::warn!("Failed to remove track file {}: {err}", path.display());
            }
        }

        self.changed();

        Ok(())
    }
}

/// One track of a recording as it should exist after
/// [`Library::set_recording_tracks`].
#[derive(Clone, Debug)]
pub enum TrackUpdate {
    /// A track that is already in the library. Its works and its position
    /// within the recording are replaced; its file is left alone.
    Existing { track_id: String, works: Vec<Work> },

    /// A file outside of the library that is to be imported.
    New { path: PathBuf, works: Vec<Work> },
}

/// A track of a batch with everything resolved that has to be decided before
/// the transaction is started.
enum PreparedTrack {
    Existing {
        track_id: String,
        works: Vec<Work>,
    },
    New {
        track_id: String,
        library_path: PathBuf,
        works: Vec<Work>,
    },
}

/// The file of a new track, copied next to its destination and waiting to be
/// moved into place.
struct StagedFile {
    tmp_path: PathBuf,
    to_path: PathBuf,
}

/// Remove the files that a failed batch left behind.
///
/// The first `renamed` files had already been moved to their destination when
/// the batch failed, the remaining ones are still waiting next to it.
fn clean_up_staged(staged: &[StagedFile], renamed: usize) {
    for (index, file) in staged.iter().enumerate() {
        let path = if index < renamed {
            &file.to_path
        } else {
            &file.tmp_path
        };

        if let Err(err) = fs::remove_file(path) {
            log::warn!(
                "Failed to remove partially imported track {}: {err}",
                path.display()
            );
        }
    }
}

/// Build a library relative file name for a new track of `recording_id` at
/// `recording_index` that no file in `folder` and no file staged by the same
/// batch is using yet.
///
/// The name only serves human orientation. Tracks keep the file they were
/// imported with even when they are renumbered later, so the obvious name for
/// an index can already belong to another track, and using it anyway would
/// overwrite that track's audio.
fn unused_track_path(
    folder: &Path,
    recording_id: &str,
    recording_index: i32,
    extension: Option<&OsStr>,
    staged: &[StagedFile],
) -> PathBuf {
    let mut suffix = 0;

    loop {
        let mut filename = OsString::from(recording_id);
        filename.push("_");
        filename.push(format!("{recording_index:02}"));

        if suffix > 0 {
            filename.push(format!("_{suffix}"));
        }

        if let Some(extension) = extension {
            filename.push(".");
            filename.push(extension);
        }

        let library_path = PathBuf::from(filename);
        let to_path = folder.join(&library_path);
        let mut tmp_path = to_path.clone();
        tmp_path.as_mut_os_string().push(".part");

        if !to_path.exists()
            && !tmp_path.exists()
            && !staged.iter().any(|file| file.to_path == to_path)
        {
            return library_path;
        }

        suffix += 1;
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use tempfile::TempDir;

    use super::*;

    fn translated(name: &str) -> TranslatedString {
        let mut translations = HashMap::new();
        translations.insert("generic".to_string(), name.to_string());
        TranslatedString(translations)
    }

    fn library(dir: &TempDir, cache_dir: &TempDir) -> Library {
        Library::new(dir.path(), cache_dir.path()).unwrap()
    }

    /// Write a file outside of the library that can be imported as a track.
    fn track_source(source_dir: &TempDir, name: &str) -> PathBuf {
        let path = source_dir.path().join(format!("{name}.mp3"));
        fs::write(&path, format!("audio of {name}").as_bytes()).unwrap();
        path
    }

    /// Create a recording with `n_tracks` imported tracks.
    fn recording_with_tracks(
        library: &Library,
        source_dir: &TempDir,
        n_tracks: usize,
    ) -> (Recording, Work) {
        let person = library
            .create_person(translated("Beethoven"), true)
            .unwrap();
        let work = library
            .create_work(
                translated("Symphony No. 5"),
                Vec::new(),
                vec![Composer { person, role: None }],
                Vec::new(),
                Vec::new(),
                true,
            )
            .unwrap();
        let recording = library
            .create_recording(work.clone(), Vec::new(), Vec::new(), Vec::new(), true)
            .unwrap();

        for index in 0..n_tracks {
            let source = track_source(source_dir, &format!("track_{index}"));
            library
                .import_track(
                    &source,
                    &recording.recording_id,
                    index as i32,
                    vec![work.clone()],
                )
                .unwrap();
        }

        (recording, work)
    }

    /// The names of all files in the library folder, sorted.
    fn library_files(dir: &TempDir) -> Vec<String> {
        let mut names = fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        names.sort();
        names
    }

    /// Run `operation` and assert that it emitted exactly one change notification.
    ///
    /// Every public mutator must notify subscribers, otherwise the UI silently
    /// keeps showing stale data.
    fn assert_notifies<T, E: std::fmt::Debug>(
        library: &Library,
        what: &str,
        operation: impl FnOnce() -> Result<T, E>,
    ) -> T {
        let receiver = library.subscribe_changed();
        let value = operation().unwrap_or_else(|err| panic!("{what} failed: {err:?}"));
        assert!(
            receiver.try_recv().is_ok(),
            "{what} did not emit a change notification"
        );
        value
    }

    /// Whether a tag takes a value decides what its existing assignments mean,
    /// so it is fixed once anything is tagged with it. A tag nothing uses yet
    /// can still be corrected.
    #[test]
    fn a_tag_in_use_keeps_its_kind() {
        let dir = TempDir::new().unwrap();
        let cache_dir = TempDir::new().unwrap();
        let library = library(&dir, &cache_dir);

        let tag = library
            .create_tag(translated("Year"), true, false, true)
            .unwrap();

        // Nothing uses it yet, so it can still be corrected.
        library
            .update_tag(&tag.tag_id, translated("Year"), false, false, true)
            .expect("an unused tag may change kind");
        library
            .update_tag(&tag.tag_id, translated("Year"), true, false, true)
            .unwrap();

        let person = library.create_person(translated("Bach"), true).unwrap();
        let work = library
            .create_work(
                translated("Toccata"),
                Vec::new(),
                vec![Composer { person, role: None }],
                Vec::new(),
                Vec::new(),
                true,
            )
            .unwrap();
        library
            .create_recording(
                work,
                Vec::new(),
                Vec::new(),
                vec![TagValue {
                    tag: tag.clone(),
                    value: Some("1963".to_string()),
                }],
                true,
            )
            .unwrap();

        let err = library
            .update_tag(&tag.tag_id, translated("Year"), false, false, true)
            .expect_err("a tag in use must keep its kind");
        assert!(
            matches!(err, LibraryError::StillReferenced(EntityKind::Tag)),
            "unexpected error: {err:?}"
        );

        // The rename is refused along with the kind change, and no value is lost.
        let connection = &mut *library.conn();
        let value = recording_tags::table
            .select(recording_tags::value)
            .first::<Option<String>>(connection)
            .unwrap();
        assert_eq!(value.as_deref(), Some("1963"));

        let takes_value = tags::table
            .filter(tags::tag_id.eq(&tag.tag_id))
            .select(tags::takes_value)
            .first::<bool>(connection)
            .unwrap();
        assert!(takes_value);
    }

    #[test]
    fn every_mutator_emits_a_change_notification() {
        let dir = TempDir::new().unwrap();
        let cache_dir = TempDir::new().unwrap();
        let library = library(&dir, &cache_dir);

        let person = assert_notifies(&library, "create_person", || {
            library.create_person(translated("Beethoven"), true)
        });
        let role = assert_notifies(&library, "create_role", || {
            library.create_role(translated("Composer"), true)
        });
        let instrument = assert_notifies(&library, "create_instrument", || {
            library.create_instrument(translated("Piano"), true)
        });
        let tag = assert_notifies(&library, "create_tag", || {
            library.create_tag(translated("Baroque"), false, false, true)
        });

        assert_notifies(&library, "update_person", || {
            library.update_person(&person.person_id, translated("van Beethoven"), true)
        });
        assert_notifies(&library, "update_role", || {
            library.update_role(&role.role_id, translated("Arranger"), true)
        });
        assert_notifies(&library, "update_instrument", || {
            library.update_instrument(&instrument.instrument_id, translated("Fortepiano"), true)
        });
        assert_notifies(&library, "update_tag", || {
            library.update_tag(&tag.tag_id, translated("Classical"), false, false, true)
        });

        let composer = Composer {
            person: person.clone(),
            role: None,
        };

        let work = assert_notifies(&library, "create_work", || {
            library.create_work(
                translated("Symphony No. 5"),
                Vec::new(),
                vec![composer.clone()],
                vec![instrument.clone()],
                Vec::new(),
                true,
            )
        });
        assert_notifies(&library, "update_work", || {
            library.update_work(
                &work.work_id,
                translated("Symphony No. 6"),
                Vec::new(),
                vec![composer.clone()],
                Vec::new(),
                Vec::new(),
                true,
            )
        });

        let ensemble = assert_notifies(&library, "create_ensemble", || {
            library.create_ensemble(
                translated("Berliner Philharmoniker"),
                vec![(person.clone(), Some(instrument.clone()))],
                true,
            )
        });
        assert_notifies(&library, "update_ensemble", || {
            library.update_ensemble(
                &ensemble.ensemble_id,
                translated("Wiener Philharmoniker"),
                Vec::new(),
                true,
            )
        });

        let recording = assert_notifies(&library, "create_recording", || {
            library.create_recording(work.clone(), Vec::new(), Vec::new(), Vec::new(), true)
        });
        assert_notifies(&library, "update_recording", || {
            library.update_recording(
                &recording.recording_id,
                work.clone(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                true,
            )
        });

        let album = assert_notifies(&library, "create_album", || {
            library.create_album(translated("The Symphonies"), vec![recording.clone()], true)
        });
        assert_notifies(&library, "update_album", || {
            library.update_album(&album.album_id, translated("Complete"), Vec::new(), true)
        });

        let track_source = dir.path().join("source_track.mp3");
        fs::write(&track_source, b"not actually audio").unwrap();

        assert_notifies(&library, "import_track", || {
            library.import_track(
                &track_source,
                &recording.recording_id,
                0,
                vec![work.clone()],
            )
        });
        let track = library
            .tracks_for_recording(&recording.recording_id)
            .unwrap()
            .remove(0);
        assert_notifies(&library, "update_track", || {
            library.update_track(&track.track_id, 1, vec![work.clone()])
        });
        assert_notifies(&library, "set_recording_tracks", || {
            library.set_recording_tracks(
                &recording.recording_id,
                vec![TrackUpdate::Existing {
                    track_id: track.track_id.clone(),
                    works: vec![work.clone()],
                }],
                &[],
            )
        });
        assert_notifies(&library, "delete_track", || library.delete_track(&track));

        assert_notifies(&library, "delete_album", || {
            library.delete_album(&album.album_id)
        });
        assert_notifies(&library, "delete_recording", || {
            library.delete_recording(&recording.recording_id)
        });
        assert_notifies(&library, "delete_ensemble", || {
            library.delete_ensemble(&ensemble.ensemble_id)
        });
        assert_notifies(&library, "delete_work", || {
            library.delete_work(&work.work_id)
        });
        assert_notifies(&library, "delete_instrument", || {
            library.delete_instrument(&instrument.instrument_id)
        });
        assert_notifies(&library, "delete_role", || {
            library.delete_role(&role.role_id)
        });
        assert_notifies(&library, "delete_person", || {
            library.delete_person(&person.person_id)
        });
    }

    /// Deleting something still in use must be reported as such, not as an
    /// opaque foreign key error that the UI can only show verbatim.
    #[test]
    fn deleting_a_referenced_item_reports_that_it_is_still_used() {
        let dir = TempDir::new().unwrap();
        let cache_dir = TempDir::new().unwrap();
        let library = library(&dir, &cache_dir);

        let person = library
            .create_person(translated("Beethoven"), true)
            .unwrap();
        let work = library
            .create_work(
                translated("Symphony No. 5"),
                Vec::new(),
                vec![Composer {
                    person: person.clone(),
                    role: None,
                }],
                Vec::new(),
                Vec::new(),
                true,
            )
            .unwrap();

        assert!(matches!(
            library.delete_person(&person.person_id),
            Err(LibraryError::StillReferenced(EntityKind::Person))
        ));

        // Once nothing refers to the person any more, the delete succeeds.
        library.delete_work(&work.work_id).unwrap();
        library.delete_person(&person.person_id).unwrap();
    }

    #[test]
    fn delete_recording_and_tracks_notifies_and_removes_files() {
        let dir = TempDir::new().unwrap();
        let cache_dir = TempDir::new().unwrap();
        let library = library(&dir, &cache_dir);

        let person = library
            .create_person(translated("Beethoven"), true)
            .unwrap();
        let work = library
            .create_work(
                translated("Symphony No. 5"),
                Vec::new(),
                vec![Composer { person, role: None }],
                Vec::new(),
                Vec::new(),
                true,
            )
            .unwrap();
        let recording = library
            .create_recording(work.clone(), Vec::new(), Vec::new(), Vec::new(), true)
            .unwrap();

        let track_source = dir.path().join("source_track.mp3");
        fs::write(&track_source, b"not actually audio").unwrap();
        library
            .import_track(&track_source, &recording.recording_id, 0, vec![work])
            .unwrap();

        let track_path = dir.path().join(
            &library
                .tracks_for_recording(&recording.recording_id)
                .unwrap()[0]
                .path,
        );
        assert!(track_path.exists());

        assert_notifies(&library, "delete_recording_and_tracks", || {
            library.delete_recording_and_tracks(&recording.recording_id)
        });

        assert!(!track_path.exists(), "track file should have been removed");
        assert!(library
            .tracks_for_recording(&recording.recording_id)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn failed_track_import_leaves_no_file_behind() {
        let dir = TempDir::new().unwrap();
        let cache_dir = TempDir::new().unwrap();
        let library = library(&dir, &cache_dir);

        let track_source = dir.path().join("source_track.mp3");
        fs::write(&track_source, b"not actually audio").unwrap();

        // No such recording, so the insert violates the foreign key and the
        // transaction rolls back.
        let before = fs::read_dir(dir.path()).unwrap().count();
        assert!(library
            .import_track(&track_source, "does-not-exist", 0, Vec::new())
            .is_err());

        assert_eq!(
            fs::read_dir(dir.path()).unwrap().count(),
            before,
            "a failed import must not leave a file in the library folder"
        );
    }

    #[test]
    fn set_recording_tracks_saves_deletes_updates_and_imports_in_one_go() {
        let dir = TempDir::new().unwrap();
        let cache_dir = TempDir::new().unwrap();
        let source_dir = TempDir::new().unwrap();
        let library = library(&dir, &cache_dir);

        let (recording, work) = recording_with_tracks(&library, &source_dir, 3);
        let tracks = library
            .tracks_for_recording(&recording.recording_id)
            .unwrap();
        let removed_path = dir.path().join(&tracks[2].path);
        assert!(removed_path.exists());

        // Reorder the first two tracks, import a new one between them and drop
        // the third one, all at once.
        let source = track_source(&source_dir, "added");
        assert_notifies(&library, "set_recording_tracks", || {
            library.set_recording_tracks(
                &recording.recording_id,
                vec![
                    TrackUpdate::Existing {
                        track_id: tracks[1].track_id.clone(),
                        works: vec![work.clone()],
                    },
                    TrackUpdate::New {
                        path: source.clone(),
                        works: vec![work.clone()],
                    },
                    TrackUpdate::Existing {
                        track_id: tracks[0].track_id.clone(),
                        works: vec![work.clone()],
                    },
                ],
                &[tracks[2].clone()],
            )
        });

        let saved = library
            .tracks_for_recording(&recording.recording_id)
            .unwrap();
        assert_eq!(saved.len(), 3);
        assert_eq!(saved[0].track_id, tracks[1].track_id);
        assert_eq!(saved[2].track_id, tracks[0].track_id);

        // The existing tracks keep their files even though they were renumbered.
        assert_eq!(saved[0].path, tracks[1].path);
        assert_eq!(saved[2].path, tracks[0].path);

        assert_eq!(
            fs::read(dir.path().join(&saved[1].path)).unwrap(),
            b"audio of added"
        );
        assert!(
            !removed_path.exists(),
            "the removed track's file should be gone"
        );

        assert!(
            library_files(&dir)
                .iter()
                .all(|name| !name.ends_with(".part")),
            "no temporary file may be left behind"
        );
    }

    /// The regression test for saves that could previously fail half way
    /// through, leaving some tracks deleted and others not.
    #[test]
    fn failed_track_save_changes_nothing() {
        let dir = TempDir::new().unwrap();
        let cache_dir = TempDir::new().unwrap();
        let source_dir = TempDir::new().unwrap();
        let library = library(&dir, &cache_dir);

        let (recording, work) = recording_with_tracks(&library, &source_dir, 2);
        let tracks = library
            .tracks_for_recording(&recording.recording_id)
            .unwrap();
        let before = library_files(&dir);

        // The second track's works reference a work that does not exist, so the
        // insert fails once the deletion and the import have already been
        // applied within the transaction.
        let mut missing_work = work.clone();
        missing_work.work_id = "does-not-exist".to_owned();

        let source = track_source(&source_dir, "added");
        assert!(library
            .set_recording_tracks(
                &recording.recording_id,
                vec![
                    TrackUpdate::New {
                        path: source,
                        works: vec![work.clone()],
                    },
                    TrackUpdate::Existing {
                        track_id: tracks[1].track_id.clone(),
                        works: vec![missing_work],
                    },
                ],
                &[tracks[0].clone()],
            )
            .is_err());

        let after = library
            .tracks_for_recording(&recording.recording_id)
            .unwrap();
        assert_eq!(
            after.len(),
            2,
            "the deleted track must have been restored by the rollback"
        );
        assert_eq!(after[0].track_id, tracks[0].track_id);
        assert_eq!(after[1].track_id, tracks[1].track_id);

        assert_eq!(
            library_files(&dir),
            before,
            "a failed save must leave the library folder untouched"
        );
    }

    /// Rolling a batch back has to remove the files it had already moved into
    /// place as well as the ones still waiting next to their destination.
    #[test]
    fn clean_up_staged_removes_renamed_and_staged_files() {
        let dir = TempDir::new().unwrap();

        let mut staged = Vec::new();
        for index in 0..2 {
            let to_path = dir.path().join(format!("track_{index}"));
            let mut tmp_path = to_path.clone();
            tmp_path.as_mut_os_string().push(".part");
            staged.push(StagedFile { tmp_path, to_path });
        }

        // The first file had been renamed into place, the second one had not.
        fs::write(&staged[0].to_path, b"renamed").unwrap();
        fs::write(&staged[1].tmp_path, b"still staged").unwrap();

        clean_up_staged(&staged, 1);

        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    /// Renumbering tracks means a new track can be given the index of an
    /// existing one, whose file must not be overwritten.
    #[test]
    fn import_track_does_not_overwrite_another_tracks_file() {
        let dir = TempDir::new().unwrap();
        let cache_dir = TempDir::new().unwrap();
        let source_dir = TempDir::new().unwrap();
        let library = library(&dir, &cache_dir);

        let (recording, work) = recording_with_tracks(&library, &source_dir, 0);

        for name in ["first", "second"] {
            let source = track_source(&source_dir, name);
            library
                .import_track(&source, &recording.recording_id, 0, vec![work.clone()])
                .unwrap();
        }

        let tracks = library
            .tracks_for_recording(&recording.recording_id)
            .unwrap();
        assert_eq!(tracks.len(), 2);
        assert_ne!(
            tracks[0].path, tracks[1].path,
            "two tracks must not share a file"
        );

        let contents = tracks
            .iter()
            .map(|track| fs::read(dir.path().join(&track.path)).unwrap())
            .collect::<Vec<_>>();
        assert!(contents.contains(&b"audio of first".to_vec()));
        assert!(contents.contains(&b"audio of second".to_vec()));
    }

    #[test]
    fn update_ensemble_is_atomic() {
        let dir = TempDir::new().unwrap();
        let cache_dir = TempDir::new().unwrap();
        let library = library(&dir, &cache_dir);

        let person = library
            .create_person(translated("Beethoven"), true)
            .unwrap();
        let ensemble = library
            .create_ensemble(
                translated("Berliner Philharmoniker"),
                vec![(person.clone(), None)],
                true,
            )
            .unwrap();
        assert_eq!(ensemble.persons.len(), 1);

        // The second member references a person that does not exist, so the
        // insert fails after the existing members have already been deleted.
        let mut missing = person.clone();
        missing.person_id = "does-not-exist".to_owned();

        assert!(library
            .update_ensemble(
                &ensemble.ensemble_id,
                translated("Renamed"),
                vec![(person, None), (missing, None)],
                true,
            )
            .is_err());

        let found = library.search_ensembles("Berliner").unwrap();
        assert_eq!(found.len(), 1, "the name must not have been changed");
        assert_eq!(
            found[0].item.persons.len(),
            1,
            "a failed update must not drop the existing members"
        );
    }
}
