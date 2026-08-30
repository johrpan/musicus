//! Ordered-association replacement.
//!
//! Associations with a meaningful order (work persons, instruments, tags,
//! relations, ensemble members, recording persons/ensembles/tags, album
//! recordings, track works) are all rewritten through the same mechanism:
//! delete the owner's previous rows, then insert the new rows in their given
//! order. Every function here follows that identical delete-then-insert shape
//! with `db::sequence_number`, so that all ordered associations share one
//! consistent write path — while each row stays an explicit, typed struct that
//! the compiler checks against its table.

use std::collections::HashSet;

use anyhow::{bail, Result};
use diesel::{prelude::*, SqliteConnection};

use crate::{
    db::{self, schema::*, tables},
    input::*,
};

/// Replace a work's credited-person rows in their given order.
pub(super) fn replace_work_persons(
    connection: &mut SqliteConnection,
    work_id: &str,
    persons: &[WorkPersonInput],
) -> Result<()> {
    diesel::delete(work_persons::table.filter(work_persons::work_id.eq(work_id)))
        .execute(connection)?;

    for (index, person) in persons.iter().enumerate() {
        diesel::insert_into(work_persons::table)
            .values(tables::WorkPerson {
                work_id: work_id.to_owned(),
                person_id: person.person_id.clone(),
                role_id: person.role_id.clone(),
                sequence_number: db::sequence_number(index)?,
            })
            .execute(connection)?;
    }

    Ok(())
}

/// Replace a work's instrument rows in their given order.
pub(super) fn replace_work_instruments(
    connection: &mut SqliteConnection,
    work_id: &str,
    instruments: &[String],
) -> Result<()> {
    diesel::delete(work_instruments::table.filter(work_instruments::work_id.eq(work_id)))
        .execute(connection)?;

    for (index, instrument_id) in instruments.iter().enumerate() {
        diesel::insert_into(work_instruments::table)
            .values(tables::WorkInstrument {
                work_id: work_id.to_owned(),
                instrument_id: instrument_id.clone(),
                sequence_number: db::sequence_number(index)?,
            })
            .execute(connection)?;
    }

    Ok(())
}

/// Replace a work's tag assignment rows in their given order.
///
/// Works and recordings share the same tag assignment semantics; this and
/// [`replace_recording_tags`] differ only in the table and row they write.
pub(super) fn replace_work_tags(
    connection: &mut SqliteConnection,
    work_id: &str,
    tags: &[TagInput],
) -> Result<()> {
    diesel::delete(work_tags::table.filter(work_tags::work_id.eq(work_id))).execute(connection)?;

    for (index, tag) in tags.iter().enumerate() {
        diesel::insert_into(work_tags::table)
            .values(tables::WorkTag {
                work_id: work_id.to_owned(),
                tag_id: tag.tag_id.clone(),
                value: tag.value.clone(),
                sequence_number: db::sequence_number(index)?,
            })
            .execute(connection)?;
    }

    Ok(())
}

/// Replace a recording's tag assignment rows in their given order.
///
/// Works and recordings share the same tag assignment semantics; this and
/// [`replace_work_tags`] differ only in the table and row they write.
pub(super) fn replace_recording_tags(
    connection: &mut SqliteConnection,
    recording_id: &str,
    tags: &[TagInput],
) -> Result<()> {
    diesel::delete(recording_tags::table.filter(recording_tags::recording_id.eq(recording_id)))
        .execute(connection)?;

    for (index, tag) in tags.iter().enumerate() {
        diesel::insert_into(recording_tags::table)
            .values(tables::RecordingTag {
                recording_id: recording_id.to_owned(),
                tag_id: tag.tag_id.clone(),
                value: tag.value.clone(),
                sequence_number: db::sequence_number(index)?,
            })
            .execute(connection)?;
    }

    Ok(())
}

/// Replace a work's outgoing relation rows, validating the new targets.
pub(super) fn replace_work_relations(
    connection: &mut SqliteConnection,
    work_id: &str,
    related_works: &[String],
) -> Result<()> {
    let mut seen = HashSet::new();

    for related_work_id in related_works {
        if related_work_id == work_id {
            bail!("Work {work_id} cannot relate to itself");
        }

        if !seen.insert(related_work_id.clone()) {
            bail!("Work {work_id} relates to {related_work_id} more than once");
        }
    }

    diesel::delete(work_relations::table.filter(work_relations::work_id.eq(work_id)))
        .execute(connection)?;

    for related_work_id in related_works {
        diesel::insert_into(work_relations::table)
            .values(tables::WorkRelation {
                work_id: work_id.to_owned(),
                related_work_id: related_work_id.clone(),
            })
            .execute(connection)?;
    }

    Ok(())
}

/// Replace an ensemble's member rows in their given order.
pub(super) fn replace_ensemble_persons(
    connection: &mut SqliteConnection,
    ensemble_id: &str,
    persons: &[RecordingPersonInput],
) -> Result<()> {
    diesel::delete(ensemble_persons::table.filter(ensemble_persons::ensemble_id.eq(ensemble_id)))
        .execute(connection)?;

    for (index, member) in persons.iter().enumerate() {
        diesel::insert_into(ensemble_persons::table)
            .values(tables::EnsemblePerson {
                ensemble_id: ensemble_id.to_owned(),
                person_id: member.person_id.clone(),
                role_id: member.role_id.clone(),
                instrument_id: member.instrument_id.clone(),
                sequence_number: db::sequence_number(index)?,
            })
            .execute(connection)?;
    }

    Ok(())
}

/// Replace a recording's performing-person rows in their given order.
pub(super) fn replace_recording_persons(
    connection: &mut SqliteConnection,
    recording_id: &str,
    persons: &[RecordingPersonInput],
) -> Result<()> {
    diesel::delete(
        recording_persons::table.filter(recording_persons::recording_id.eq(recording_id)),
    )
    .execute(connection)?;

    for (index, person) in persons.iter().enumerate() {
        diesel::insert_into(recording_persons::table)
            .values(tables::RecordingPerson {
                recording_id: recording_id.to_owned(),
                person_id: person.person_id.clone(),
                role_id: person.role_id.clone(),
                instrument_id: person.instrument_id.clone(),
                sequence_number: db::sequence_number(index)?,
            })
            .execute(connection)?;
    }

    Ok(())
}

/// Replace a recording's performing-ensemble rows in their given order.
pub(super) fn replace_recording_ensembles(
    connection: &mut SqliteConnection,
    recording_id: &str,
    ensembles: &[RecordingEnsembleInput],
) -> Result<()> {
    diesel::delete(
        recording_ensembles::table.filter(recording_ensembles::recording_id.eq(recording_id)),
    )
    .execute(connection)?;

    for (index, ensemble) in ensembles.iter().enumerate() {
        diesel::insert_into(recording_ensembles::table)
            .values(tables::RecordingEnsemble {
                recording_id: recording_id.to_owned(),
                ensemble_id: ensemble.ensemble_id.clone(),
                role_id: ensemble.role_id.clone(),
                sequence_number: db::sequence_number(index)?,
            })
            .execute(connection)?;
    }

    Ok(())
}

/// Replace an album's recording rows in their given order.
pub(super) fn replace_album_recordings(
    connection: &mut SqliteConnection,
    album_id: &str,
    recording_ids: &[String],
) -> Result<()> {
    diesel::delete(album_recordings::table.filter(album_recordings::album_id.eq(album_id)))
        .execute(connection)?;

    for (index, recording_id) in recording_ids.iter().enumerate() {
        diesel::insert_into(album_recordings::table)
            .values(tables::AlbumRecording {
                album_id: album_id.to_owned(),
                recording_id: recording_id.clone(),
                sequence_number: db::sequence_number(index)?,
            })
            .execute(connection)?;
    }

    Ok(())
}

/// Replace a track's associated-work rows in their given order.
pub(super) fn replace_track_works(
    connection: &mut SqliteConnection,
    track_id: &str,
    work_ids: &[String],
) -> Result<()> {
    diesel::delete(track_works::table.filter(track_works::track_id.eq(track_id)))
        .execute(connection)?;

    for (index, work_id) in work_ids.iter().enumerate() {
        diesel::insert_into(track_works::table)
            .values(tables::TrackWork {
                track_id: track_id.to_owned(),
                work_id: work_id.clone(),
                sequence_number: db::sequence_number(index)?,
            })
            .execute(connection)?;
    }

    Ok(())
}
