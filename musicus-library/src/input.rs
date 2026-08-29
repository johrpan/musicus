//! Input types for creating and updating library entities.
//!
//! Mutations take one of these command objects instead of loaded models.
//! Relationships use IDs; every input is validated completely before it is
//! applied.

use crate::db::TranslatedString;

/// A person, role or instrument: a name plus whether catalogue updates may
/// overwrite it.
#[derive(Clone, Debug)]
pub struct NamedEntityInput {
    pub name: TranslatedString,
    pub enable_updates: bool,
}

/// A tag assignment: which tag, and the value to assign if the tag takes one.
#[derive(Clone, Debug)]
pub struct TagInput {
    pub tag_id: String,
    pub value: Option<TranslatedString>,
}

/// A person credited on a work, with an optional role.
#[derive(Clone, Debug)]
pub struct WorkPersonInput {
    pub person_id: String,
    pub role_id: Option<String>,
}

/// A person performing in a recording or playing in an ensemble.
#[derive(Clone, Debug)]
pub struct RecordingPersonInput {
    pub person_id: String,
    pub role_id: Option<String>,
    pub instrument_id: Option<String>,
}

/// An ensemble performing in a recording, with an optional role.
#[derive(Clone, Debug)]
pub struct RecordingEnsembleInput {
    pub ensemble_id: String,
    pub role_id: Option<String>,
}

/// The complete desired work tree.
///
/// `id` is `Some` only for a child node that already exists and is being
/// edited within its parent. A create operation treats a child ID that does
/// not exist as a new child; a child ID that exists but belongs to a different
/// parent is rejected rather than silently moved.
#[derive(Clone, Debug)]
pub struct WorkInput {
    pub id: Option<String>,
    pub name: TranslatedString,
    pub parts: Vec<WorkInput>,
    pub work_persons: Vec<WorkPersonInput>,
    pub instrument_ids: Vec<String>,
    pub tags: Vec<TagInput>,
    pub related_work_ids: Vec<String>,
    pub enable_updates: bool,
}

/// The complete desired ensemble.
#[derive(Clone, Debug)]
pub struct EnsembleInput {
    pub name: TranslatedString,
    pub members: Vec<RecordingPersonInput>,
    pub enable_updates: bool,
}

/// The complete desired recording.
#[derive(Clone, Debug)]
pub struct RecordingInput {
    pub work_id: String,
    pub recording_persons: Vec<RecordingPersonInput>,
    pub recording_ensembles: Vec<RecordingEnsembleInput>,
    pub tags: Vec<TagInput>,
    pub comment: Option<String>,
    pub enable_updates: bool,
}

/// The complete desired album, referencing its recordings by ID.
#[derive(Clone, Debug)]
pub struct AlbumInput {
    pub name: TranslatedString,
    pub recording_ids: Vec<String>,
    pub enable_updates: bool,
}
