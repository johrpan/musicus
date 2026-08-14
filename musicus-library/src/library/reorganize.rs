use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use anyhow::{Context, Error, Result};
use diesel::{prelude::*, SqliteConnection};
use gettextrs::gettext;

use super::{filenames, Library};
use crate::{
    db::{
        self,
        models::Recording,
        schema::*,
        tables::{self},
    },
    format_translated,
    process::{spawn_process, Cancellation, ProcessHandle, ProcessMsg},
};

/// A track file that has to be renamed, with paths relative to the library
/// folder.
struct Rename {
    track_id: String,
    from: PathBuf,
    to: PathBuf,
    /// The name the file is parked under while the other files are moved.
    tmp: PathBuf,
}

impl Library {
    /// Rename every track file that does not match the filename pattern.
    ///
    /// Files that no track refers to are left untouched, and so are tracks
    /// whose file is missing; both are reported as warnings.
    pub fn reorganize_files(&self) -> Result<ProcessHandle> {
        let folder = PathBuf::from(self.folder());
        let pattern = self.filename_pattern();
        let connection = Arc::clone(&self.connection);

        // Reject an unusable pattern before starting an operation that could
        // only rename every file to its fallback name.
        filenames::validate(&pattern)?;

        Ok(spawn_process(move |sender, cancellation| {
            reorganize(&folder, &pattern, &connection, sender, cancellation)
        }))
    }
}

fn reorganize(
    folder: &Path,
    pattern: &str,
    connection: &Arc<Mutex<SqliteConnection>>,
    sender: &async_channel::Sender<ProcessMsg>,
    cancellation: &Cancellation,
) -> Result<()> {
    let connection = &mut *db::lock_connection(connection);

    let rows = tracks::table
        .order((tracks::recording_id, tracks::recording_index))
        .select(tables::Track::as_select())
        .load::<tables::Track>(connection)?;

    // Everything in the folder that is not a track file keeps its name, so no
    // track may be renamed onto it.
    let mut taken = unrelated_file_names(folder, &rows)?;

    let mut recordings: HashMap<String, Option<Recording>> = HashMap::new();
    let mut renames = Vec::new();
    let n_rows = rows.len();

    for (index, row) in rows.iter().enumerate() {
        cancellation.check()?;

        let from = row.path.0.clone();

        if !folder.join(&from).exists() {
            let _ = sender.send_blocking(ProcessMsg::Warning(format_translated!(
                gettext("The file of a track is missing: {}"),
                from.display()
            )));

            // The name stays reserved: the file may come back, and taking its
            // name for another track would then lose one of the two.
            taken.insert(name_key(&from));
            continue;
        }

        let recording = recordings
            .entry(row.recording_id.clone())
            .or_insert_with(|| match load_recording(&row.recording_id, connection) {
                Ok(recording) => Some(recording),
                Err(err) => {
                    log::warn!(
                        "Failed to load recording {} for naming: {err:?}",
                        row.recording_id
                    );
                    None
                }
            });

        let stem = recording
            .as_ref()
            .and_then(|recording| {
                let works = load_track_works(&row.track_id, connection).unwrap_or_default();

                filenames::render(
                    pattern,
                    &filenames::TrackNameData::new(recording, row.recording_index, &works),
                )
            })
            .unwrap_or_else(|| filenames::fallback_stem(&row.recording_id, row.recording_index));

        let extension = from
            .extension()
            .map(|extension| extension.to_string_lossy().into_owned());

        let to = PathBuf::from(unused_name(&stem, extension.as_deref(), &mut taken));

        if to != from {
            renames.push(Rename {
                track_id: row.track_id.clone(),
                from,
                to,
                tmp: PathBuf::from(format!(".musicus-reorganize-{}", row.track_id)),
            });
        }

        let _ = sender.send_blocking(ProcessMsg::Progress((index + 1) as f64 / n_rows as f64));
    }

    if renames.is_empty() {
        let _ = sender.send_blocking(ProcessMsg::Message(gettext(
            "All files already match the pattern.",
        )));

        return Ok(());
    }

    cancellation.check()?;

    // Park every file that moves under a name nothing else uses first. Two
    // tracks can trade names, and moving them one after the other would
    // overwrite one of the two files.
    let mut parked = 0;

    for rename in &renames {
        if let Err(err) = fs::rename(folder.join(&rename.from), folder.join(&rename.tmp)) {
            unpark(folder, &renames[..parked]);

            return Err(Error::from(err).context(format!(
                "Failed to move {} out of the way",
                rename.from.display()
            )));
        }

        parked += 1;
    }

    let n_renames = renames.len();
    let now = db::now();
    let mut moved = 0;

    let result = connection.transaction::<(), Error, _>(|connection| {
        for rename in &renames {
            diesel::update(tracks::table)
                .filter(tracks::track_id.eq(&rename.track_id))
                .set((
                    tracks::path.eq(tables::PathBufWrapper(rename.to.clone())),
                    tracks::edited_at.eq(now),
                ))
                .execute(connection)?;
        }

        // Only once the database is consistent are the files moved into place,
        // so that a failure here still rolls the whole operation back.
        for rename in &renames {
            fs::rename(folder.join(&rename.tmp), folder.join(&rename.to)).with_context(|| {
                format!("Failed to rename a track file to {}", rename.to.display())
            })?;

            moved += 1;

            let _ = sender.send_blocking(ProcessMsg::Progress(moved as f64 / n_renames as f64));
        }

        Ok(())
    });

    if let Err(err) = result {
        // The files that already reached their destination go back to their
        // parked name, from where everything returns to where it started.
        for rename in &renames[..moved] {
            if let Err(err) = fs::rename(folder.join(&rename.to), folder.join(&rename.tmp)) {
                log::error!("Failed to undo a rename: {err:?}");
            }
        }

        unpark(folder, &renames);

        return Err(err);
    }

    let _ = sender.send_blocking(ProcessMsg::Message(format_translated!(
        gettext("Renamed {} files."),
        n_renames
    )));

    Ok(())
}

/// Move parked files back to the name they had before.
fn unpark(folder: &Path, renames: &[Rename]) {
    for rename in renames {
        if let Err(err) = fs::rename(folder.join(&rename.tmp), folder.join(&rename.from)) {
            log::error!(
                "Failed to restore {} after a failed reorganization: {err:?}",
                rename.from.display()
            );
        }
    }
}

/// The names of the files in `folder` that none of `rows` refers to.
fn unrelated_file_names(folder: &Path, rows: &[tables::Track]) -> Result<HashSet<String>> {
    let track_names = rows
        .iter()
        .map(|row| name_key(&row.path.0))
        .collect::<HashSet<String>>();

    let mut names = HashSet::new();

    for entry in fs::read_dir(folder)? {
        let name = name_key(&PathBuf::from(entry?.file_name()));

        if !track_names.contains(&name) {
            names.insert(name);
        }
    }

    Ok(names)
}

/// How a file name is compared for collisions.
///
/// The library folder can live on a file system that does not distinguish upper
/// and lower case, where two names that only differ in case are the same file.
fn name_key(path: &Path) -> String {
    path.to_string_lossy().to_lowercase()
}

/// Build a file name from `stem` that nothing in `taken` uses yet, and reserve
/// it.
fn unused_name(stem: &str, extension: Option<&str>, taken: &mut HashSet<String>) -> String {
    let mut suffix = 0;

    loop {
        let mut name = stem.to_owned();

        if suffix > 0 {
            name.push_str(&format!("_{suffix}"));
        }

        if let Some(extension) = extension {
            name.push('.');
            name.push_str(extension);
        }

        if taken.insert(name.to_lowercase()) {
            return name;
        }

        suffix += 1;
    }
}

fn load_recording(recording_id: &str, connection: &mut SqliteConnection) -> Result<Recording> {
    let row = recordings::table
        .filter(recordings::recording_id.eq(recording_id))
        .select(tables::Recording::as_select())
        .first::<tables::Recording>(connection)?;

    Recording::from_table(row, connection)
}

fn load_track_works(
    track_id: &str,
    connection: &mut SqliteConnection,
) -> Result<Vec<crate::db::models::Work>> {
    works::table
        .inner_join(track_works::table)
        .order(track_works::sequence_number)
        .filter(track_works::track_id.eq(track_id))
        .select(tables::Work::as_select())
        .load::<tables::Work>(connection)?
        .into_iter()
        .map(|work| crate::db::models::Work::from_table(work, connection))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use tempfile::TempDir;

    use super::*;
    use crate::{
        db::{models::*, TranslatedString},
        library::TrackUpdate,
    };

    fn translated(name: &str) -> TranslatedString {
        let mut translations = HashMap::new();
        translations.insert("generic".to_string(), name.to_string());
        TranslatedString(translations)
    }

    fn library(dir: &TempDir, cache_dir: &TempDir) -> Library {
        Library::new(dir.path(), cache_dir.path()).unwrap()
    }

    /// Import `n_tracks` tracks of a work named `work_name`, one per movement.
    fn recording_with_tracks(
        library: &Library,
        source_dir: &TempDir,
        work_name: &str,
        movements: &[&str],
    ) -> Recording {
        let person = library
            .create_person(translated("Beethoven"), true)
            .unwrap();
        let work = library
            .create_work(
                translated(work_name),
                Vec::new(),
                vec![Composer { person, role: None }],
                Vec::new(),
                Vec::new(),
                true,
            )
            .unwrap();
        let recording = library
            .create_recording(work, Vec::new(), Vec::new(), Vec::new(), true)
            .unwrap();

        for (index, movement) in movements.iter().enumerate() {
            let part = library
                .create_work(
                    translated(movement),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    true,
                )
                .unwrap();

            let source = source_dir.path().join(format!("{movement}.mp3"));
            fs::write(&source, format!("audio of {movement}").as_bytes()).unwrap();

            library
                .import_track(&source, &recording.recording_id, index as i32, vec![part])
                .unwrap();
        }

        recording
    }

    /// Run a reorganization to completion.
    fn run(library: &Library) -> Vec<String> {
        let handle = library.reorganize_files().unwrap();
        let mut warnings = Vec::new();

        while let Ok(msg) = handle.receiver.recv_blocking() {
            match msg {
                ProcessMsg::Warning(warning) => warnings.push(warning),
                ProcessMsg::Result(result) => result.unwrap(),
                ProcessMsg::Cancelled => panic!("the reorganization was cancelled"),
                _ => (),
            }
        }

        warnings
    }

    /// The paths of all tracks of `recording`, in order.
    fn track_paths(library: &Library, recording: &Recording) -> Vec<PathBuf> {
        library
            .tracks_for_recording(&recording.recording_id)
            .unwrap()
            .into_iter()
            .map(|track| track.path)
            .collect()
    }

    #[test]
    fn renaming_a_work_is_picked_up_by_a_reorganization() {
        let dir = TempDir::new().unwrap();
        let cache_dir = TempDir::new().unwrap();
        let source_dir = TempDir::new().unwrap();
        let library = library(&dir, &cache_dir);

        let recording =
            recording_with_tracks(&library, &source_dir, "Symphonie No. 5", &["Allegro"]);

        library
            .update_work(
                &recording.work.work_id,
                translated("Symphony No. 5"),
                Vec::new(),
                recording.work.persons.clone(),
                Vec::new(),
                Vec::new(),
                true,
            )
            .unwrap();

        assert!(run(&library).is_empty());

        assert_eq!(
            track_paths(&library, &recording),
            vec![PathBuf::from("Beethoven_Symphony No. 5_01 Allegro.mp3")]
        );
        assert_eq!(
            fs::read(dir.path().join("Beethoven_Symphony No. 5_01 Allegro.mp3")).unwrap(),
            b"audio of Allegro"
        );
    }

    #[test]
    fn a_second_reorganization_changes_nothing() {
        let dir = TempDir::new().unwrap();
        let cache_dir = TempDir::new().unwrap();
        let source_dir = TempDir::new().unwrap();
        let library = library(&dir, &cache_dir);

        let recording = recording_with_tracks(
            &library,
            &source_dir,
            "Symphony No. 5",
            &["Allegro", "Andante"],
        );

        run(&library);
        let before = track_paths(&library, &recording);
        run(&library);

        assert_eq!(track_paths(&library, &recording), before);
    }

    /// Two tracks that trade names must both keep their audio.
    #[test]
    fn files_can_swap_names() {
        let dir = TempDir::new().unwrap();
        let cache_dir = TempDir::new().unwrap();
        let source_dir = TempDir::new().unwrap();
        let library = library(&dir, &cache_dir);

        let recording = recording_with_tracks(
            &library,
            &source_dir,
            "Symphony No. 5",
            &["Allegro", "Andante"],
        );

        // Reordering the tracks makes each of them want the file name of the
        // other one, because the index is part of the name.
        let tracks = library
            .tracks_for_recording(&recording.recording_id)
            .unwrap();
        library
            .set_recording_tracks(
                &recording.recording_id,
                vec![
                    TrackUpdate::Existing {
                        track_id: tracks[1].track_id.clone(),
                        works: tracks[1].works.clone(),
                    },
                    TrackUpdate::Existing {
                        track_id: tracks[0].track_id.clone(),
                        works: tracks[0].works.clone(),
                    },
                ],
                &[],
            )
            .unwrap();

        run(&library);

        let paths = track_paths(&library, &recording);
        assert_eq!(
            paths,
            vec![
                PathBuf::from("Beethoven_Symphony No. 5_01 Andante.mp3"),
                PathBuf::from("Beethoven_Symphony No. 5_02 Allegro.mp3"),
            ]
        );
        assert_eq!(
            fs::read(dir.path().join(&paths[0])).unwrap(),
            b"audio of Andante"
        );
        assert_eq!(
            fs::read(dir.path().join(&paths[1])).unwrap(),
            b"audio of Allegro"
        );
    }

    #[test]
    fn unrelated_files_are_left_alone() {
        let dir = TempDir::new().unwrap();
        let cache_dir = TempDir::new().unwrap();
        let source_dir = TempDir::new().unwrap();
        let library = library(&dir, &cache_dir);

        let recording = recording_with_tracks(&library, &source_dir, "Symphony No. 5", &["Notes"]);

        // A file that no track refers to and that the track would otherwise be
        // named after.
        library.set_filename_pattern("{movement}");
        fs::write(dir.path().join("Notes.mp3"), b"not a track").unwrap();

        run(&library);

        assert_eq!(
            fs::read(dir.path().join("Notes.mp3")).unwrap(),
            b"not a track"
        );
        assert_eq!(
            track_paths(&library, &recording),
            vec![PathBuf::from("Notes_1.mp3")]
        );
        assert!(dir.path().join("musicus.db").exists());
    }

    #[test]
    fn a_missing_file_is_reported_and_skipped() {
        let dir = TempDir::new().unwrap();
        let cache_dir = TempDir::new().unwrap();
        let source_dir = TempDir::new().unwrap();
        let library = library(&dir, &cache_dir);

        let recording =
            recording_with_tracks(&library, &source_dir, "Symphonie", &["Allegro", "Andante"]);
        let before = track_paths(&library, &recording);
        fs::remove_file(dir.path().join(&before[0])).unwrap();

        library
            .update_work(
                &recording.work.work_id,
                translated("Symphony"),
                Vec::new(),
                recording.work.persons.clone(),
                Vec::new(),
                Vec::new(),
                true,
            )
            .unwrap();

        let warnings = run(&library);

        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("Symphonie"), "{}", warnings[0]);

        let after = track_paths(&library, &recording);
        assert_eq!(after[0], before[0], "the missing file keeps its record");
        assert_eq!(after[1], PathBuf::from("Beethoven_Symphony_02 Andante.mp3"));
    }

    #[test]
    fn an_unusable_pattern_is_rejected_before_anything_moves() {
        let dir = TempDir::new().unwrap();
        let cache_dir = TempDir::new().unwrap();
        let source_dir = TempDir::new().unwrap();
        let library = library(&dir, &cache_dir);

        let recording = recording_with_tracks(&library, &source_dir, "Symphony", &["Allegro"]);
        let before = track_paths(&library, &recording);

        library.set_filename_pattern("{bogus}");
        assert!(library.reorganize_files().is_err());
        assert_eq!(track_paths(&library, &recording), before);
    }
}
