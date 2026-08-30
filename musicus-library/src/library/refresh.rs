//! Cathalogue-driven metadata refresh.
//!
//! A refresh replaces catalogue-owned fields and associations of every local
//! entity whose `enable_updates` is on, imports newly referenced dependencies,
//! and leaves everything else exactly as it is. The plan is built from the
//! validated catalogue graph before the write transaction opens, so a refresh
//! is all-or-nothing and never leaves a half-replaced aggregate behind.

use std::collections::HashSet;

use diesel::{prelude::*, SqliteConnection};

use crate::{
    db::{self, schema::*, tables, tables::Source},
    error::{EntityKind, LibraryError, Result},
    library::import_graph::{DestinationState, ImportClosure, ImportGraph},
};

/// The entities a refresh considers, chosen before any row is written.
///
/// The seeds are the catalogue entities that already exist locally with
/// `enable_updates = true`; their closure is everything the refreshed records
/// need (ancestors, descendants, relations, people, roles, instruments,
/// ensembles and tags). Entities in the closure that are not local are
/// dependencies and are imported as new records; entities in the closure that
/// are local but disabled are left untouched.
pub(crate) struct RefreshPlan {
    closure: ImportClosure,
}

impl RefreshPlan {
    /// Build the refresh plan from the catalogue graph and the destination
    /// library.
    ///
    /// Fails with a structured conflict before any mutation if adopting the
    /// catalogue's tag semantics would make existing local assignments
    /// invalid.
    pub(crate) fn build(graph: &ImportGraph, destination: &DestinationState) -> Result<Self> {
        let mut closure = ImportClosure::default();

        for person in graph.persons.values() {
            if destination.persons.contains(&person.person_id)
                && destination.enabled_persons.contains(&person.person_id)
            {
                closure.persons.insert(person.person_id.clone());
            }
        }
        for role in graph.roles.values() {
            if destination.roles.contains(&role.role_id)
                && destination.enabled_roles.contains(&role.role_id)
            {
                closure.roles.insert(role.role_id.clone());
            }
        }
        for instrument in graph.instruments.values() {
            if destination.instruments.contains(&instrument.instrument_id)
                && destination
                    .enabled_instruments
                    .contains(&instrument.instrument_id)
            {
                closure.instruments.insert(instrument.instrument_id.clone());
            }
        }
        for tag in graph.tags.values() {
            if destination.tags.contains(&tag.tag_id)
                && destination.enabled_tags.contains(&tag.tag_id)
            {
                // Adopting the catalogue's `takes_value` must not invalidate
                // any local assignment; otherwise the refresh cannot proceed
                // without changing what the local assignments mean.
                if let Some(local) = destination.tag_rows.get(&tag.tag_id) {
                    if local.takes_value != tag.takes_value
                        && destination.tags_in_use.contains(&tag.tag_id)
                    {
                        return Err(LibraryError::ImportConflict(format!(
                            "Tag {} is assigned in this library with a different value rule \
                             than the catalogue now defines",
                            tag.tag_id
                        ))
                        .into());
                    }
                }

                closure.tags.insert(tag.tag_id.clone());
            }
        }

        for work in graph.works.values() {
            if destination.works.contains(&work.work_id)
                && destination.enabled_works.contains(&work.work_id)
            {
                graph.closure_work(&mut closure, &work.work_id)?;
            }
        }

        for ensemble in graph.ensembles.values() {
            if destination.ensembles.contains(&ensemble.ensemble_id)
                && destination
                    .enabled_ensembles
                    .contains(&ensemble.ensemble_id)
            {
                graph.closure_ensemble(&mut closure, &ensemble.ensemble_id)?;
            }
        }

        for recording in graph.recordings.values() {
            if destination.recordings.contains(&recording.recording_id)
                && destination
                    .enabled_recordings
                    .contains(&recording.recording_id)
            {
                graph.closure_recording(&mut closure, &recording.recording_id)?;
            }
        }

        for album in graph.albums.values() {
            if destination.albums.contains(&album.album_id)
                && destination.enabled_albums.contains(&album.album_id)
            {
                closure.albums.insert(album.album_id.clone());
            }
        }

        Ok(Self { closure })
    }

    fn includes(&self, kind: EntityKind, id: &str) -> bool {
        match kind {
            EntityKind::Person => self.closure.persons.contains(id),
            EntityKind::Role => self.closure.roles.contains(id),
            EntityKind::Instrument => self.closure.instruments.contains(id),
            EntityKind::Tag => self.closure.tags.contains(id),
            EntityKind::Work => self.closure.works.contains(id),
            EntityKind::Ensemble => self.closure.ensembles.contains(id),
            EntityKind::Recording => self.closure.recordings.contains(id),
            EntityKind::Album => self.closure.albums.contains(id),
            EntityKind::Track => false,
        }
    }
}

/// Apply a refresh plan to the destination, inside a transaction.
pub(crate) fn refresh_metadata(
    graph: &ImportGraph,
    destination_connection: &mut SqliteConnection,
) -> Result<()> {
    let destination = DestinationState::load(destination_connection)?;
    let plan = RefreshPlan::build(graph, &destination)?;
    destination_connection.transaction::<(), LibraryError, _>(|connection| {
        insert_dependencies(connection, graph, &plan, &destination)?;
        replace_enabled(connection, graph, &plan, &destination)
    })
}

/// Insert every entity the closure needs that is not local yet, in
/// foreign-key-safe order.
fn insert_dependencies(
    connection: &mut SqliteConnection,
    graph: &ImportGraph,
    plan: &RefreshPlan,
    destination: &DestinationState,
) -> Result<()> {
    for row in graph.persons.values() {
        if plan.includes(EntityKind::Person, &row.person_id)
            && !destination.persons.contains(&row.person_id)
        {
            let mut row = row.clone();
            row.source = Source::Metadata;
            row.created_at = db::now();
            row.edited_at = db::now();
            row.last_used_at = db::now();
            diesel::insert_into(persons::table)
                .values(&row)
                .execute(connection)?;
        }
    }
    for row in graph.roles.values() {
        if plan.includes(EntityKind::Role, &row.role_id)
            && !destination.roles.contains(&row.role_id)
        {
            let mut row = row.clone();
            row.source = Source::Metadata;
            row.created_at = db::now();
            row.edited_at = db::now();
            row.last_used_at = db::now();
            diesel::insert_into(roles::table)
                .values(&row)
                .execute(connection)?;
        }
    }
    for row in graph.tags.values() {
        if plan.includes(EntityKind::Tag, &row.tag_id) && !destination.tags.contains(&row.tag_id) {
            let mut row = row.clone();
            row.source = Source::Metadata;
            row.created_at = db::now();
            row.edited_at = db::now();
            row.last_used_at = db::now();
            diesel::insert_into(tags::table)
                .values(&row)
                .execute(connection)?;
        }
    }
    for row in graph.instruments.values() {
        if plan.includes(EntityKind::Instrument, &row.instrument_id)
            && !destination.instruments.contains(&row.instrument_id)
        {
            let mut row = row.clone();
            row.source = Source::Metadata;
            row.created_at = db::now();
            row.edited_at = db::now();
            row.last_used_at = db::now();
            diesel::insert_into(instruments::table)
                .values(&row)
                .execute(connection)?;
        }
    }
    // Works parents first, so a child never precedes its parent.
    let mut work_ids = plan
        .closure
        .works
        .iter()
        .filter(|id| !destination.works.contains(*id))
        .cloned()
        .collect::<Vec<_>>();

    // Simple topological sort by parent relationship (acyclic by
    // validation): a work is inserted only once its parent is already there.
    let mut inserted = HashSet::new();
    while !work_ids.is_empty() {
        let mut progressed = false;
        let mut remaining = Vec::new();

        for id in work_ids.drain(..) {
            let parent_present = graph
                .works
                .get(&id)
                .and_then(|work| work.parent_work_id.clone())
                .is_none_or(|parent| inserted.contains(&parent));

            if !parent_present {
                remaining.push(id);
                continue;
            }

            let work = graph.works.get(&id).ok_or_else(|| {
                LibraryError::Corrupt(format!("The catalogue is missing the work {id}"))
            })?;

            let mut row = work.clone();
            row.source = Source::Metadata;
            row.created_at = db::now();
            row.edited_at = db::now();
            row.last_used_at = db::now();
            diesel::insert_into(works::table)
                .values(&row)
                .execute(connection)?;

            inserted.insert(id);
            progressed = true;
        }

        if !progressed {
            return Err(LibraryError::Corrupt(
                "The new works of the catalogue form a parent cycle".to_owned(),
            )
            .into());
        }

        work_ids = remaining;
    }

    for row in graph.ensembles.values() {
        if plan.includes(EntityKind::Ensemble, &row.ensemble_id)
            && !destination.ensembles.contains(&row.ensemble_id)
        {
            let mut row = row.clone();
            row.source = Source::Metadata;
            row.created_at = db::now();
            row.edited_at = db::now();
            row.last_used_at = db::now();
            diesel::insert_into(ensembles::table)
                .values(&row)
                .execute(connection)?;
        }
    }

    // NB: recordings and albums are separate; their dependencies are inserted
    // by the entity loops above/around because a metadata catalogue never
    // creates new recordings or albums (they only exist in user libraries).
    Ok(())
}
/// Replace the catalogue-owned data of every included local enabled entity,
/// touching edit timestamps only when something actually changed.
fn replace_enabled(
    connection: &mut SqliteConnection,
    graph: &ImportGraph,
    plan: &RefreshPlan,
    destination: &DestinationState,
) -> Result<()> {
    for row in graph.persons.values() {
        if plan.includes(EntityKind::Person, &row.person_id)
            && destination.enabled_persons.contains(&row.person_id)
        {
            let local = destination.person_rows.get(&row.person_id);
            if local.is_none_or(|local| local.name != row.name) {
                let edited_at = db::now();
                diesel::update(persons::table.filter(persons::person_id.eq(&row.person_id)))
                    .set((
                        persons::name.eq(row.name.clone()),
                        persons::edited_at.eq(edited_at),
                        persons::last_used_at.eq(edited_at),
                    ))
                    .execute(connection)?;
            }
        }
    }

    for row in graph.roles.values() {
        if plan.includes(EntityKind::Role, &row.role_id)
            && destination.enabled_roles.contains(&row.role_id)
        {
            let local = destination.role_rows.get(&row.role_id);
            if local.is_none_or(|local| local.name != row.name) {
                let edited_at = db::now();
                diesel::update(roles::table.filter(roles::role_id.eq(&row.role_id)))
                    .set((
                        roles::name.eq(row.name.clone()),
                        roles::edited_at.eq(edited_at),
                        roles::last_used_at.eq(edited_at),
                    ))
                    .execute(connection)?;
            }
        }
    }

    for row in graph.instruments.values() {
        if plan.includes(EntityKind::Instrument, &row.instrument_id)
            && destination.enabled_instruments.contains(&row.instrument_id)
        {
            let local = destination.instrument_rows.get(&row.instrument_id);
            if local.is_none_or(|local| local.name != row.name) {
                let edited_at = db::now();
                diesel::update(
                    instruments::table.filter(instruments::instrument_id.eq(&row.instrument_id)),
                )
                .set((
                    instruments::name.eq(row.name.clone()),
                    instruments::edited_at.eq(edited_at),
                    instruments::last_used_at.eq(edited_at),
                ))
                .execute(connection)?;
            }
        }
    }

    for row in graph.tags.values() {
        if plan.includes(EntityKind::Tag, &row.tag_id)
            && destination.enabled_tags.contains(&row.tag_id)
        {
            let local = destination.tag_rows.get(&row.tag_id);
            // `private` is local-owned and never changed; `takes_value` is
            // schema-defining and was checked for conflicts when the plan was
            // built.
            if local
                .is_none_or(|local| local.name != row.name || local.takes_value != row.takes_value)
            {
                let edited_at = db::now();
                diesel::update(tags::table.filter(tags::tag_id.eq(&row.tag_id)))
                    .set((
                        tags::name.eq(row.name.clone()),
                        tags::takes_value.eq(row.takes_value),
                        tags::edited_at.eq(edited_at),
                        tags::last_used_at.eq(edited_at),
                    ))
                    .execute(connection)?;
            }
        }
    }

    for work in graph.works.values() {
        if plan.includes(EntityKind::Work, &work.work_id)
            && destination.enabled_works.contains(&work.work_id)
        {
            let local = destination.work_rows.get(&work.work_id);
            let mut changed = local.is_none_or(|local| {
                local.name != work.name
                    || local.parent_work_id != work.parent_work_id
                    || local.sequence_number != work.sequence_number
            });

            if changed {
                let edited_at = db::now();
                diesel::update(works::table.filter(works::work_id.eq(&work.work_id)))
                    .set((
                        works::name.eq(work.name.clone()),
                        works::parent_work_id.eq(work.parent_work_id.clone()),
                        works::sequence_number.eq(work.sequence_number),
                        works::edited_at.eq(edited_at),
                        works::last_used_at.eq(edited_at),
                    ))
                    .execute(connection)?;
            }

            changed |= replace_work_persons(connection, &work.work_id, graph, destination)?;
            changed |= replace_work_instruments(connection, &work.work_id, graph, destination)?;
            changed |= replace_work_tags(connection, &work.work_id, graph, destination)?;
            changed |= replace_work_relations(connection, &work.work_id, graph, destination)?;

            if changed {
                let edited_at = db::now();
                diesel::update(works::table.filter(works::work_id.eq(&work.work_id)))
                    .set((
                        works::edited_at.eq(edited_at),
                        works::last_used_at.eq(edited_at),
                    ))
                    .execute(connection)?;
            }
        }
    }

    for ensemble in graph.ensembles.values() {
        if plan.includes(EntityKind::Ensemble, &ensemble.ensemble_id)
            && destination
                .enabled_ensembles
                .contains(&ensemble.ensemble_id)
        {
            let local = destination.ensemble_rows.get(&ensemble.ensemble_id);
            let mut changed = local.is_none_or(|local| local.name != ensemble.name);

            if changed {
                let edited_at = db::now();
                diesel::update(
                    ensembles::table.filter(ensembles::ensemble_id.eq(&ensemble.ensemble_id)),
                )
                .set((
                    ensembles::name.eq(ensemble.name.clone()),
                    ensembles::edited_at.eq(edited_at),
                    ensembles::last_used_at.eq(edited_at),
                ))
                .execute(connection)?;
            }

            changed |=
                replace_ensemble_persons(connection, &ensemble.ensemble_id, graph, destination)?;

            if changed {
                let edited_at = db::now();
                diesel::update(
                    ensembles::table.filter(ensembles::ensemble_id.eq(&ensemble.ensemble_id)),
                )
                .set((
                    ensembles::edited_at.eq(edited_at),
                    ensembles::last_used_at.eq(edited_at),
                ))
                .execute(connection)?;
            }
        }
    }

    for recording in graph.recordings.values() {
        if plan.includes(EntityKind::Recording, &recording.recording_id)
            && destination
                .enabled_recordings
                .contains(&recording.recording_id)
        {
            let local = destination.recording_rows.get(&recording.recording_id);
            let mut changed = local.is_none_or(|local| {
                local.work_id != recording.work_id || local.comment != recording.comment
            });

            if changed {
                let edited_at = db::now();
                diesel::update(
                    recordings::table.filter(recordings::recording_id.eq(&recording.recording_id)),
                )
                .set((
                    recordings::work_id.eq(recording.work_id.clone()),
                    recordings::comment.eq(recording.comment.clone()),
                    recordings::edited_at.eq(edited_at),
                    recordings::last_used_at.eq(edited_at),
                ))
                .execute(connection)?;
            }

            changed |=
                replace_recording_persons(connection, &recording.recording_id, graph, destination)?;
            changed |= replace_recording_ensembles(
                connection,
                &recording.recording_id,
                graph,
                destination,
            )?;
            changed |=
                replace_recording_tags(connection, &recording.recording_id, graph, destination)?;

            if changed {
                let edited_at = db::now();
                diesel::update(
                    recordings::table.filter(recordings::recording_id.eq(&recording.recording_id)),
                )
                .set((
                    recordings::edited_at.eq(edited_at),
                    recordings::last_used_at.eq(edited_at),
                ))
                .execute(connection)?;
            }
        }
    }

    for album in graph.albums.values() {
        if plan.includes(EntityKind::Album, &album.album_id)
            && destination.enabled_albums.contains(&album.album_id)
        {
            let local = destination.album_rows.get(&album.album_id);
            let mut changed = local.is_none_or(|local| local.name != album.name);

            if changed {
                let edited_at = db::now();
                diesel::update(albums::table.filter(albums::album_id.eq(&album.album_id)))
                    .set((
                        albums::name.eq(album.name.clone()),
                        albums::edited_at.eq(edited_at),
                        albums::last_used_at.eq(edited_at),
                    ))
                    .execute(connection)?;
            }

            changed |= replace_album_recordings(connection, &album.album_id, graph, destination)?;

            if changed {
                let edited_at = db::now();
                diesel::update(albums::table.filter(albums::album_id.eq(&album.album_id)))
                    .set((
                        albums::edited_at.eq(edited_at),
                        albums::last_used_at.eq(edited_at),
                    ))
                    .execute(connection)?;
            }
        }
    }

    Ok(())
}

/// Whether two ordered association row sets are equal, row by row, in their
/// stored order (identified by sequence number).
fn rows_equal<Row>(left: &[Row], right: &[Row], key: impl Fn(&Row) -> Vec<String>) -> bool {
    if left.len() != right.len() {
        return false;
    }

    let mut left = left.iter().map(&key).collect::<Vec<_>>();
    let mut right = right.iter().map(|row| key(row)).collect::<Vec<_>>();
    left.sort();
    right.sort();

    left == right
}

fn row_key(values: Vec<Option<String>>) -> Vec<String> {
    values
        .into_iter()
        .map(|value| value.unwrap_or_default())
        .collect()
}

fn replace_work_persons(
    connection: &mut SqliteConnection,
    work_id: &str,
    graph: &ImportGraph,
    _destination: &DestinationState,
) -> Result<bool> {
    let catalogue = graph.work_persons_of(work_id);
    let local = work_persons::table
        .filter(work_persons::work_id.eq(work_id))
        .load::<tables::WorkPerson>(connection)?;

    if rows_equal(catalogue, &local, |row| {
        row_key(vec![
            Some(row.work_id.clone()),
            Some(row.person_id.clone()),
            row.role_id.clone(),
            Some(row.sequence_number.to_string()),
        ])
    }) {
        return Ok(false);
    }

    diesel::delete(work_persons::table.filter(work_persons::work_id.eq(work_id)))
        .execute(connection)?;

    for row in catalogue {
        diesel::insert_into(work_persons::table)
            .values(row.clone())
            .execute(connection)?;
    }

    Ok(true)
}

fn replace_work_instruments(
    connection: &mut SqliteConnection,
    work_id: &str,
    graph: &ImportGraph,
    _destination: &DestinationState,
) -> Result<bool> {
    let catalogue = graph.work_instruments_of(work_id);
    let local = work_instruments::table
        .filter(work_instruments::work_id.eq(work_id))
        .load::<tables::WorkInstrument>(connection)?;

    if rows_equal(catalogue, &local, |row| {
        row_key(vec![
            Some(row.work_id.clone()),
            Some(row.instrument_id.clone()),
            Some(row.sequence_number.to_string()),
        ])
    }) {
        return Ok(false);
    }

    diesel::delete(work_instruments::table.filter(work_instruments::work_id.eq(work_id)))
        .execute(connection)?;

    for row in catalogue {
        diesel::insert_into(work_instruments::table)
            .values(row.clone())
            .execute(connection)?;
    }

    Ok(true)
}

fn replace_work_tags(
    connection: &mut SqliteConnection,
    work_id: &str,
    graph: &ImportGraph,
    _destination: &DestinationState,
) -> Result<bool> {
    let catalogue = graph.work_tags_of(work_id);
    let local = work_tags::table
        .filter(work_tags::work_id.eq(work_id))
        .load::<tables::WorkTag>(connection)?;

    if rows_equal(catalogue, &local, |row| {
        row_key(vec![
            Some(row.work_id.clone()),
            Some(row.tag_id.clone()),
            row.value.as_ref().map(|value| value.get().to_owned()),
            Some(row.sequence_number.to_string()),
        ])
    }) {
        return Ok(false);
    }

    diesel::delete(work_tags::table.filter(work_tags::work_id.eq(work_id))).execute(connection)?;

    for row in catalogue {
        diesel::insert_into(work_tags::table)
            .values(row.clone())
            .execute(connection)?;
    }

    Ok(true)
}

fn replace_work_relations(
    connection: &mut SqliteConnection,
    work_id: &str,
    graph: &ImportGraph,
    _destination: &DestinationState,
) -> Result<bool> {
    let catalogue = graph.work_relations_of(work_id);
    let local = work_relations::table
        .filter(work_relations::work_id.eq(work_id))
        .load::<tables::WorkRelation>(connection)?;

    if rows_equal(catalogue, &local, |row| {
        row_key(vec![
            Some(row.work_id.clone()),
            Some(row.related_work_id.clone()),
        ])
    }) {
        return Ok(false);
    }

    diesel::delete(work_relations::table.filter(work_relations::work_id.eq(work_id)))
        .execute(connection)?;

    for row in catalogue {
        diesel::insert_into(work_relations::table)
            .values(row.clone())
            .execute(connection)?;
    }

    Ok(true)
}

fn replace_ensemble_persons(
    connection: &mut SqliteConnection,
    ensemble_id: &str,
    graph: &ImportGraph,
    _destination: &DestinationState,
) -> Result<bool> {
    let catalogue = graph.ensemble_persons_of(ensemble_id);
    let local = ensemble_persons::table
        .filter(ensemble_persons::ensemble_id.eq(ensemble_id))
        .load::<tables::EnsemblePerson>(connection)?;

    if rows_equal(catalogue, &local, |row| {
        row_key(vec![
            Some(row.ensemble_id.clone()),
            Some(row.person_id.clone()),
            row.role_id.clone(),
            row.instrument_id.clone(),
            Some(row.sequence_number.to_string()),
        ])
    }) {
        return Ok(false);
    }

    diesel::delete(ensemble_persons::table.filter(ensemble_persons::ensemble_id.eq(ensemble_id)))
        .execute(connection)?;

    for row in catalogue {
        diesel::insert_into(ensemble_persons::table)
            .values(row.clone())
            .execute(connection)?;
    }

    Ok(true)
}

fn replace_recording_persons(
    connection: &mut SqliteConnection,
    recording_id: &str,
    graph: &ImportGraph,
    _destination: &DestinationState,
) -> Result<bool> {
    let catalogue = graph.recording_persons_of(recording_id);
    let local = recording_persons::table
        .filter(recording_persons::recording_id.eq(recording_id))
        .load::<tables::RecordingPerson>(connection)?;

    if rows_equal(catalogue, &local, |row| {
        row_key(vec![
            Some(row.recording_id.clone()),
            Some(row.person_id.clone()),
            row.role_id.clone(),
            row.instrument_id.clone(),
            Some(row.sequence_number.to_string()),
        ])
    }) {
        return Ok(false);
    }

    diesel::delete(
        recording_persons::table.filter(recording_persons::recording_id.eq(recording_id)),
    )
    .execute(connection)?;

    for row in catalogue {
        diesel::insert_into(recording_persons::table)
            .values(row.clone())
            .execute(connection)?;
    }

    Ok(true)
}

fn replace_recording_ensembles(
    connection: &mut SqliteConnection,
    recording_id: &str,
    graph: &ImportGraph,
    _destination: &DestinationState,
) -> Result<bool> {
    let catalogue = graph.recording_ensembles_of(recording_id);
    let local = recording_ensembles::table
        .filter(recording_ensembles::recording_id.eq(recording_id))
        .load::<tables::RecordingEnsemble>(connection)?;

    if rows_equal(catalogue, &local, |row| {
        row_key(vec![
            Some(row.recording_id.clone()),
            Some(row.ensemble_id.clone()),
            row.role_id.clone(),
            Some(row.sequence_number.to_string()),
        ])
    }) {
        return Ok(false);
    }

    diesel::delete(
        recording_ensembles::table.filter(recording_ensembles::recording_id.eq(recording_id)),
    )
    .execute(connection)?;

    for row in catalogue {
        diesel::insert_into(recording_ensembles::table)
            .values(row.clone())
            .execute(connection)?;
    }

    Ok(true)
}

fn replace_recording_tags(
    connection: &mut SqliteConnection,
    recording_id: &str,
    graph: &ImportGraph,
    _destination: &DestinationState,
) -> Result<bool> {
    let catalogue = graph.recording_tags_of(recording_id);
    let local = recording_tags::table
        .filter(recording_tags::recording_id.eq(recording_id))
        .load::<tables::RecordingTag>(connection)?;

    if rows_equal(catalogue, &local, |row| {
        row_key(vec![
            Some(row.recording_id.clone()),
            Some(row.tag_id.clone()),
            row.value.as_ref().map(|value| value.get().to_owned()),
            Some(row.sequence_number.to_string()),
        ])
    }) {
        return Ok(false);
    }

    diesel::delete(recording_tags::table.filter(recording_tags::recording_id.eq(recording_id)))
        .execute(connection)?;

    for row in catalogue {
        diesel::insert_into(recording_tags::table)
            .values(row.clone())
            .execute(connection)?;
    }

    Ok(true)
}

fn replace_album_recordings(
    connection: &mut SqliteConnection,
    album_id: &str,
    graph: &ImportGraph,
    _destination: &DestinationState,
) -> Result<bool> {
    let catalogue = graph.album_recordings_of(album_id);
    let local = album_recordings::table
        .filter(album_recordings::album_id.eq(album_id))
        .load::<tables::AlbumRecording>(connection)?;

    if rows_equal(catalogue, &local, |row| {
        row_key(vec![
            Some(row.album_id.clone()),
            Some(row.recording_id.clone()),
            Some(row.sequence_number.to_string()),
        ])
    }) {
        return Ok(false);
    }

    diesel::delete(album_recordings::table.filter(album_recordings::album_id.eq(album_id)))
        .execute(connection)?;

    for row in catalogue {
        diesel::insert_into(album_recordings::table)
            .values(row.clone())
            .execute(connection)?;
    }

    Ok(true)
}
