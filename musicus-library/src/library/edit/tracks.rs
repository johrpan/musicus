//! Saving the track list of a recording, including its files on disk.

use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Error, Result};
use diesel::prelude::*;

use crate::db::tables::{LibraryTrackPath, TRACKS_SUBDIR};
use crate::{
    db::{self, schema::*, tables},
    error::{EntityKind, LibraryError},
    library::Library,
};

use super::assoc;

impl Library {
    /// Delete a recording along with its tracks' files.
    pub fn delete_recording_and_tracks(&self, recording_id: &str) -> Result<()> {
        // Delete from the library first: the track list is read inside the
        // transaction so that it cannot go stale between reading and deleting.
        // Only afterwards are the files removed, so a file-system error can
        // never leave a track row pointing at a deleted file.
        let folder = self.folder();

        let tracks = self.commit(|connection| {
            connection.transaction::<Vec<tables::Track>, Error, _>(|connection| {
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
            })
        })?;

        // The database no longer references these files. A failure to remove
        // one of them leaves an unreferenced file behind, which must not fail
        // the operation or prevent the remaining files from being removed.
        for track in tracks {
            let path = track.path.resolve(folder);
            if let Err(err) = fs::remove_file(&path) {
                log::warn!("Failed to remove track file {}: {err}", path.display());
            }
        }

        Ok(())
    }

    /// Save the complete track list of `recording_id` in one transaction.
    ///
    /// `tracks` is the entire desired state of the recording's track list, in
    /// order. The retained tracks are identified by ID and may only reference
    /// tracks that belong to this recording; every stored track not named in
    /// the list is deleted together with its file. Whether a track is updated,
    /// imported, moved or deleted is decided inside the transaction from the
    /// stored rows — a caller can neither delete through a forged model nor
    /// touch another recording's tracks.
    ///
    /// The recording keeps no gaps or duplicates: indices are assigned
    /// contiguously from the requested order. Either the whole save is applied
    /// or none of it is, so a failed save can simply be retried.
    pub fn set_recording_tracks(&self, recording_id: &str, tracks: Vec<TrackUpdate>) -> Result<()> {
        let mut seen = HashSet::new();
        for track in &tracks {
            if let TrackUpdate::Existing { track_id, .. } = track {
                if !seen.insert(track_id.clone()) {
                    bail!("Track {track_id} appears twice in the track list");
                }
            }
        }

        let folder = self.folder();

        // New tracks are copied into the managed subtree first. The subtree
        // must exist before anything is staged.
        if tracks
            .iter()
            .any(|track| matches!(track, TrackUpdate::New { .. }))
        {
            fs::create_dir_all(folder.join(TRACKS_SUBDIR))?;
        }

        // Copy the file of every new track next to its destination before
        // touching the database. Nothing is moved into place until the
        // transaction is about to commit, so a failure can leave neither a
        // track row pointing at a missing file nor an unreferenced file in the
        // library folder.
        let mut staged = Vec::new();
        let mut prepared = Vec::with_capacity(tracks.len());

        for (index, track) in tracks.into_iter().enumerate() {
            let recording_index = db::sequence_number(index)?;

            match track {
                TrackUpdate::Existing { track_id, works } => {
                    prepared.push(PreparedTrack::Existing {
                        track_id,
                        works,
                        recording_index,
                    })
                }
                TrackUpdate::New { path, works } => {
                    // TODO: Human interpretable filenames?
                    let library_path = unused_track_path(
                        folder,
                        recording_id,
                        recording_index,
                        path.extension().and_then(std::ffi::OsStr::to_str),
                        &staged,
                    )?;

                    let to_path = library_path.resolve(folder);
                    let mut tmp_path = to_path.clone();
                    tmp_path.as_mut_os_string().push(".part");

                    if let Err(err) = fs::copy(&path, &tmp_path) {
                        clean_up_staged(&staged, 0);
                        return Err(err.into());
                    }

                    staged.push(StagedFile { tmp_path, to_path });

                    prepared.push(PreparedTrack::New {
                        track_id: db::generate_id(),
                        library_path,
                        works,
                        recording_index,
                    });
                }
            }
        }

        let connection = &mut *self.conn();
        let now = db::now();
        let mut renamed = 0;

        let result = connection.transaction::<Vec<tables::Track>, Error, _>(|connection| {
            // A track cannot be saved through a recording that does not exist,
            // and no imported track may point at it.
            let recording_exists = recordings::table
                .filter(recordings::recording_id.eq(recording_id))
                .select(recordings::recording_id)
                .first::<String>(connection)
                .optional()?
                .is_some();

            if !recording_exists {
                return Err(
                    LibraryError::NotFound(EntityKind::Recording, recording_id.to_owned()).into(),
                );
            }

            // The desired final list derives from the rows stored right here;
            // whatever the caller names must be among them.
            let stored = tracks::table
                .filter(tracks::recording_id.eq(recording_id))
                .load::<tables::Track>(connection)?;

            for track in &prepared {
                if let PreparedTrack::Existing { track_id, .. } = track {
                    if !stored.iter().any(|row| row.track_id == *track_id) {
                        bail!("Track {track_id} does not exist or belongs to another recording");
                    }
                }
            }

            // Delete the tracks the requested list no longer contains. Their
            // paths come from the stored rows, never from caller data.
            let retained = prepared
                .iter()
                .filter_map(|track| match track {
                    PreparedTrack::Existing { track_id, .. } => Some(track_id.as_str()),
                    PreparedTrack::New { .. } => None,
                })
                .collect::<HashSet<&str>>();

            let deleted = stored
                .into_iter()
                .filter(|row| !retained.contains(row.track_id.as_str()))
                .collect::<Vec<tables::Track>>();

            for track in &deleted {
                diesel::delete(track_works::table)
                    .filter(track_works::track_id.eq(&track.track_id))
                    .execute(connection)?;

                diesel::delete(tracks::table)
                    .filter(tracks::track_id.eq(&track.track_id))
                    .execute(connection)?;
            }

            for track in &prepared {
                match track {
                    PreparedTrack::Existing {
                        track_id,
                        works,
                        recording_index,
                    } => {
                        diesel::update(tracks::table)
                            .filter(tracks::track_id.eq(track_id))
                            .set((
                                tracks::recording_index.eq(recording_index),
                                tracks::edited_at.eq(now),
                                tracks::last_used_at.eq(now),
                            ))
                            .execute(connection)?;

                        assoc::replace_track_works(connection, track_id, works)?;
                    }
                    PreparedTrack::New {
                        track_id,
                        library_path,
                        works,
                        recording_index,
                    } => {
                        let track_data = tables::Track {
                            track_id: track_id.clone(),
                            recording_id: recording_id.to_owned(),
                            recording_index: *recording_index,
                            path: library_path.clone(),
                            created_at: now,
                            edited_at: now,
                            last_used_at: now,
                        };

                        diesel::insert_into(tracks::table)
                            .values(&track_data)
                            .execute(connection)?;

                        assoc::replace_track_works(connection, track_id, works)?;
                    }
                }
            }

            // Moving the files into place is the last fallible step before the
            // commit, so that a failure here rolls the database back instead
            // of leaving track rows pointing at missing files. A destination
            // that appeared while the batch was being prepared is never
            // overwritten.
            for file in &staged {
                if file.to_path.exists() {
                    bail!(
                        "Track file {} appeared while the batch was being prepared",
                        file.to_path.display()
                    );
                }

                fs::rename(&file.tmp_path, &file.to_path)?;
                renamed += 1;
            }

            Ok(deleted)
        });

        match result {
            Ok(deleted) => {
                // The database no longer references the deleted tracks' files.
                // A failure to remove one of them leaves an unreferenced file
                // behind, which must not fail the operation or prevent the
                // remaining files from being removed.
                for track in deleted {
                    let path = track.path.resolve(folder);
                    if let Err(err) = fs::remove_file(&path) {
                        log::warn!("Failed to remove track file {}: {err}", path.display());
                    }
                }
            }
            Err(err) => {
                clean_up_staged(&staged, renamed);
                return Err(err);
            }
        }

        self.changed();

        Ok(())
    }
}

/// One track of the recording's desired final track list.
///
/// The requested list is the complete desired state: it names the tracks that
/// should remain (by ID) and the files that should be imported. Every stored
/// track that is not in the list is deleted together with its file.
#[derive(Clone, Debug)]
pub enum TrackUpdate {
    /// A track that is already in the library. Its works and its position
    /// within the recording are replaced; its file is left alone. The track
    /// must belong to the recording being saved.
    Existing {
        track_id: String,
        /// IDs of the works this track belongs to, in order.
        works: Vec<String>,
    },

    /// A file outside of the library that is to be imported as a new track.
    New {
        path: PathBuf,
        /// IDs of the works this track belongs to, in order.
        works: Vec<String>,
    },
}

/// A track of the requested list with everything resolved that has to be
/// decided before the transaction is started.
enum PreparedTrack {
    Existing {
        track_id: String,
        works: Vec<String>,
        recording_index: i32,
    },
    New {
        track_id: String,
        library_path: LibraryTrackPath,
        works: Vec<String>,
        recording_index: i32,
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
/// the batch failed, the remaining ones are still waiting next to it. Since a
/// destination is only ever moved into after checking that it does not exist,
/// removing it here can never delete a pre-existing file.
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

/// Build a library relative file name inside the managed [`TRACKS_SUBDIR`]
/// subtree that no file in `folder` and no file staged by the same batch is
/// using yet.
///
/// The name only serves human orientation. Tracks keep the file they were
/// imported with even when they are renamed or renumbered later, so the obvious
/// name for a track can already belong to another one, and using it anyway
/// would overwrite that track's audio.
fn unused_track_path(
    folder: &Path,
    recording_id: &str,
    recording_index: i32,
    extension: Option<&str>,
    staged: &[StagedFile],
) -> Result<LibraryTrackPath> {
    let mut suffix = 0;

    loop {
        let mut filename = format!("{recording_id}_{recording_index:02}");

        if suffix > 0 {
            filename.push_str(&format!("_{suffix}"));
        }

        if let Some(extension) = extension {
            filename.push('.');
            filename.push_str(extension);
        }

        let library_path = LibraryTrackPath::managed(&[TRACKS_SUBDIR, &filename])?;
        let to_path = library_path.resolve(folder);
        let mut tmp_path = to_path.clone();
        tmp_path.as_mut_os_string().push(".part");

        if !to_path.exists()
            && !tmp_path.exists()
            && !staged.iter().any(|file| file.to_path == to_path)
        {
            return Ok(library_path);
        }

        suffix += 1;
    }
}
