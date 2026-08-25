//! Building a library through `musicus-library`'s public API.
//!
//! Everything goes through the same mutators the app uses, so the result is
//! indistinguishable from a library the user built by hand.

use std::{collections::HashMap, fs, path::Path};

use anyhow::{Context, Result};
use musicus_library::{
    db::{
        models::{
            Composer, Ensemble, EnsemblePerformer, Instrument, Performer, Person, Role, Work,
        },
        TranslatedString,
    },
    library::Patterns,
    Library,
};
use rand::prelude::*;
use tempfile::TempDir;

use crate::names::Names;

/// How much of each kind of entity to generate.
#[derive(Clone, Copy, Debug)]
pub struct Counts {
    pub instruments: usize,
    pub roles: usize,
    pub persons: usize,
    pub ensembles: usize,
    pub works: usize,
    pub recordings: usize,
    pub tracks_per_recording: usize,
}

/// What was written, for the summary at the end of a run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub instruments: usize,
    pub roles: usize,
    pub persons: usize,
    pub ensembles: usize,
    pub works: usize,
    pub recordings: usize,
    pub tracks: usize,
}

/// Generate a library at `folder`, creating it if it does not exist.
///
/// `progress` is called with the number of recordings whose tracks have been
/// imported so far and the total, so a caller can show that the import — which
/// copies a file and opens a transaction per track — is making headway.
pub fn generate(
    folder: &Path,
    seed: u64,
    counts: Counts,
    mut progress: impl FnMut(usize, usize),
) -> Result<Summary> {
    fs::create_dir_all(folder)
        .with_context(|| format!("Failed to create the library folder {}", folder.display()))?;

    // The generator never downloads metadata, so the cache is thrown away with
    // the temporary directory.
    let cache_dir = TempDir::new().context("Failed to create a metadata cache directory")?;
    let library = Library::new(folder, cache_dir.path())
        .with_context(|| format!("Failed to open a library at {}", folder.display()))?;

    let staging = TempDir::new().context("Failed to create a staging directory")?;

    let mut names = Names::new(seed);
    let mut summary = Summary::default();

    let instruments = (0..counts.instruments)
        .map(|_| {
            let name = names.instrument();
            library.create_instrument(translated(&name), false)
        })
        .collect::<Result<Vec<Instrument>>>()
        .context("Failed to create instruments")?;
    summary.instruments = instruments.len();

    let roles = (0..counts.roles)
        .map(|_| {
            let name = names.role();
            library.create_role(translated(&name), false)
        })
        .collect::<Result<Vec<Role>>>()
        .context("Failed to create roles")?;
    summary.roles = roles.len();

    let persons = (0..counts.persons)
        .map(|_| {
            let name = names.person();
            library.create_person(translated(&name), false)
        })
        .collect::<Result<Vec<Person>>>()
        .context("Failed to create persons")?;
    summary.persons = persons.len();

    let mut ensembles: Vec<Ensemble> = Vec::with_capacity(counts.ensembles);
    for _ in 0..counts.ensembles {
        let name = names.ensemble();
        let members = pick_members(names.rng(), &persons, &instruments);

        ensembles.push(
            library
                .create_ensemble(translated(&name), members, false)
                .context("Failed to create an ensemble")?,
        );
    }
    summary.ensembles = ensembles.len();

    let mut works: Vec<Work> = Vec::with_capacity(counts.works);
    for _ in 0..counts.works {
        let name = names.work();
        let composer = match persons.choose(names.rng()) {
            Some(person) => vec![Composer {
                person: person.clone(),
                role: None,
            }],
            None => Vec::new(),
        };

        let work_instruments = sample(names.rng(), &instruments, 1, 3);

        works.push(
            library
                .create_work(
                    translated(&name),
                    Vec::new(),
                    composer,
                    work_instruments,
                    Vec::new(),
                    None,
                    false,
                )
                .context("Failed to create a work")?,
        );
    }
    summary.works = works.len();

    let mut recordings = Vec::with_capacity(counts.recordings);
    for _ in 0..counts.recordings {
        let Some(work) = works.choose(names.rng()).cloned() else {
            break;
        };

        let performers = sample(names.rng(), &persons, 0, 3)
            .into_iter()
            .map(|person| Performer {
                person,
                role: maybe(names.rng(), &roles, 0.3),
                instrument: maybe(names.rng(), &instruments, 0.7),
            })
            .collect::<Vec<Performer>>();

        let ensemble_performers = sample(names.rng(), &ensembles, 0, 2)
            .into_iter()
            .map(|ensemble| EnsemblePerformer {
                ensemble,
                role: maybe(names.rng(), &roles, 0.2),
            })
            .collect::<Vec<EnsemblePerformer>>();

        recordings.push(
            library
                .create_recording(
                    work,
                    performers,
                    ensemble_performers,
                    Vec::new(),
                    None,
                    false,
                )
                .context("Failed to create a recording")?,
        );
    }
    summary.recordings = recordings.len();

    for (index, recording) in recordings.iter().enumerate() {
        for track_index in 0..counts.tracks_per_recording {
            let source = stub_track(staging.path(), summary.tracks)?;

            library
                .import_track(
                    &source,
                    &recording.recording_id,
                    track_index as i32,
                    Vec::new(),
                    &Patterns::default(),
                )
                .context("Failed to import a track")?;

            // The library copies the file, so the staged one is not needed
            // anymore and would otherwise pile up for the whole run.
            let _ = fs::remove_file(&source);
            summary.tracks += 1;
        }

        progress(index + 1, recordings.len());
    }

    Ok(summary)
}

/// A `TranslatedString` carrying only the generic variant.
///
/// Every translated string needs that key: without it `TranslatedString::get`
/// logs a warning and returns an empty string.
fn translated(name: &str) -> TranslatedString {
    let mut translations = HashMap::new();
    translations.insert("generic".to_string(), name.to_string());
    TranslatedString(translations)
}

/// Write a placeholder file that can be imported as a track.
///
/// This is not audio. The library copies and renames it like any other file,
/// but writing tags into it fails, which the import only logs. The generated
/// library is therefore complete and browsable, but not playable.
fn stub_track(dir: &Path, number: usize) -> Result<std::path::PathBuf> {
    let path = dir.join(format!("{number}.mp3"));

    fs::write(&path, format!("placeholder audio {number}").as_bytes())
        .with_context(|| format!("Failed to write the staged file {}", path.display()))?;

    Ok(path)
}

/// Between `min` and `max` distinct items, cloned.
fn sample<T: Clone>(rng: &mut StdRng, items: &[T], min: usize, max: usize) -> Vec<T> {
    let max = max.min(items.len());

    if max < min {
        return Vec::new();
    }

    let amount = rng.random_range(min..=max);
    items.sample(rng, amount).cloned().collect()
}

/// One random item with probability `p`, nothing otherwise.
fn maybe<T: Clone>(rng: &mut StdRng, items: &[T], p: f64) -> Option<T> {
    if rng.random_bool(p) {
        items.choose(rng).cloned()
    } else {
        None
    }
}

/// Ensemble members, most of them with an instrument.
fn pick_members(
    rng: &mut StdRng,
    persons: &[Person],
    instruments: &[Instrument],
) -> Vec<Performer> {
    sample(rng, persons, 2, 8)
        .into_iter()
        .map(|person| Performer {
            person,
            role: None,
            instrument: maybe(rng, instruments, 0.8),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use musicus_library::library::LibraryQuery;

    use super::*;

    fn counts() -> Counts {
        Counts {
            instruments: 5,
            roles: 3,
            persons: 12,
            ensembles: 4,
            works: 8,
            recordings: 10,
            tracks_per_recording: 2,
        }
    }

    #[test]
    fn a_generated_library_is_populated_and_searchable() {
        let folder = TempDir::new().unwrap();
        let cache_dir = TempDir::new().unwrap();

        let summary = generate(folder.path(), 1, counts(), |_, _| {}).unwrap();

        assert_eq!(summary.persons, 12);
        assert_eq!(summary.recordings, 10);
        assert_eq!(summary.tracks, 20);

        let library = Library::new(folder.path(), cache_dir.path()).unwrap();
        assert!(!library.is_empty().unwrap());

        // Only persons that compose or perform something show up here, so this
        // asserts that the entities were actually wired together.
        let results = library.search(&LibraryQuery::default(), "").unwrap();
        assert!(!results.composers.is_empty());
        assert!(!results.works.is_empty());
        assert!(!results.ensembles.is_empty());

        // An unfiltered search deliberately does not list recordings; they are
        // reached through a work.
        let has_recordings = results.works.iter().any(|work| {
            let query = LibraryQuery {
                work: Some(work.clone()),
                ..Default::default()
            };

            !library.search(&query, "").unwrap().recordings.is_empty()
        });

        assert!(has_recordings);
    }

    #[test]
    fn every_track_has_a_file_in_the_library_folder() {
        let folder = TempDir::new().unwrap();

        let summary = generate(folder.path(), 2, counts(), |_, _| {}).unwrap();

        let files = fs::read_dir(folder.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name != "musicus.musdb")
            .count();

        assert_eq!(files, summary.tracks);
    }

    #[test]
    fn the_same_seed_produces_the_same_library() {
        let names = |seed| {
            let folder = TempDir::new().unwrap();
            let cache_dir = TempDir::new().unwrap();

            generate(folder.path(), seed, counts(), |_, _| {}).unwrap();

            let library = Library::new(folder.path(), cache_dir.path()).unwrap();
            let mut names = library
                .search_persons("")
                .unwrap()
                .into_iter()
                .map(|item| item.item.name.get().to_string())
                .collect::<Vec<String>>();

            names.sort();
            names
        };

        assert_eq!(names(3), names(3));
        assert_ne!(names(3), names(4));
    }
}
