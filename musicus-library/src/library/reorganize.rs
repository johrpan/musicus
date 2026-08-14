use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use anyhow::{Context, Error, Result};
use diesel::{prelude::*, SqliteConnection};
use gettextrs::gettext;

use super::{
    audio_tags::{self, AudioTags},
    filenames, pattern, Library, Patterns,
};
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

/// What a reorganization has to do to the file of one track.
struct TrackTask {
    /// The name the file ends up under, relative to the library folder.
    path: PathBuf,
    /// `None` if the file is already named after the pattern.
    rename: Option<Rename>,
    /// `None` if the metadata the tags are built from could not be loaded.
    tags: Option<AudioTags>,
}

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
    /// Bring every track file in line with the configured patterns.
    ///
    /// Files that no track refers to are left untouched, and so are tracks
    /// whose file is missing; both are reported as warnings.
    ///
    /// Renaming and tagging are deliberately not equally safe. The renames are
    /// applied together with the database update, so they either all happen or
    /// none of them do. Writing a tag overwrites the user's file and cannot be
    /// undone, so it happens afterwards, per file and best effort: everything
    /// the tags contain is derived from the database, which makes the tagging
    /// pass idempotent and repeatable, and a file it could not write is reported
    /// rather than rolled back.
    pub fn reorganize_files(&self) -> Result<ProcessHandle> {
        let folder = PathBuf::from(self.folder());
        let patterns = self.patterns();
        let connection = Arc::clone(&self.connection);

        // Reject an unusable pattern before starting an operation that could
        // only rename every file to its fallback name and strip every tag.
        filenames::validate(&patterns.filename)?;
        audio_tags::validate(&patterns.album)?;
        audio_tags::validate(&patterns.artist)?;
        audio_tags::validate(&patterns.title)?;

        Ok(spawn_process(move |sender, cancellation| {
            reorganize(&folder, &patterns, &connection, sender, cancellation)
        }))
    }
}

fn reorganize(
    folder: &Path,
    patterns: &Patterns,
    connection: &Arc<Mutex<SqliteConnection>>,
    sender: &async_channel::Sender<ProcessMsg>,
    cancellation: &Cancellation,
) -> Result<()> {
    // The database is only needed to plan and to record the renames. It is
    // unlocked again before the files are tagged, which is by far the longest
    // part and does not touch it.
    let (tasks, n_renames) = rename_files(folder, patterns, connection, sender, cancellation)?;

    let n_tagged = tag_files(folder, &tasks, n_renames, sender, cancellation)?;

    let _ = sender.send_blocking(ProcessMsg::Message(format_translated!(
        gettext("Renamed {} files and updated the tags of {} files."),
        n_renames,
        n_tagged
    )));

    Ok(())
}

/// Plan the work for every track and apply the renames among it.
///
/// Returns the planned tasks, whose paths are the ones the files have now, and
/// the number of files that were renamed.
fn rename_files(
    folder: &Path,
    patterns: &Patterns,
    connection: &Arc<Mutex<SqliteConnection>>,
    sender: &async_channel::Sender<ProcessMsg>,
    cancellation: &Cancellation,
) -> Result<(Vec<TrackTask>, usize)> {
    let connection = &mut *db::lock_connection(connection);

    let rows = tracks::table
        .order((tracks::recording_id, tracks::recording_index))
        .select(tables::Track::as_select())
        .load::<tables::Track>(connection)?;

    // Everything in the folder that is not a track file keeps its name, so no
    // track may be renamed onto it.
    let mut taken = unrelated_file_names(folder, &rows)?;

    let mut recordings: HashMap<String, Option<Recording>> = HashMap::new();
    let mut tasks = Vec::with_capacity(rows.len());
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

        // The same values describe the name and the tags of a track, so they are
        // collected once.
        let data = recording.as_ref().map(|recording| {
            let works = load_track_works(&row.track_id, connection).unwrap_or_default();
            pattern::TrackData::new(recording, row.recording_index, &works)
        });

        let stem = data
            .as_ref()
            .and_then(|data| filenames::render(&patterns.filename, data))
            .unwrap_or_else(|| filenames::fallback_stem(&row.recording_id, row.recording_index));

        let tags = data
            .as_ref()
            .map(|data| AudioTags::render(patterns, data, row.recording_index));

        let extension = from
            .extension()
            .map(|extension| extension.to_string_lossy().into_owned());

        let to = PathBuf::from(unused_name(&stem, extension.as_deref(), &mut taken));

        let rename = (to != from).then(|| Rename {
            track_id: row.track_id.clone(),
            from,
            to: to.clone(),
            tmp: PathBuf::from(format!(".musicus-reorganize-{}", row.track_id)),
        });

        tasks.push(TrackTask {
            path: to,
            rename,
            tags,
        });

        // Planning is the first half of the work; renaming and tagging share the
        // second one, because their number is only known once this is done.
        let _ = sender.send_blocking(ProcessMsg::Progress(
            0.5 * (index + 1) as f64 / n_rows as f64,
        ));
    }

    let renames = tasks
        .iter()
        .filter_map(|task| task.rename.as_ref())
        .collect::<Vec<&Rename>>();

    if renames.is_empty() {
        return Ok((tasks, 0));
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
    let n_remaining = n_renames + tasks.iter().filter(|task| task.tags.is_some()).count();
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

            let _ = sender.send_blocking(ProcessMsg::Progress(
                0.5 + 0.5 * moved as f64 / n_remaining as f64,
            ));
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

    Ok((tasks, n_renames))
}

/// Write the tags of every task whose metadata could be loaded.
///
/// Returns the number of files that were actually rewritten. A file whose tags
/// cannot be written is reported and skipped: the database already describes
/// the library correctly, and another reorganization can try again.
fn tag_files(
    folder: &Path,
    tasks: &[TrackTask],
    n_renames: usize,
    sender: &async_channel::Sender<ProcessMsg>,
    cancellation: &Cancellation,
) -> Result<usize> {
    let n_remaining = (n_renames + tasks.iter().filter(|task| task.tags.is_some()).count()).max(1);
    let mut done = n_renames;
    let mut n_tagged = 0;

    for task in tasks {
        let Some(tags) = &task.tags else {
            continue;
        };

        cancellation.check()?;

        let path = folder.join(&task.path);

        match audio_tags::write(&path, tags) {
            Ok(true) => n_tagged += 1,
            Ok(false) => (),
            Err(err) => {
                log::warn!("Failed to tag {}: {err:?}", path.display());

                let _ = sender.send_blocking(ProcessMsg::Warning(format_translated!(
                    gettext("The tags of a file could not be written: {}"),
                    task.path.display()
                )));
            }
        }

        done += 1;

        let _ = sender.send_blocking(ProcessMsg::Progress(
            0.5 + 0.5 * done as f64 / n_remaining as f64,
        ));
    }

    Ok(n_tagged)
}

/// Move parked files back to the name they had before.
fn unpark(folder: &Path, renames: &[&Rename]) {
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

    use lofty::{file::TaggedFileExt, probe::read_from_path, tag::Accessor};
    use tempfile::TempDir;

    use super::*;
    use crate::{
        db::{models::*, TranslatedString},
        library::{audio_tags::minimal_wav, TrackUpdate},
    };

    fn translated(name: &str) -> TranslatedString {
        let mut translations = HashMap::new();
        translations.insert("generic".to_string(), name.to_string());
        TranslatedString(translations)
    }

    fn library(dir: &TempDir, cache_dir: &TempDir) -> Library {
        Library::new(dir.path(), cache_dir.path()).unwrap()
    }

    /// Import one track per movement of a work named `work_name`.
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

            // A real container, because the tags are written into it.
            let source = source_dir.path().join(format!("{movement}.wav"));
            fs::write(&source, minimal_wav()).unwrap();

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

    /// The album, title and track number written into the file at `path`.
    fn tags_of(path: &Path) -> (Option<String>, Option<String>, Option<u32>) {
        let file = read_from_path(path).unwrap();
        let tag = file.primary_tag().unwrap();

        (
            tag.album().map(|album| album.into_owned()),
            tag.title().map(|title| title.into_owned()),
            tag.track(),
        )
    }

    fn modified(path: &Path) -> std::time::SystemTime {
        fs::metadata(path).unwrap().modified().unwrap()
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
            vec![PathBuf::from("Beethoven; Symphony No. 5; 01 Allegro.wav")]
        );

        // The tags follow the new name.
        let (album, title, track) =
            tags_of(&dir.path().join("Beethoven; Symphony No. 5; 01 Allegro.wav"));

        assert_eq!(album.as_deref(), Some("Beethoven"));
        assert_eq!(title.as_deref(), Some("Symphony No. 5: Allegro"));
        assert_eq!(track, Some(1));
    }

    /// Retagging is not renaming: a file that is already named correctly still
    /// has to follow a change of the tag patterns.
    #[test]
    fn a_file_that_is_not_renamed_is_still_tagged() {
        let dir = TempDir::new().unwrap();
        let cache_dir = TempDir::new().unwrap();
        let source_dir = TempDir::new().unwrap();
        let library = library(&dir, &cache_dir);

        let recording = recording_with_tracks(&library, &source_dir, "Symphony", &["Allegro"]);
        let before = track_paths(&library, &recording);

        library.set_patterns(&Patterns {
            album: "{composer} - {work}".to_owned(),
            ..Patterns::default()
        });

        assert!(run(&library).is_empty());

        assert_eq!(track_paths(&library, &recording), before, "no rename");

        let (album, ..) = tags_of(&dir.path().join(&before[0]));
        assert_eq!(album.as_deref(), Some("Beethoven - Symphony"));
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
        let written = before
            .iter()
            .map(|path| modified(&dir.path().join(path)))
            .collect::<Vec<_>>();

        run(&library);

        assert_eq!(track_paths(&library, &recording), before);

        // A file that already carries the right tags may not be rewritten.
        for (path, modified_before) in before.iter().zip(written) {
            assert_eq!(modified(&dir.path().join(path)), modified_before);
        }
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
                PathBuf::from("Beethoven; Symphony No. 5; 01 Andante.wav"),
                PathBuf::from("Beethoven; Symphony No. 5; 02 Allegro.wav"),
            ]
        );

        // The tags identify which of the two files ended up where.
        let (_, title, track) = tags_of(&dir.path().join(&paths[0]));
        assert_eq!(title.as_deref(), Some("Symphony No. 5: Andante"));
        assert_eq!(track, Some(1));

        let (_, title, track) = tags_of(&dir.path().join(&paths[1]));
        assert_eq!(title.as_deref(), Some("Symphony No. 5: Allegro"));
        assert_eq!(track, Some(2));
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
        library.set_filename_pattern("{part}");
        fs::write(dir.path().join("Notes.wav"), b"not a track").unwrap();

        run(&library);

        assert_eq!(
            fs::read(dir.path().join("Notes.wav")).unwrap(),
            b"not a track"
        );
        assert_eq!(
            track_paths(&library, &recording),
            vec![PathBuf::from("Notes_1.wav")]
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
        assert_eq!(
            after[1],
            PathBuf::from("Beethoven; Symphony; 02 Andante.wav")
        );
    }

    /// A file whose tags cannot be written may neither fail the reorganization
    /// nor keep it from renaming that same file.
    #[test]
    fn a_file_that_cannot_be_tagged_is_reported_and_still_renamed() {
        let dir = TempDir::new().unwrap();
        let cache_dir = TempDir::new().unwrap();
        let source_dir = TempDir::new().unwrap();
        let library = library(&dir, &cache_dir);

        let recording =
            recording_with_tracks(&library, &source_dir, "Symphonie No. 5", &["Allegro"]);

        // Replace the audio with something lofty cannot make sense of.
        let before = track_paths(&library, &recording);
        fs::write(dir.path().join(&before[0]), b"not audio at all").unwrap();

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

        let warnings = run(&library);

        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("Allegro"), "{}", warnings[0]);

        let after = track_paths(&library, &recording);
        assert_eq!(
            after,
            vec![PathBuf::from("Beethoven; Symphony No. 5; 01 Allegro.wav")],
            "the rename still happened"
        );
        assert_eq!(
            fs::read(dir.path().join(&after[0])).unwrap(),
            b"not audio at all"
        );
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

        // A tag pattern is rejected just as early, so that a typo cannot strip
        // the tags of the whole library.
        library.set_patterns(&Patterns {
            album: "{bogus}".to_owned(),
            ..Patterns::default()
        });
        assert!(library.reorganize_files().is_err());
        assert_eq!(track_paths(&library, &recording), before);
    }
}
