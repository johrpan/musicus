use std::{
    ffi::OsString,
    fs::{self},
    path::{Path, PathBuf},
};

use adw::subclass::prelude::*;
use anyhow::{Error, Result};
use chrono::prelude::*;
use diesel::{prelude::*, QueryDsl, SqliteConnection};

use super::Library;
use crate::db::{self, models::*, schema::*, tables, TranslatedString};

impl Library {
    pub fn create_person(&self, name: TranslatedString, enable_updates: bool) -> Result<Person> {
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        let now = Local::now().naive_local();

        let person = Person {
            person_id: db::generate_id(),
            name,
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
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        let now = Local::now().naive_local();

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
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

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
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        let now = Local::now().naive_local();

        let instrument = Instrument {
            instrument_id: db::generate_id(),
            name,
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
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        let now = Local::now().naive_local();

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
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        diesel::delete(instruments::table)
            .filter(instruments::instrument_id.eq(instrument_id))
            .execute(connection)?;

        self.changed();

        Ok(())
    }

    pub fn create_role(&self, name: TranslatedString, enable_updates: bool) -> Result<Role> {
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        let now = Local::now().naive_local();

        let role = Role {
            role_id: db::generate_id(),
            name,
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
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        let now = Local::now().naive_local();

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
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

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
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        let work = self.create_work_priv(
            connection,
            name,
            parts,
            persons,
            instruments,
            None,
            None,
            enable_updates,
        )?;

        self.changed();

        Ok(work)
    }

    fn create_work_priv(
        &self,
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
        let now = Local::now().naive_local();

        let work_data = tables::Work {
            work_id: work_id.clone(),
            parent_work_id: parent_work_id.map(|w| w.to_string()),
            sequence_number: sequence_number,
            name,
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
            self.create_work_priv(
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
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        self.update_work_priv(
            connection,
            work_id,
            name,
            parts,
            persons,
            instruments,
            None,
            None,
            enable_updates,
        )?;

        self.changed();

        Ok(())
    }

    fn update_work_priv(
        &self,
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
        let now = Local::now().naive_local();

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
                self.update_work_priv(
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
                self.create_work_priv(
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
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        diesel::delete(works::table)
            .filter(works::work_id.eq(work_id))
            .execute(connection)?;

        self.changed();

        Ok(())
    }

    pub fn create_ensemble(
        &self,
        name: TranslatedString,
        enable_updates: bool,
    ) -> Result<Ensemble> {
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        let now = Local::now().naive_local();

        let ensemble_data = tables::Ensemble {
            ensemble_id: db::generate_id(),
            name,
            created_at: now,
            edited_at: now,
            last_used_at: now,
            last_played_at: None,
            enable_updates,
        };

        // TODO: Add persons.

        diesel::insert_into(ensembles::table)
            .values(&ensemble_data)
            .execute(connection)?;

        let ensemble = Ensemble::from_table(ensemble_data, connection)?;

        self.changed();

        Ok(ensemble)
    }

    pub fn update_ensemble(
        &self,
        id: &str,
        name: TranslatedString,
        enable_updates: bool,
    ) -> Result<()> {
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        let now = Local::now().naive_local();

        diesel::update(ensembles::table)
            .filter(ensembles::ensemble_id.eq(id))
            .set((
                ensembles::name.eq(name),
                ensembles::edited_at.eq(now),
                ensembles::last_used_at.eq(now),
                ensembles::enable_updates.eq(enable_updates),
            ))
            .execute(connection)?;

        // TODO: Support updating persons.

        self.changed();

        Ok(())
    }

    pub fn delete_ensemble(&self, ensemble_id: &str) -> Result<()> {
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

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
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        let recording_id = db::generate_id();
        let now = Local::now().naive_local();

        let recording_data = tables::Recording {
            recording_id: recording_id.clone(),
            work_id: work.work_id.clone(),
            year,
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

        let recording = Recording::from_table(recording_data, connection)?;

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
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        let now = Local::now().naive_local();

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

        self.changed();

        Ok(())
    }

    pub fn delete_recording(&self, recording_id: &str) -> Result<()> {
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        diesel::delete(recordings::table)
            .filter(recordings::recording_id.eq(recording_id))
            .execute(connection)?;

        self.changed();

        Ok(())
    }

    pub fn delete_recording_and_tracks(&self, recording_id: &str) -> Result<()> {
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        let tracks = tracks::table
            .filter(tracks::recording_id.eq(recording_id))
            .load::<tables::Track>(connection)?;

        // Delete from library first to avoid orphan tracks in case of file
        // system related errors.

        connection.transaction::<(), Error, _>(|connection| {
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

            Ok(())
        })?;

        let library_path = PathBuf::from(self.folder());
        for track in tracks {
            fs::remove_file(library_path.join(&track.path))?;
        }

        self.changed();

        Ok(())
    }

    pub fn create_album(
        &self,
        name: TranslatedString,
        recordings: Vec<Recording>,
    ) -> Result<Album> {
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        let album_id = db::generate_id();
        let now = Local::now().naive_local();

        let album_data = tables::Album {
            album_id: album_id.clone(),
            name,
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

        let album = Album::from_table(album_data, connection)?;

        self.changed();

        Ok(album)
    }

    pub fn update_album(
        &self,
        album_id: &str,
        name: TranslatedString,
        recordings: Vec<Recording>,
    ) -> Result<()> {
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        let now = Local::now().naive_local();

        diesel::update(albums::table)
            .filter(albums::album_id.eq(album_id))
            .set((
                albums::name.eq(name),
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

        self.changed();

        Ok(())
    }

    pub fn delete_album(&self, album_id: &str) -> Result<()> {
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

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
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        let track_id = db::generate_id();
        let now = Local::now().naive_local();

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

        fs::copy(path, to_path)?;

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

        Ok(())
    }

    // TODO: Support mediums, think about albums.
    pub fn delete_track(&self, track: &Track) -> Result<()> {
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        diesel::delete(track_works::table)
            .filter(track_works::track_id.eq(&track.track_id))
            .execute(connection)?;

        diesel::delete(tracks::table)
            .filter(tracks::track_id.eq(&track.track_id))
            .execute(connection)?;

        let mut path = PathBuf::from(self.folder());
        path.push(&track.path);
        fs::remove_file(path)?;

        Ok(())
    }

    // TODO: Support mediums, think about albums.
    pub fn update_track(
        &self,
        track_id: &str,
        recording_index: i32,
        works: Vec<Work>,
    ) -> Result<()> {
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        let now = Local::now().naive_local();

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
    }
}
