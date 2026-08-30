//! Bulk-importing recordings and tracks described in JSON.

use std::{
    collections::HashSet,
    ops::ControlFlow,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::{
    db::TranslatedString,
    error::Result,
    input::{RecordingEnsembleInput, RecordingInput, RecordingPersonInput, TagInput},
    library::{
        edit::TrackUpdate,
        exchange::{self, ExchangeEvent},
        Library,
    },
};

/// A person or ensemble performing on a recording, referenced by catalogue
/// key.
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum PerformerRef {
    Person {
        person: String,
        role: Option<String>,
        instrument: Option<String>,
    },
    Ensemble {
        ensemble: String,
        role: Option<String>,
    },
}

/// A tag association on a recording: a plain tag key, or a tag key with a
/// value for tags that take one.
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum TagRef {
    Key(String),
    Value {
        key: String,
        value: TranslatedString,
    },
}

/// One track: an audio file on disk, plus the works (by catalogue key) it
/// belongs to, in order.
#[derive(Clone, Debug, Deserialize)]
pub struct TrackEntry {
    pub file: PathBuf,
    #[serde(default)]
    pub works: Vec<String>,
}

/// One recording: a catalogue work, its performers and tags, and its tracks.
#[derive(Clone, Debug, Deserialize)]
pub struct RecordingEntry {
    pub work: String,
    #[serde(default)]
    pub performers: Vec<PerformerRef>,
    #[serde(default)]
    pub tags: Vec<TagRef>,
    #[serde(default)]
    pub comment: Option<String>,
    #[serde(default)]
    pub tracks: Vec<TrackEntry>,
}

/// The contents of a recordings JSON file.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct RecordingsFile {
    #[serde(default)]
    pub recordings: Vec<RecordingEntry>,
}

/// What was written, for the summary at the end of a run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub recordings: usize,
    pub tracks: usize,
}

/// Import every recording of `file` into `library`.
///
/// `metadata_path` is installed as `library`'s metadata catalogue first, so
/// that every catalogue key `file` references can be resolved and imported
/// with the ID the catalogue uses. `tracks_dir` is the base every track's
/// relative `file` is resolved against.
pub fn import(
    library: &Library,
    metadata_path: &Path,
    tracks_dir: &Path,
    file: &RecordingsFile,
    observer: &mut dyn FnMut(ExchangeEvent) -> ControlFlow<()>,
) -> Result<Summary> {
    exchange::install_metadata(&library.exchange_context(), metadata_path, observer)?;

    let mut resolved = ResolvedKeys::default();
    let mut summary = Summary::default();

    for entry in &file.recordings {
        resolved.work(library, &entry.work)?;

        let mut recording_persons = Vec::new();
        let mut recording_ensembles = Vec::new();

        for performer in &entry.performers {
            match performer {
                PerformerRef::Person {
                    person,
                    role,
                    instrument,
                } => {
                    resolved.person(library, person)?;
                    resolved.role(library, role.as_deref())?;
                    resolved.instrument(library, instrument.as_deref())?;

                    recording_persons.push(RecordingPersonInput {
                        person_id: person.clone(),
                        role_id: role.clone(),
                        instrument_id: instrument.clone(),
                    });
                }
                PerformerRef::Ensemble { ensemble, role } => {
                    resolved.ensemble(library, ensemble)?;
                    resolved.role(library, role.as_deref())?;

                    recording_ensembles.push(RecordingEnsembleInput {
                        ensemble_id: ensemble.clone(),
                        role_id: role.clone(),
                    });
                }
            }
        }

        let mut tags = Vec::with_capacity(entry.tags.len());
        for tag in &entry.tags {
            let (tag_id, value) = match tag {
                TagRef::Key(tag_id) => (tag_id.clone(), None),
                TagRef::Value { key, value } => (key.clone(), Some(value.clone())),
            };

            resolved.tag(library, &tag_id)?;
            tags.push(TagInput { tag_id, value });
        }

        let recording = library.create_recording(RecordingInput {
            work_id: entry.work.clone(),
            recording_persons,
            recording_ensembles,
            tags,
            comment: entry.comment.clone(),
            enable_updates: false,
        })?;
        summary.recordings += 1;

        let track_updates = entry
            .tracks
            .iter()
            .map(|track| TrackUpdate::New {
                path: tracks_dir.join(&track.file),
                works: track.works.clone(),
            })
            .collect::<Vec<_>>();
        summary.tracks += track_updates.len();

        library.set_recording_tracks(&recording.id, track_updates)?;
    }

    Ok(summary)
}

/// Which catalogue keys have already been imported into the library, so that
/// a key shared by many recordings (a favourite conductor, a recurring
/// ensemble) is only imported once.
#[derive(Default)]
struct ResolvedKeys {
    works: HashSet<String>,
    persons: HashSet<String>,
    roles: HashSet<String>,
    instruments: HashSet<String>,
    ensembles: HashSet<String>,
    tags: HashSet<String>,
}

impl ResolvedKeys {
    fn work(&mut self, library: &Library, id: &str) -> Result<()> {
        if self.works.insert(id.to_owned()) {
            library.import_metadata_work(id)?;
        }
        Ok(())
    }

    fn person(&mut self, library: &Library, id: &str) -> Result<()> {
        if self.persons.insert(id.to_owned()) {
            library.import_metadata_person(id)?;
        }
        Ok(())
    }

    fn role(&mut self, library: &Library, id: Option<&str>) -> Result<()> {
        if let Some(id) = id {
            if self.roles.insert(id.to_owned()) {
                library.import_metadata_role(id)?;
            }
        }
        Ok(())
    }

    fn instrument(&mut self, library: &Library, id: Option<&str>) -> Result<()> {
        if let Some(id) = id {
            if self.instruments.insert(id.to_owned()) {
                library.import_metadata_instrument(id)?;
            }
        }
        Ok(())
    }

    fn ensemble(&mut self, library: &Library, id: &str) -> Result<()> {
        if self.ensembles.insert(id.to_owned()) {
            library.import_metadata_ensemble(id)?;
        }
        Ok(())
    }

    fn tag(&mut self, library: &Library, id: &str) -> Result<()> {
        if self.tags.insert(id.to_owned()) {
            library.import_metadata_tag(id)?;
        }
        Ok(())
    }
}
