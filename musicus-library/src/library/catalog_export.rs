//! Reconstructing source definitions from an existing library.
//!
//! The inverse of `catalog_import`: reads every entity, recording and track a
//! library holds and reassembles them into the JSON-shaped
//! [`crate::catalog::Definitions`] a source file uses. Private tags — and any
//! tag association that refers to one — are left out, matching every other
//! export path (`exchange::export_library_zip`).

use std::path::PathBuf;

use crate::{
    catalog::{
        ComposerRef, Definitions, NamedEntity, PerformerRef, RecordingEntry, TagEntity, TagRef,
        TrackEntry, WorkEntity,
    },
    error::Result,
    library::Library,
    models,
};

/// Read every entity, recording and track `library` holds.
pub fn export(library: &Library) -> Result<Definitions> {
    let persons = library
        .list_persons()?
        .into_iter()
        .map(|person| NamedEntity {
            key: person.id,
            name: person.name,
        })
        .collect();
    let roles = library
        .list_roles()?
        .into_iter()
        .map(|role| NamedEntity {
            key: role.id,
            name: role.name,
        })
        .collect();
    let instruments = library
        .list_instruments()?
        .into_iter()
        .map(|instrument| NamedEntity {
            key: instrument.id,
            name: instrument.name,
        })
        .collect();
    let ensembles = library
        .list_ensembles()?
        .into_iter()
        .map(|ensemble| NamedEntity {
            key: ensemble.id,
            name: ensemble.name,
        })
        .collect();
    let tags = library
        .list_tags()?
        .into_iter()
        .filter_map(tag_entity)
        .collect();
    let works = library
        .list_works()?
        .into_iter()
        .map(|item| work_entity(&item.work))
        .collect();

    let mut recordings = Vec::new();
    for item in library.list_recordings()? {
        recordings.push(recording_entry(library, item.recording)?);
    }

    Ok(Definitions {
        persons,
        roles,
        instruments,
        ensembles,
        tags,
        works,
        recordings,
    })
}

fn tag_entity(tag: models::Tag) -> Option<TagEntity> {
    if tag.private {
        return None;
    }

    Some(TagEntity {
        key: tag.id,
        name: tag.name,
        takes_value: tag.takes_value,
    })
}

fn tag_ref(value: &models::TagValue) -> Option<TagRef> {
    if value.tag.private {
        return None;
    }

    Some(match &value.value {
        Some(text) => TagRef::Value {
            key: value.tag.id.clone(),
            value: text.clone(),
        },
        None => TagRef::Key(value.tag.id.clone()),
    })
}

fn composer_ref(work_person: &models::WorkPerson) -> ComposerRef {
    match &work_person.role {
        Some(role) => ComposerRef::Credited {
            key: work_person.person.id.clone(),
            role: Some(role.id.clone()),
        },
        None => ComposerRef::Person(work_person.person.id.clone()),
    }
}

fn work_entity(work: &models::Work) -> WorkEntity {
    WorkEntity {
        key: work.id.clone(),
        name: work.name.clone(),
        composers: work.work_persons.iter().map(composer_ref).collect(),
        tags: work.tags.iter().filter_map(tag_ref).collect(),
        instruments: work.instruments.iter().map(|i| i.id.clone()).collect(),
        parts: work.parts.iter().map(work_entity).collect(),
        // A work relates to at most one other in this format; a source
        // library with more than one relation on a work keeps only the
        // first.
        related_work: work.related_works.first().map(|related| related.id.clone()),
    }
}

fn recording_entry(library: &Library, recording: models::Recording) -> Result<RecordingEntry> {
    let mut performers: Vec<PerformerRef> = recording
        .recording_persons
        .iter()
        .map(|person| PerformerRef::Person {
            person: person.person.id.clone(),
            role: person.role.as_ref().map(|role| role.id.clone()),
            instrument: person.instrument.as_ref().map(|i| i.id.clone()),
        })
        .collect();

    performers.extend(recording.recording_ensembles.iter().map(|ensemble| {
        PerformerRef::Ensemble {
            ensemble: ensemble.ensemble.id.clone(),
            role: ensemble.role.as_ref().map(|role| role.id.clone()),
        }
    }));

    let tags = recording.tags.iter().filter_map(tag_ref).collect();

    let mut tracks = library.tracks_for_recording(&recording.id)?;
    tracks.sort_by_key(|track| track.recording_index);

    let tracks = tracks
        .into_iter()
        .map(|track| TrackEntry {
            file: PathBuf::from(track.path.as_str()),
            works: track.works.iter().map(|work| work.id.clone()).collect(),
        })
        .collect();

    Ok(RecordingEntry {
        work: recording.work.id,
        performers,
        tags,
        comment: recording.comment,
        tracks,
    })
}
