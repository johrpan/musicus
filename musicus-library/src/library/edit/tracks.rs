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
