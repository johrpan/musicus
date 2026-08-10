use std::{
    ffi::OsString,
    fs::{self},
    path::{Path, PathBuf},
};

use anyhow::{Error, Result};
use diesel::{prelude::*, QueryDsl, SqliteConnection};

use super::Library;
use crate::db::{
    self,
    models::*,
    schema::*,
    tables::{self, Source},
    TranslatedString,
};

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
            last_played_at: None,
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

    pub fn delete_person(&self, person_id: &str) -> Result<()> {
        let connection = &mut *self.conn();

        diesel::delete(persons::table)
            .filter(persons::person_id.eq(person_id))
            .execute(connection)?;

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
            last_played_at: None,
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

    pub fn delete_instrument(&self, instrument_id: &str) -> Result<()> {
        let connection = &mut *self.conn();

        diesel::delete(instruments::table)
            .filter(instruments::instrument_id.eq(instrument_id))
            .execute(connection)?;

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

    pub fn delete_role(&self, role_id: &str) -> Result<()> {
        let connection = &mut *self.conn();

        diesel::delete(roles::table)
            .filter(roles::role_id.eq(role_id))
            .execute(connection)?;

        self.changed();

        Ok(())
    }

    pub fn create_work(
        &self,
        name: TranslatedString,
        parts: Vec<Work>,
        persons: Vec<Composer>,
        instruments: Vec<Instrument>,
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
            last_played_at: None,
            enable_updates,
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
                Some(&work_id),
                Some(index as i32),
                enable_updates,
            )?;
        }

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

        Ok(())
    }

    pub fn delete_work(&self, work_id: &str) -> Result<()> {
        let connection = &mut *self.conn();

        diesel::delete(works::table)
            .filter(works::work_id.eq(work_id))
            .execute(connection)?;

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
                last_played_at: None,
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

    pub fn delete_ensemble(&self, ensemble_id: &str) -> Result<()> {
        let connection = &mut *self.conn();

        diesel::delete(ensembles::table)
            .filter(ensembles::ensemble_id.eq(ensemble_id))
            .execute(connection)?;

        self.changed();

        Ok(())
    }

    pub fn create_recording(
        &self,
        work: Work,
        year: Option<i32>,
        performers: Vec<Performer>,
        ensembles: Vec<EnsemblePerformer>,
        enable_updates: bool,
    ) -> Result<Recording> {
        let connection = &mut *self.conn();

        let recording = connection.transaction::<Recording, Error, _>(|connection| {
            let recording_id = db::generate_id();
            let now = db::now();

            let recording_data = tables::Recording {
                recording_id: recording_id.clone(),
                work_id: work.work_id.clone(),
                year,
                source: Source::User,
                created_at: now,
                edited_at: now,
                last_used_at: now,
                last_played_at: None,
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

            Recording::from_table(recording_data, connection)
        })?;

        self.changed();

        Ok(recording)
    }

    pub fn update_recording(
        &self,
        recording_id: &str,
        work: Work,
        year: Option<i32>,
        performers: Vec<Performer>,
        ensembles: Vec<EnsemblePerformer>,
        enable_updates: bool,
    ) -> Result<()> {
        let connection = &mut *self.conn();

        connection.transaction::<(), Error, _>(|connection| {
            let now = db::now();

            diesel::update(recordings::table)
                .filter(recordings::recording_id.eq(recording_id))
                .set((
                    recordings::work_id.eq(work.work_id),
                    recordings::year.eq(year),
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

            Ok(())
        })?;

        self.changed();

        Ok(())
    }

    pub fn delete_recording(&self, recording_id: &str) -> Result<()> {
        let connection = &mut *self.conn();

        diesel::delete(recordings::table)
            .filter(recordings::recording_id.eq(recording_id))
            .execute(connection)?;

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
                last_played_at: None,
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

    pub fn delete_album(&self, album_id: &str) -> Result<()> {
        let connection = &mut *self.conn();

        diesel::delete(albums::table)
            .filter(albums::album_id.eq(album_id))
            .execute(connection)?;

        self.changed();

        Ok(())
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
        let connection = &mut *self.conn();

        let track_id = db::generate_id();
        let now = db::now();

        // TODO: Human interpretable filenames?
        let mut filename = OsString::from(recording_id);
        filename.push("_");
        filename.push(OsString::from(format!("{recording_index:02}")));
        if let Some(extension) = path.as_ref().extension() {
            filename.push(".");
            filename.push(extension);
        };

        let mut to_path = PathBuf::from(self.folder());
        to_path.push(&filename);
        let library_path = PathBuf::from(filename);

        // Copy to a temporary name first and only move the file into place once
        // the database transaction has committed, so that a failed import does
        // not leave an unreferenced file in the library folder.
        let mut tmp_path = to_path.clone();
        tmp_path.as_mut_os_string().push(".part");

        fs::copy(path, &tmp_path)?;

        let result = connection.transaction::<(), Error, _>(|connection| {
            let track_data = tables::Track {
                track_id: track_id.clone(),
                recording_id: recording_id.to_owned(),
                recording_index,
                medium_id: None,
                medium_index: None,
                path: library_path.into(),
                created_at: now,
                edited_at: now,
                last_used_at: now,
                last_played_at: None,
            };

            diesel::insert_into(tracks::table)
                .values(&track_data)
                .execute(connection)?;

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

            // Moving the file into place is the last fallible step before the
            // commit, so that a failure here rolls the database back instead of
            // leaving a track row pointing at a missing file.
            fs::rename(&tmp_path, &to_path)?;

            Ok(())
        });

        if let Err(err) = result {
            if let Err(err) = fs::remove_file(&tmp_path) {
                log::warn!(
                    "Failed to remove partially imported track {}: {err}",
                    tmp_path.display()
                );
            }

            return Err(err);
        }

        self.changed();

        Ok(())
    }

    // TODO: Support mediums, think about albums.
    pub fn delete_track(&self, track: &Track) -> Result<()> {
        let connection = &mut *self.conn();

        // Delete from the database first to avoid orphan tracks in case of file system
        // related errors.

        connection.transaction::<(), Error, _>(|connection| {
            diesel::delete(track_works::table)
                .filter(track_works::track_id.eq(&track.track_id))
                .execute(connection)?;

            diesel::delete(tracks::table)
                .filter(tracks::track_id.eq(&track.track_id))
                .execute(connection)?;

            Ok(())
        })?;

        // The database no longer references this file. Failing to remove it
        // leaves an unreferenced file behind, which must not fail the deletion.
        let mut path = PathBuf::from(self.folder());
        path.push(&track.path);
        if let Err(err) = fs::remove_file(&path) {
            log::warn!("Failed to remove track file {}: {err}", path.display());
        }

        self.changed();

        Ok(())
    }

    // TODO: Support mediums, think about albums.
    pub fn update_track(
        &self,
        track_id: &str,
        recording_index: i32,
        works: Vec<Work>,
    ) -> Result<()> {
        let connection = &mut *self.conn();

        connection.transaction::<(), Error, _>(|connection| {
            let now = db::now();

            diesel::update(tracks::table)
                .filter(tracks::track_id.eq(track_id.to_owned()))
                .set((
                    tracks::recording_index.eq(recording_index),
                    tracks::edited_at.eq(now),
                    tracks::last_used_at.eq(now),
                ))
                .execute(connection)?;

            diesel::delete(track_works::table)
                .filter(track_works::track_id.eq(track_id))
                .execute(connection)?;

            for (index, work) in works.into_iter().enumerate() {
                let track_work_data = tables::TrackWork {
                    track_id: track_id.to_owned(),
                    work_id: work.work_id,
                    sequence_number: index as i32,
                };

                diesel::insert_into(track_works::table)
                    .values(&track_work_data)
                    .execute(connection)?;
            }

            Ok(())
        })?;

        self.changed();

        Ok(())
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

    /// Run `operation` and assert that it emitted exactly one change notification.
    ///
    /// Every public mutator must notify subscribers, otherwise the UI silently
    /// keeps showing stale data.
    fn assert_notifies<T>(
        library: &Library,
        what: &str,
        operation: impl FnOnce() -> Result<T>,
    ) -> T {
        let receiver = library.subscribe_changed();
        let value = operation().unwrap_or_else(|err| panic!("{what} failed: {err:?}"));
        assert!(
            receiver.try_recv().is_ok(),
            "{what} did not emit a change notification"
        );
        value
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

        assert_notifies(&library, "update_person", || {
            library.update_person(&person.person_id, translated("van Beethoven"), true)
        });
        assert_notifies(&library, "update_role", || {
            library.update_role(&role.role_id, translated("Arranger"), true)
        });
        assert_notifies(&library, "update_instrument", || {
            library.update_instrument(&instrument.instrument_id, translated("Fortepiano"), true)
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
            library.create_recording(work.clone(), Some(1963), Vec::new(), Vec::new(), true)
        });
        assert_notifies(&library, "update_recording", || {
            library.update_recording(
                &recording.recording_id,
                work.clone(),
                Some(1964),
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
                true,
            )
            .unwrap();
        let recording = library
            .create_recording(work.clone(), None, Vec::new(), Vec::new(), true)
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
