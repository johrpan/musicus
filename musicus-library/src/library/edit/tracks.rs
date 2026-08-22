//! Adding and removing tracks, including its files on disk.

use std::{
    ffi::{OsStr, OsString},
    fs::{self},
    path::{Path, PathBuf},
};

use anyhow::{bail, Error, Result};
use diesel::prelude::*;

use crate::db::{self, models::*, schema::*, tables};
use crate::library::Library;

impl Library {
    /// Delete a recording along with its tracks' files.
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

    /// Save the complete track list of `recording_id` in one transaction.
    ///
    /// `tracks` is the track list as it should be afterwards, in order; each
    /// track's `recording_index` is its position in that list, so a save can
    /// never leave duplicate or gapped indices behind. `deleted_tracks` are the
    /// tracks the caller removed from the recording.
    ///
    /// Either all of the deletions, updates and imports are applied or none of
    /// them are, so a failed save can simply be retried.
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

    pub fn delete_track(&self, track: &Track) -> Result<()> {
        // No recording is needed: without an import there is no file to name.
        self.apply_track_changes(None, Vec::new(), std::slice::from_ref(track))
    }

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
    pub tmp_path: PathBuf,
    pub to_path: PathBuf,
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

/// Build a library relative file name based on `stem` that no file in `folder`
/// and no file staged by the same batch is using yet.
///
/// The name only serves human orientation. Tracks keep the file they were
/// imported with even when they are renamed or renumbered later, so the obvious
/// name for a track can already belong to another one, and using it anyway
/// would overwrite that track's audio.
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
    use crate::db::TranslatedString;

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
                None,
                true,
            )
            .unwrap();
        let recording = library
            .create_recording(work.clone(), Vec::new(), Vec::new(), Vec::new(), None, true)
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
                None,
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
                None,
                true,
            )
        });

        let ensemble = assert_notifies(&library, "create_ensemble", || {
            library.create_ensemble(
                translated("Berliner Philharmoniker"),
                vec![Performer {
                    person: person.clone(),
                    role: None,
                    instrument: Some(instrument.clone()),
                }],
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
            library.create_recording(work.clone(), Vec::new(), Vec::new(), Vec::new(), None, true)
        });
        assert_notifies(&library, "update_recording", || {
            library.update_recording(
                &recording.recording_id,
                work.clone(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                None,
                true,
            )
        });

        assert_notifies(&library, "add_tag_to_works", || {
            library.add_tag_to_works(&[&work.work_id], &tag, None)
        });
        assert_notifies(&library, "add_tag_to_recordings", || {
            library.add_tag_to_recordings(&[&recording.recording_id], &tag, None)
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
                None,
                true,
            )
            .unwrap();
        let recording = library
            .create_recording(work.clone(), Vec::new(), Vec::new(), Vec::new(), None, true)
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

}
