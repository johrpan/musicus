//! A validated snapshot of a source database.
//!
//! Import paths read their source (a `.muslib` archive or the metadata
//! catalogue) through [`ImportGraph`]: every table is loaded once and indexed
//! by ID and owner, and the whole graph is validated before any row is written
//! to a destination. A malformed source is rejected here, before the first
//! mutation, instead of halfway through an import that has already changed the
//! destination.

use std::collections::{HashMap, HashSet};

use diesel::prelude::*;

use crate::{
    db::{self, schema::*, tables, tables::Source},
    error::{EntityKind, LibraryError, Result},
};

/// Every row of a source database, loaded once and indexed.
///
/// Association rows are grouped by owner so that import and refresh loops read
/// an owner's rows directly instead of rescanning a whole table.
pub(crate) struct ImportGraph {
    pub(crate) persons: HashMap<String, tables::Person>,
    pub(crate) roles: HashMap<String, tables::Role>,
    pub(crate) instruments: HashMap<String, tables::Instrument>,
    pub(crate) tags: HashMap<String, tables::Tag>,
    pub(crate) works: HashMap<String, tables::Work>,
    /// Outgoing relation rows per source work.
    pub(crate) work_relations: HashMap<String, Vec<tables::WorkRelation>>,
    /// Child work IDs per parent, in their stored order.
    pub(crate) children: HashMap<String, Vec<String>>,
    pub(crate) work_persons: HashMap<String, Vec<tables::WorkPerson>>,
    pub(crate) work_instruments: HashMap<String, Vec<tables::WorkInstrument>>,
    pub(crate) work_tags: HashMap<String, Vec<tables::WorkTag>>,
    pub(crate) ensembles: HashMap<String, tables::Ensemble>,
    pub(crate) ensemble_persons: HashMap<String, Vec<tables::EnsemblePerson>>,
    pub(crate) recordings: HashMap<String, tables::Recording>,
    pub(crate) recording_persons: HashMap<String, Vec<tables::RecordingPerson>>,
    pub(crate) recording_ensembles: HashMap<String, Vec<tables::RecordingEnsemble>>,
    pub(crate) recording_tags: HashMap<String, Vec<tables::RecordingTag>>,
    pub(crate) tracks: HashMap<String, tables::Track>,
    pub(crate) track_works: HashMap<String, Vec<tables::TrackWork>>,
    pub(crate) albums: HashMap<String, tables::Album>,
    pub(crate) album_recordings: HashMap<String, Vec<tables::AlbumRecording>>,
}

impl ImportGraph {
    /// Load every relevant table of `connection` (a source database) and
    /// validate the resulting graph. Nothing is mutated.
    pub(crate) fn load(connection: &mut SqliteConnection) -> Result<Self> {
        let works = works::table.load::<tables::Work>(connection)?;

        let mut children = HashMap::<String, Vec<String>>::new();
        for work in &works {
            if let Some(parent) = &work.parent_work_id {
                children
                    .entry(parent.clone())
                    .or_default()
                    .push(work.work_id.clone());
            }
        }

        let graph = Self {
            persons: keyed_by(persons::table.load::<tables::Person>(connection)?, |row| {
                row.person_id.clone()
            }),
            roles: keyed_by(roles::table.load::<tables::Role>(connection)?, |row| {
                row.role_id.clone()
            }),
            instruments: keyed_by(
                instruments::table.load::<tables::Instrument>(connection)?,
                |row| row.instrument_id.clone(),
            ),
            tags: keyed_by(tags::table.load::<tables::Tag>(connection)?, |row| {
                row.tag_id.clone()
            }),
            works: keyed_by(works, |row| row.work_id.clone()),
            children,
            work_relations: grouped_by(
                work_relations::table.load::<tables::WorkRelation>(connection)?,
                |row| row.work_id.clone(),
            ),
            work_persons: grouped_by(
                work_persons::table.load::<tables::WorkPerson>(connection)?,
                |row| row.work_id.clone(),
            ),
            work_instruments: grouped_by(
                work_instruments::table.load::<tables::WorkInstrument>(connection)?,
                |row| row.work_id.clone(),
            ),
            work_tags: grouped_by(
                work_tags::table.load::<tables::WorkTag>(connection)?,
                |row| row.work_id.clone(),
            ),
            ensembles: keyed_by(
                ensembles::table.load::<tables::Ensemble>(connection)?,
                |row| row.ensemble_id.clone(),
            ),
            ensemble_persons: grouped_by(
                ensemble_persons::table.load::<tables::EnsemblePerson>(connection)?,
                |row| row.ensemble_id.clone(),
            ),
            recordings: keyed_by(
                recordings::table.load::<tables::Recording>(connection)?,
                |row| row.recording_id.clone(),
            ),
            recording_persons: grouped_by(
                recording_persons::table.load::<tables::RecordingPerson>(connection)?,
                |row| row.recording_id.clone(),
            ),
            recording_ensembles: grouped_by(
                recording_ensembles::table.load::<tables::RecordingEnsemble>(connection)?,
                |row| row.recording_id.clone(),
            ),
            recording_tags: grouped_by(
                recording_tags::table.load::<tables::RecordingTag>(connection)?,
                |row| row.recording_id.clone(),
            ),
            tracks: keyed_by(tracks::table.load::<tables::Track>(connection)?, |row| {
                row.track_id.clone()
            }),
            track_works: grouped_by(
                track_works::table.load::<tables::TrackWork>(connection)?,
                |row| row.track_id.clone(),
            ),
            albums: keyed_by(albums::table.load::<tables::Album>(connection)?, |row| {
                row.album_id.clone()
            }),
            album_recordings: grouped_by(
                album_recordings::table.load::<tables::AlbumRecording>(connection)?,
                |row| row.album_id.clone(),
            ),
        };

        graph.validate()?;

        Ok(graph)
    }

    /// A work's outgoing relation rows.
    pub(crate) fn work_relations_of(&self, work_id: &str) -> &[tables::WorkRelation] {
        self.work_relations.get(work_id).map_or(&[], Vec::as_slice)
    }

    /// A work's credited-person rows.
    pub(crate) fn work_persons_of(&self, work_id: &str) -> &[tables::WorkPerson] {
        self.work_persons.get(work_id).map_or(&[], Vec::as_slice)
    }

    /// A work's instrument rows.
    pub(crate) fn work_instruments_of(&self, work_id: &str) -> &[tables::WorkInstrument] {
        self.work_instruments
            .get(work_id)
            .map_or(&[], Vec::as_slice)
    }

    /// A work's tag rows.
    pub(crate) fn work_tags_of(&self, work_id: &str) -> &[tables::WorkTag] {
        self.work_tags.get(work_id).map_or(&[], Vec::as_slice)
    }

    /// An ensemble's member rows.
    pub(crate) fn ensemble_persons_of(&self, ensemble_id: &str) -> &[tables::EnsemblePerson] {
        self.ensemble_persons
            .get(ensemble_id)
            .map_or(&[], Vec::as_slice)
    }

    /// A recording's performing-person rows.
    pub(crate) fn recording_persons_of(&self, recording_id: &str) -> &[tables::RecordingPerson] {
        self.recording_persons
            .get(recording_id)
            .map_or(&[], Vec::as_slice)
    }

    /// A recording's performing-ensemble rows.
    pub(crate) fn recording_ensembles_of(
        &self,
        recording_id: &str,
    ) -> &[tables::RecordingEnsemble] {
        self.recording_ensembles
            .get(recording_id)
            .map_or(&[], Vec::as_slice)
    }

    /// A recording's tag rows.
    pub(crate) fn recording_tags_of(&self, recording_id: &str) -> &[tables::RecordingTag] {
        self.recording_tags
            .get(recording_id)
            .map_or(&[], Vec::as_slice)
    }

    /// A track's associated-work rows.
    pub(crate) fn track_works_of(&self, track_id: &str) -> &[tables::TrackWork] {
        self.track_works.get(track_id).map_or(&[], Vec::as_slice)
    }

    /// An album's recording rows.
    pub(crate) fn album_recordings_of(&self, album_id: &str) -> &[tables::AlbumRecording] {
        self.album_recordings
            .get(album_id)
            .map_or(&[], Vec::as_slice)
    }

    /// Check the whole graph before any of its rows is written anywhere.
    fn validate(&self) -> Result<()> {
        self.validate_work_hierarchy()?;
        self.validate_work_rows()?;
        self.validate_ensemble_rows()?;
        self.validate_recording_rows()?;
        self.validate_track_rows()?;
        self.validate_album_rows()?;

        Ok(())
    }

    /// Works: parents exist, root/child flags match, child sequence numbers
    /// are unique per parent, and no work is its own ancestor.
    fn validate_work_hierarchy(&self) -> Result<()> {
        let mut sequences = HashMap::<&str, HashSet<i32>>::new();

        for work in self.works.values() {
            match (&work.parent_work_id, work.sequence_number) {
                (None, None) => {}
                (Some(parent), Some(sequence)) => {
                    if !self.works.contains_key(parent) {
                        return Err(corrupt(format!(
                            "Work {} belongs to the missing parent {parent}",
                            work.work_id
                        )));
                    }

                    if sequence < 0 {
                        return Err(corrupt(format!(
                            "Work {} has the negative sequence number {sequence}",
                            work.work_id
                        )));
                    }

                    if !sequences
                        .entry(parent.as_str())
                        .or_default()
                        .insert(sequence)
                    {
                        return Err(corrupt(format!(
                            "The works under {parent} share the sequence number {sequence}"
                        )));
                    }
                }
                (Some(_), None) => {
                    return Err(corrupt(format!(
                        "Work {} has a parent but no sequence number",
                        work.work_id
                    )))
                }
                (None, Some(_)) => {
                    return Err(corrupt(format!(
                        "Work {} has a sequence number but no parent",
                        work.work_id
                    )))
                }
            }
        }

        // Following the parent chain from any work must terminate before it
        // returns to a work already on the chain.
        for id in self.works.keys() {
            let mut active = HashSet::new();
            let mut current = id.as_str();

            while let Some(work) = self.works.get(current) {
                if !active.insert(current.to_owned()) {
                    return Err(corrupt(format!("Work {current} is part of a parent cycle")));
                }

                match work.parent_work_id.as_deref() {
                    Some(parent) => current = parent,
                    None => break,
                }
            }
        }

        Ok(())
    }

    fn validate_work_rows(&self) -> Result<()> {
        for relation in self.work_relations.values().flatten() {
            self.require_work(
                EntityKind::Work,
                &relation.work_id,
                &relation.related_work_id,
            )?;
        }

        for row in self.work_persons.values().flatten() {
            self.require_person(EntityKind::Work, &row.work_id, &row.person_id)?;
            if let Some(role_id) = &row.role_id {
                self.require_role(EntityKind::Work, &row.work_id, role_id)?;
            }
            check_sequence(EntityKind::Work, &row.work_id, row.sequence_number)?;
        }

        for row in self.work_instruments.values().flatten() {
            self.require_instrument(EntityKind::Work, &row.work_id, &row.instrument_id)?;
            check_sequence(EntityKind::Work, &row.work_id, row.sequence_number)?;
        }

        for row in self.work_tags.values().flatten() {
            self.check_tag_row(EntityKind::Work, &row.work_id, &row.tag_id, &row.value)?;
            check_sequence(EntityKind::Work, &row.work_id, row.sequence_number)?;
        }

        Ok(())
    }

    fn validate_ensemble_rows(&self) -> Result<()> {
        for row in self.ensemble_persons.values().flatten() {
            self.require_person(EntityKind::Ensemble, &row.ensemble_id, &row.person_id)?;
            if let Some(role_id) = &row.role_id {
                self.require_role(EntityKind::Ensemble, &row.ensemble_id, role_id)?;
            }
            if let Some(instrument_id) = &row.instrument_id {
                self.require_instrument(EntityKind::Ensemble, &row.ensemble_id, instrument_id)?;
            }
            check_sequence(EntityKind::Ensemble, &row.ensemble_id, row.sequence_number)?;
        }

        Ok(())
    }

    fn validate_recording_rows(&self) -> Result<()> {
        for recording in self.recordings.values() {
            self.require_work(
                EntityKind::Recording,
                &recording.recording_id,
                &recording.work_id,
            )?;
        }

        for row in self.recording_persons.values().flatten() {
            self.require_person(EntityKind::Recording, &row.recording_id, &row.person_id)?;
            if let Some(role_id) = &row.role_id {
                self.require_role(EntityKind::Recording, &row.recording_id, role_id)?;
            }
            if let Some(instrument_id) = &row.instrument_id {
                self.require_instrument(EntityKind::Recording, &row.recording_id, instrument_id)?;
            }
            check_sequence(
                EntityKind::Recording,
                &row.recording_id,
                row.sequence_number,
            )?;
        }

        for row in self.recording_ensembles.values().flatten() {
            self.require_ensemble(EntityKind::Recording, &row.recording_id, &row.ensemble_id)?;
            if let Some(role_id) = &row.role_id {
                self.require_role(EntityKind::Recording, &row.recording_id, role_id)?;
            }
            check_sequence(
                EntityKind::Recording,
                &row.recording_id,
                row.sequence_number,
            )?;
        }

        for row in self.recording_tags.values().flatten() {
            self.check_tag_row(
                EntityKind::Recording,
                &row.recording_id,
                &row.tag_id,
                &row.value,
            )?;
            check_sequence(
                EntityKind::Recording,
                &row.recording_id,
                row.sequence_number,
            )?;
        }

        Ok(())
    }

    fn validate_track_rows(&self) -> Result<()> {
        let mut paths = HashSet::new();
        let mut per_recording = HashMap::<&str, HashSet<i32>>::new();

        for track in self.tracks.values() {
            if track.recording_index < 0 {
                return Err(corrupt(format!(
                    "Track {} has the negative index {}",
                    track.track_id, track.recording_index
                )));
            }

            if !paths.insert(track.path.as_str()) {
                return Err(corrupt(format!("Tracks share the file '{}'", track.path)));
            }

            if !self.recordings.contains_key(&track.recording_id) {
                return Err(corrupt(format!(
                    "Track {} belongs to the missing recording {}",
                    track.track_id, track.recording_id
                )));
            }

            if !per_recording
                .entry(&track.recording_id)
                .or_default()
                .insert(track.recording_index)
            {
                return Err(corrupt(format!(
                    "Recording {} has two tracks with the index {}",
                    track.recording_id, track.recording_index
                )));
            }
        }

        for row in self.track_works.values().flatten() {
            self.require_work(EntityKind::Track, &row.track_id, &row.work_id)?;
            check_sequence(EntityKind::Track, &row.track_id, row.sequence_number)?;
        }

        Ok(())
    }

    fn validate_album_rows(&self) -> Result<()> {
        let mut sequences = HashMap::<&str, HashSet<i32>>::new();

        for row in self.album_recordings.values().flatten() {
            if !self.recordings.contains_key(&row.recording_id) {
                return Err(corrupt(format!(
                    "Album {} contains the missing recording {}",
                    row.album_id, row.recording_id
                )));
            }

            if row.sequence_number < 0 {
                return Err(corrupt(format!(
                    "Album {} has the negative sequence number {}",
                    row.album_id, row.sequence_number
                )));
            }

            if !sequences
                .entry(&row.album_id)
                .or_default()
                .insert(row.sequence_number)
            {
                return Err(corrupt(format!(
                    "Album {} has two recordings with the sequence number {}",
                    row.album_id, row.sequence_number
                )));
            }
        }

        Ok(())
    }

    fn check_tag_row(
        &self,
        owner_kind: EntityKind,
        owner_id: &str,
        tag_id: &str,
        value: &Option<crate::db::TranslatedString>,
    ) -> Result<()> {
        let tag = self.tags.get(tag_id).ok_or_else(|| {
            corrupt(format!(
                "{owner_kind} {owner_id} uses the missing tag {tag_id}"
            ))
        })?;

        match (tag.takes_value, value) {
            (true, None) => Err(corrupt(format!(
                "Tag {tag_id} requires a value, but {owner_kind} {owner_id} has none"
            ))),
            (false, Some(_)) => Err(corrupt(format!(
                "Tag {tag_id} does not take a value, but {owner_kind} {owner_id} assigns one"
            ))),
            _ => Ok(()),
        }
    }

    fn require_person(&self, owner_kind: EntityKind, owner_id: &str, id: &str) -> Result<()> {
        require_present(&self.persons, owner_kind, owner_id, EntityKind::Person, id)
    }

    fn require_role(&self, owner_kind: EntityKind, owner_id: &str, id: &str) -> Result<()> {
        require_present(&self.roles, owner_kind, owner_id, EntityKind::Role, id)
    }

    fn require_instrument(&self, owner_kind: EntityKind, owner_id: &str, id: &str) -> Result<()> {
        require_present(
            &self.instruments,
            owner_kind,
            owner_id,
            EntityKind::Instrument,
            id,
        )
    }

    fn require_ensemble(&self, owner_kind: EntityKind, owner_id: &str, id: &str) -> Result<()> {
        require_present(
            &self.ensembles,
            owner_kind,
            owner_id,
            EntityKind::Ensemble,
            id,
        )
    }

    fn require_work(&self, owner_kind: EntityKind, owner_id: &str, id: &str) -> Result<()> {
        require_present(&self.works, owner_kind, owner_id, EntityKind::Work, id)
    }
}

/// Everything about the destination library that a local-wins merge needs to
/// know, loaded once before the import transaction starts.
pub(crate) struct DestinationState {
    pub(crate) persons: HashSet<String>,
    /// Local persons with `enable_updates = true`, for metadata refresh.
    pub(crate) enabled_persons: HashSet<String>,
    pub(crate) person_rows: HashMap<String, tables::Person>,
    pub(crate) roles: HashSet<String>,
    pub(crate) enabled_roles: HashSet<String>,
    pub(crate) role_rows: HashMap<String, tables::Role>,
    pub(crate) instruments: HashSet<String>,
    pub(crate) enabled_instruments: HashSet<String>,
    pub(crate) instrument_rows: HashMap<String, tables::Instrument>,
    pub(crate) tags: HashSet<String>,
    pub(crate) enabled_tags: HashSet<String>,
    /// Local tags that any work or recording is assigned, for refresh
    /// conflict checks.
    pub(crate) tags_in_use: HashSet<String>,
    pub(crate) tag_rows: HashMap<String, tables::Tag>,
    pub(crate) works: HashSet<String>,
    pub(crate) enabled_works: HashSet<String>,
    pub(crate) work_rows: HashMap<String, tables::Work>,
    /// The position of every local work, for structural conflict checks:
    /// parent and sequence as stored.
    pub(crate) work_positions: HashMap<String, (Option<String>, Option<i32>)>,
    /// Positions already taken by local children: (parent, sequence).
    pub(crate) child_positions: HashSet<(String, i32)>,
    pub(crate) ensembles: HashSet<String>,
    pub(crate) enabled_ensembles: HashSet<String>,
    pub(crate) ensemble_rows: HashMap<String, tables::Ensemble>,
    pub(crate) recordings: HashSet<String>,
    pub(crate) enabled_recordings: HashSet<String>,
    pub(crate) recording_rows: HashMap<String, tables::Recording>,
    pub(crate) albums: HashSet<String>,
    pub(crate) enabled_albums: HashSet<String>,
    pub(crate) album_rows: HashMap<String, tables::Album>,
    pub(crate) tracks: HashSet<String>,
    /// Local track paths, for path collision checks.
    pub(crate) track_paths: HashSet<String>,
}

impl DestinationState {
    pub(crate) fn load(connection: &mut SqliteConnection) -> Result<Self> {
        let persons = persons::table.load::<tables::Person>(connection)?;
        let roles = roles::table.load::<tables::Role>(connection)?;
        let instruments = instruments::table.load::<tables::Instrument>(connection)?;
        let tags = tags::table.load::<tables::Tag>(connection)?;
        let works = works::table.load::<tables::Work>(connection)?;
        let ensembles = ensembles::table.load::<tables::Ensemble>(connection)?;
        let recordings = recordings::table.load::<tables::Recording>(connection)?;
        let albums = albums::table.load::<tables::Album>(connection)?;

        let tags_in_use = work_tags::table
            .select(work_tags::tag_id)
            .load::<String>(connection)?
            .into_iter()
            .chain(
                recording_tags::table
                    .select(recording_tags::tag_id)
                    .load::<String>(connection)?,
            )
            .collect::<HashSet<_>>();

        let work_positions = works
            .iter()
            .map(|work| {
                (
                    work.work_id.clone(),
                    (work.parent_work_id.clone(), work.sequence_number),
                )
            })
            .collect::<HashMap<_, _>>();

        let child_positions = works
            .iter()
            .filter_map(|work| match (&work.parent_work_id, work.sequence_number) {
                (Some(parent), Some(sequence)) => Some((parent.clone(), sequence)),
                _ => None,
            })
            .collect::<HashSet<_>>();

        let person_rows = keyed_by(persons, |row| row.person_id.clone());
        let role_rows = keyed_by(roles, |row| row.role_id.clone());
        let instrument_rows = keyed_by(instruments, |row| row.instrument_id.clone());
        let tag_rows = keyed_by(tags, |row| row.tag_id.clone());
        let work_rows = keyed_by(works, |row| row.work_id.clone());
        let ensemble_rows = keyed_by(ensembles, |row| row.ensemble_id.clone());
        let recording_rows = keyed_by(recordings, |row| row.recording_id.clone());
        let album_rows = keyed_by(albums, |row| row.album_id.clone());

        Ok(Self {
            enabled_persons: person_rows
                .values()
                .filter(|row| row.enable_updates)
                .map(|row| row.person_id.clone())
                .collect(),
            enabled_roles: role_rows
                .values()
                .filter(|row| row.enable_updates)
                .map(|row| row.role_id.clone())
                .collect(),
            enabled_instruments: instrument_rows
                .values()
                .filter(|row| row.enable_updates)
                .map(|row| row.instrument_id.clone())
                .collect(),
            enabled_tags: tag_rows
                .values()
                .filter(|row| row.enable_updates)
                .map(|row| row.tag_id.clone())
                .collect(),
            enabled_works: work_rows
                .values()
                .filter(|row| row.enable_updates)
                .map(|row| row.work_id.clone())
                .collect(),
            enabled_ensembles: ensemble_rows
                .values()
                .filter(|row| row.enable_updates)
                .map(|row| row.ensemble_id.clone())
                .collect(),
            enabled_recordings: recording_rows
                .values()
                .filter(|row| row.enable_updates)
                .map(|row| row.recording_id.clone())
                .collect(),
            enabled_albums: album_rows
                .values()
                .filter(|row| row.enable_updates)
                .map(|row| row.album_id.clone())
                .collect(),
            persons: person_rows.keys().cloned().collect(),
            person_rows,
            roles: role_rows.keys().cloned().collect(),
            role_rows,
            instruments: instrument_rows.keys().cloned().collect(),
            instrument_rows,
            tags: tag_rows.keys().cloned().collect(),
            tag_rows,
            tags_in_use,
            works: work_rows.keys().cloned().collect(),
            work_rows,
            work_positions,
            child_positions,
            ensembles: ensemble_rows.keys().cloned().collect(),
            ensemble_rows,
            recordings: recording_rows.keys().cloned().collect(),
            recording_rows,
            albums: album_rows.keys().cloned().collect(),
            album_rows,
            tracks: tracks::table
                .select(tracks::track_id)
                .load::<String>(connection)?
                .into_iter()
                .collect::<HashSet<_>>(),
            track_paths: tracks::table
                .select(tracks::path)
                .load::<crate::path::LibraryTrackPath>(connection)?
                .into_iter()
                .map(|path| path.as_str().to_owned())
                .collect(),
        })
    }
}

/// How a local-wins import applied.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ImportReport {
    /// Rows written into the destination.
    pub(crate) inserted: usize,
    /// Entities that already existed locally and were kept unchanged.
    pub(crate) reused_local: usize,
    /// Imported rows dropped because their entity already exists locally.
    pub(crate) skipped: usize,
    /// Structural conflicts detected; always zero on success, because a
    /// conflict is an error, not a skipped row.
    pub(crate) conflicted: usize,
}

/// The import plan: local-wins decisions, made before any destination row is
/// written.
#[derive(Clone, Debug, Default)]
pub(crate) struct MergePlan {
    /// Track IDs of the source whose rows and files must be left alone because
    /// a local track already owns the ID or the file.
    pub(crate) skipped_tracks: HashSet<String>,
    /// The works of the source in an order in which parents come before their
    /// children.
    pub(crate) works_in_order: Vec<String>,
    /// The rendered report, filled while the plan is executed.
    pub(crate) report: std::cell::RefCell<ImportReport>,
}

impl ImportGraph {
    /// Decide the outcome of importing this graph into `destination`.
    ///
    /// With a `closure`, only the entities it selects are considered; without
    /// one, the whole graph. Local-wins keeps every colliding local entity
    /// entirely unchanged, so an imported parent/child structure that would
    /// require a colliding local work to be reparented, renumbered or
    /// otherwise disturbed is rejected here, before any destination row
    /// changes.
    pub(crate) fn plan_merge(
        &self,
        destination: &DestinationState,
        closure: Option<&ImportClosure>,
    ) -> Result<MergePlan> {
        let mut skipped_tracks = HashSet::new();

        if closure.is_none() {
            for track in self.tracks.values() {
                if destination.tracks.contains(&track.track_id)
                    || destination.track_paths.contains(track.path.as_str())
                {
                    // Skipping the imported track (and its owned rows) leaves
                    // every reference consistent: nothing else points at its ID.
                    skipped_tracks.insert(track.track_id.clone());
                }
            }
        }

        let mut child_positions = HashSet::new();
        let mut works_in_order = Vec::with_capacity(self.works.len());

        for work in self.works.values() {
            if !closure.is_none_or(|closure| closure.works.contains(&work.work_id)) {
                continue;
            }

            if destination.works.contains(&work.work_id) {
                // A colliding local work wins, so its position must already be
                // exactly what the imported structure assumes; otherwise the
                // import could only proceed by moving it.
                let local = destination
                    .work_positions
                    .get(&work.work_id)
                    .ok_or_else(|| {
                        LibraryError::Corrupt(
                            "Destination is inconsistent while planning the import".to_string(),
                        )
                    })?;

                if local.0.as_deref() != work.parent_work_id.as_deref()
                    || local.1 != work.sequence_number
                {
                    return Err(conflict(format!(
                        "Work {} already exists in this library with a different parent or position",
                        work.work_id
                    )));
                }

                continue;
            }

            // A new child of a local work must not take a sequence number a
            // local child already has; that would disturb the local order.
            if let (Some(parent), Some(sequence)) =
                (work.parent_work_id.as_deref(), work.sequence_number)
            {
                if destination.works.contains(parent) {
                    if destination
                        .child_positions
                        .contains(&(parent.to_owned(), sequence))
                    {
                        return Err(conflict(format!(
                            "Work {} would take the position of an existing part of work {parent}",
                            work.work_id
                        )));
                    }
                } else if !child_positions.insert((parent.to_owned(), sequence)) {
                    return Err(conflict(format!(
                        "The imported works under {parent} share the position {sequence}"
                    )));
                }
            }

            works_in_order.push(work.work_id.clone());
        }

        // Parents before children: children are inserted after their imported
        // parent, and new children of local parents are inserted last.
        let depth = depth_of_works(&self.works);
        works_in_order.sort_by(|a, b| {
            depth
                .get(a)
                .unwrap_or(&usize::MAX)
                .cmp(depth.get(b).unwrap_or(&usize::MAX))
                .then_with(|| a.cmp(b))
        });

        Ok(MergePlan {
            skipped_tracks,
            works_in_order,
            report: std::cell::RefCell::new(ImportReport::default()),
        })
    }
}

/// The set of entities one import needs, selected from a source graph.
///
/// A work's closure contains the work itself, its ancestors, its descendants
/// and its directly related works, plus every person, role, instrument and
/// tag those records need. Relation rows are only included when both their
/// source and target are part of the closure.
#[derive(Clone, Debug, Default)]
pub(crate) struct ImportClosure {
    pub(crate) persons: HashSet<String>,
    pub(crate) roles: HashSet<String>,
    pub(crate) instruments: HashSet<String>,
    pub(crate) tags: HashSet<String>,
    pub(crate) works: HashSet<String>,
    pub(crate) ensembles: HashSet<String>,
    pub(crate) recordings: HashSet<String>,
    /// Albums are never part of a metadata selection; reserved for symmetry.
    pub(crate) albums: HashSet<String>,
}

impl ImportGraph {
    /// Select the entities an import of `kind` `id` needs from this graph.
    pub(crate) fn closure_for(&self, kind: EntityKind, id: &str) -> Result<ImportClosure> {
        let mut closure = ImportClosure::default();

        match kind {
            EntityKind::Person => {
                self.require_present_for_closure(EntityKind::Person, id)?;
                closure.persons.insert(id.to_owned());
            }
            EntityKind::Role => {
                self.require_present_for_closure(EntityKind::Role, id)?;
                closure.roles.insert(id.to_owned());
            }
            EntityKind::Instrument => {
                self.require_present_for_closure(EntityKind::Instrument, id)?;
                closure.instruments.insert(id.to_owned());
            }
            EntityKind::Tag => {
                self.require_present_for_closure(EntityKind::Tag, id)?;
                closure.tags.insert(id.to_owned());
            }
            EntityKind::Ensemble => self.closure_ensemble(&mut closure, id)?,
            EntityKind::Work => self.closure_work(&mut closure, id)?,
            EntityKind::Recording => self.closure_recording(&mut closure, id)?,
            // Albums and tracks cannot be selected from the metadata
            // catalogue; they only exist in user libraries.
            EntityKind::Album | EntityKind::Track => {
                return Err(LibraryError::InvalidInput(format!(
                    "Cannot import a {kind} from the metadata catalogue"
                )))
            }
        }

        Ok(closure)
    }

    /// Include an ensemble and everything its record needs: its members with
    /// their people, roles and instruments.
    pub(crate) fn closure_ensemble(&self, closure: &mut ImportClosure, id: &str) -> Result<()> {
        self.require_present_for_closure(EntityKind::Ensemble, id)?;
        closure.ensembles.insert(id.to_owned());

        for member in self.ensemble_persons_of(id) {
            closure.persons.insert(member.person_id.clone());
            if let Some(role_id) = &member.role_id {
                closure.roles.insert(role_id.clone());
            }
            if let Some(instrument_id) = &member.instrument_id {
                closure.instruments.insert(instrument_id.clone());
            }
        }

        Ok(())
    }

    /// Include a recording and everything its record needs: its work tree,
    /// performing persons and ensembles (with the ensembles' members) and its
    /// tags.
    pub(crate) fn closure_recording(&self, closure: &mut ImportClosure, id: &str) -> Result<()> {
        self.require_present_for_closure(EntityKind::Recording, id)?;
        closure.recordings.insert(id.to_owned());

        let recording = self
            .recordings
            .get(id)
            .ok_or_else(|| LibraryError::NotFound(EntityKind::Recording, id.to_owned()))?;

        self.closure_work(closure, &recording.work_id)?;

        for person in self.recording_persons_of(id) {
            closure.persons.insert(person.person_id.clone());
            if let Some(role_id) = &person.role_id {
                closure.roles.insert(role_id.clone());
            }
            if let Some(instrument_id) = &person.instrument_id {
                closure.instruments.insert(instrument_id.clone());
            }
        }

        for ensemble in self.recording_ensembles_of(id) {
            self.closure_ensemble(closure, &ensemble.ensemble_id)?;
            if let Some(role_id) = &ensemble.role_id {
                closure.roles.insert(role_id.clone());
            }
        }

        for tag in self.recording_tags_of(id) {
            closure.tags.insert(tag.tag_id.clone());
        }

        Ok(())
    }

    /// Include a work with its complete tree: the work itself, its ancestors,
    /// its descendants and its directly related works, plus every credit,
    /// instrument and tag those records need.
    ///
    /// Traversal is cycle-safe: relation graphs may contain cycles, and the
    /// `visited` set terminates them. The parent hierarchy itself is acyclic
    /// (validated when the graph was loaded), so ancestors terminate.
    pub(crate) fn closure_work(&self, closure: &mut ImportClosure, id: &str) -> Result<()> {
        self.require_present_for_closure(EntityKind::Work, id)?;

        let mut queue = std::collections::VecDeque::new();
        queue.push_back(id.to_owned());
        let mut visited = HashSet::new();

        // Works to include: the root, its ancestors, its descendants and its
        // directly related works.
        while let Some(work_id) = queue.pop_front() {
            if !visited.insert(work_id.clone()) {
                continue;
            }

            closure.works.insert(work_id.clone());

            let work = self
                .works
                .get(&work_id)
                .ok_or_else(|| LibraryError::NotFound(EntityKind::Work, work_id.clone()))?;

            if let Some(parent) = &work.parent_work_id {
                queue.push_back(parent.clone());
            }

            if let Some(children) = self.children.get(&work_id) {
                queue.extend(children.iter().cloned());
            }

            for relation in self.work_relations_of(&work_id) {
                queue.push_back(relation.related_work_id.clone());
            }
        }

        // Every included work brings the rows that make its record complete:
        // credits, instruments and tags, with the referenced people, roles and
        // instruments themselves.
        for work_id in &closure.works {
            for work_person in self.work_persons_of(work_id) {
                closure.persons.insert(work_person.person_id.clone());
                if let Some(role_id) = &work_person.role_id {
                    closure.roles.insert(role_id.clone());
                }
            }

            for instrument in self.work_instruments_of(work_id) {
                closure.instruments.insert(instrument.instrument_id.clone());
            }

            for tag in self.work_tags_of(work_id) {
                closure.tags.insert(tag.tag_id.clone());
            }
        }

        Ok(())
    }

    fn require_present_for_closure(&self, kind: EntityKind, id: &str) -> Result<()> {
        let present = match kind {
            EntityKind::Person => self.persons.contains_key(id),
            EntityKind::Role => self.roles.contains_key(id),
            EntityKind::Instrument => self.instruments.contains_key(id),
            EntityKind::Tag => self.tags.contains_key(id),
            EntityKind::Work => self.works.contains_key(id),
            EntityKind::Ensemble => self.ensembles.contains_key(id),
            EntityKind::Recording => self.recordings.contains_key(id),
            EntityKind::Album => self.albums.contains_key(id),
            EntityKind::Track => self.tracks.contains_key(id),
        };

        if present {
            Ok(())
        } else {
            Err(LibraryError::NotFound(kind, id.to_owned()))
        }
    }
}

/// Apply a planned local-wins import of `graph` to `connection`.
///
/// The caller must be inside a transaction. `plan` and `destination` describe
/// the decisions made beforehand; `closure` restricts the import to a selected
/// set of entities, `None` imports the whole graph. Rows are written in
/// foreign-key-safe order (works parents first) and every colliding local
/// entity is left entirely unchanged.
pub(crate) fn apply_import(
    connection: &mut SqliteConnection,
    graph: &ImportGraph,
    plan: &MergePlan,
    destination: &DestinationState,
    closure: Option<&ImportClosure>,
    source: Source,
    import_tracks: bool,
) -> Result<()> {
    let now = db::now();
    let mut report = plan.report.borrow_mut();

    for row in graph.persons.values() {
        if !closure.is_none_or(|closure| closure.persons.contains(&row.person_id)) {
            continue;
        }
        if destination.persons.contains(&row.person_id) {
            report.reused_local += 1;
            continue;
        }

        let mut row = row.clone();
        row.source = source;
        row.created_at = now;
        row.edited_at = now;
        row.last_used_at = now;

        diesel::insert_into(persons::table)
            .values(&row)
            .execute(connection)?;

        report.inserted += 1;
    }

    for row in graph.roles.values() {
        if !closure.is_none_or(|closure| closure.roles.contains(&row.role_id)) {
            continue;
        }
        if destination.roles.contains(&row.role_id) {
            report.reused_local += 1;
            continue;
        }

        let mut row = row.clone();
        row.source = source;
        row.created_at = now;
        row.edited_at = now;
        row.last_used_at = now;

        diesel::insert_into(roles::table)
            .values(&row)
            .execute(connection)?;

        report.inserted += 1;
    }

    for row in graph.tags.values() {
        if !closure.is_none_or(|closure| closure.tags.contains(&row.tag_id)) {
            continue;
        }
        if destination.tags.contains(&row.tag_id) {
            report.reused_local += 1;
            continue;
        }

        let mut row = row.clone();
        row.source = source;
        row.created_at = now;
        row.edited_at = now;
        row.last_used_at = now;

        diesel::insert_into(tags::table)
            .values(&row)
            .execute(connection)?;

        report.inserted += 1;
    }

    for row in graph.instruments.values() {
        if !closure.is_none_or(|closure| closure.instruments.contains(&row.instrument_id)) {
            continue;
        }
        if destination.instruments.contains(&row.instrument_id) {
            report.reused_local += 1;
            continue;
        }

        let mut row = row.clone();
        row.source = source;
        row.created_at = now;
        row.edited_at = now;
        row.last_used_at = now;

        diesel::insert_into(instruments::table)
            .values(&row)
            .execute(connection)?;

        report.inserted += 1;
    }

    // Works are inserted parents first, so that a child never precedes the
    // work it belongs to. Reused local works and their owned rows are left
    // alone.
    for work_id in &plan.works_in_order {
        let work = graph.works.get(work_id).ok_or_else(|| {
            LibraryError::Corrupt(format!(
                "The import plan references the missing work {work_id}"
            ))
        })?;

        if destination.works.contains(work_id) {
            report.reused_local += 1;
            report.skipped += graph.work_persons_of(work_id).len()
                + graph.work_instruments_of(work_id).len()
                + graph.work_tags_of(work_id).len()
                + graph.work_relations_of(work_id).len();
            continue;
        }

        let mut row = work.clone();
        row.source = source;
        row.created_at = now;
        row.edited_at = now;
        row.last_used_at = now;

        diesel::insert_into(works::table)
            .values(&row)
            .execute(connection)?;

        report.inserted += 1;

        for work_person in graph.work_persons_of(work_id) {
            diesel::insert_into(work_persons::table)
                .values(work_person.clone())
                .execute(connection)?;
            report.inserted += 1;
        }

        for work_instrument in graph.work_instruments_of(work_id) {
            diesel::insert_into(work_instruments::table)
                .values(work_instrument.clone())
                .execute(connection)?;
            report.inserted += 1;
        }

        for work_tag in graph.work_tags_of(work_id) {
            diesel::insert_into(work_tags::table)
                .values(work_tag.clone())
                .execute(connection)?;
            report.inserted += 1;
        }

        for relation in graph.work_relations_of(work_id) {
            diesel::insert_into(work_relations::table)
                .values(relation.clone())
                .execute(connection)?;
            report.inserted += 1;
        }
    }

    for row in graph.ensembles.values() {
        if !closure.is_none_or(|closure| closure.ensembles.contains(&row.ensemble_id)) {
            continue;
        }
        if destination.ensembles.contains(&row.ensemble_id) {
            report.reused_local += 1;
            report.skipped += graph.ensemble_persons_of(&row.ensemble_id).len();
            continue;
        }

        let mut row = row.clone();
        row.source = source;
        row.created_at = now;
        row.edited_at = now;
        row.last_used_at = now;

        diesel::insert_into(ensembles::table)
            .values(&row)
            .execute(connection)?;

        report.inserted += 1;

        for ensemble_person in graph.ensemble_persons_of(&row.ensemble_id) {
            diesel::insert_into(ensemble_persons::table)
                .values(ensemble_person.clone())
                .execute(connection)?;
            report.inserted += 1;
        }
    }

    for row in graph.recordings.values() {
        if !closure.is_none_or(|closure| closure.recordings.contains(&row.recording_id)) {
            continue;
        }
        if destination.recordings.contains(&row.recording_id) {
            report.reused_local += 1;
            report.skipped += graph.recording_persons_of(&row.recording_id).len()
                + graph.recording_ensembles_of(&row.recording_id).len()
                + graph.recording_tags_of(&row.recording_id).len();
            continue;
        }

        let mut row = row.clone();
        row.source = source;
        row.created_at = now;
        row.edited_at = now;
        row.last_used_at = now;

        diesel::insert_into(recordings::table)
            .values(&row)
            .execute(connection)?;

        report.inserted += 1;

        for recording_person in graph.recording_persons_of(&row.recording_id) {
            diesel::insert_into(recording_persons::table)
                .values(recording_person.clone())
                .execute(connection)?;
            report.inserted += 1;
        }

        for recording_ensemble in graph.recording_ensembles_of(&row.recording_id) {
            diesel::insert_into(recording_ensembles::table)
                .values(recording_ensemble.clone())
                .execute(connection)?;
            report.inserted += 1;
        }

        for recording_tag in graph.recording_tags_of(&row.recording_id) {
            diesel::insert_into(recording_tags::table)
                .values(recording_tag.clone())
                .execute(connection)?;
            report.inserted += 1;
        }
    }

    if import_tracks && closure.is_none() {
        for track in graph.tracks.values() {
            if plan.skipped_tracks.contains(&track.track_id) {
                report.skipped += 1 + graph.track_works_of(&track.track_id).len();
                continue;
            }

            let mut row = track.clone();
            row.created_at = now;
            row.edited_at = now;
            row.last_used_at = now;

            diesel::insert_into(tracks::table)
                .values(&row)
                .execute(connection)?;

            report.inserted += 1;

            for track_work in graph.track_works_of(&track.track_id) {
                diesel::insert_into(track_works::table)
                    .values(track_work.clone())
                    .execute(connection)?;
                report.inserted += 1;
            }
        }
    }

    for row in graph.albums.values() {
        // Albums are never part of a metadata selection; only whole-graph
        // imports (archives) can bring them.
        if closure.is_some() {
            break;
        }
        if destination.albums.contains(&row.album_id) {
            report.reused_local += 1;
            report.skipped += graph.album_recordings_of(&row.album_id).len();
            continue;
        }

        let mut row = row.clone();
        row.source = source;
        row.created_at = now;
        row.edited_at = now;
        row.last_used_at = now;

        diesel::insert_into(albums::table)
            .values(&row)
            .execute(connection)?;

        report.inserted += 1;

        for album_recording in graph.album_recordings_of(&row.album_id) {
            diesel::insert_into(album_recordings::table)
                .values(album_recording.clone())
                .execute(connection)?;
            report.inserted += 1;
        }
    }

    Ok(())
}

/// How far each work is from the root of its hierarchy, for parent-before-
/// child insertion order. The hierarchy is validated acyclic, so this
/// terminates.
fn depth_of_works(works: &HashMap<String, tables::Work>) -> HashMap<String, usize> {
    fn depth(
        id: &str,
        works: &HashMap<String, tables::Work>,
        memo: &mut HashMap<String, usize>,
    ) -> usize {
        if let Some(&depth) = memo.get(id) {
            return depth;
        }

        let depth = match works
            .get(id)
            .and_then(|work| work.parent_work_id.as_deref())
        {
            Some(parent) => 1 + depth(parent, works, memo),
            None => 0,
        };

        memo.insert(id.to_owned(), depth);
        depth
    }

    let mut memo = HashMap::new();
    for id in works.keys() {
        depth(id, works, &mut memo);
    }
    memo
}

fn require_present<Row>(
    rows: &HashMap<String, Row>,
    owner_kind: EntityKind,
    owner_id: &str,
    kind: EntityKind,
    id: &str,
) -> Result<()> {
    if rows.contains_key(id) {
        Ok(())
    } else {
        Err(corrupt(format!(
            "{owner_kind} {owner_id} refers to the missing {kind} {id}"
        )))
    }
}

fn check_sequence(owner_kind: EntityKind, owner_id: &str, sequence: i32) -> Result<()> {
    if sequence < 0 {
        Err(corrupt(format!(
            "{owner_kind} {owner_id} has the negative sequence number {sequence}"
        )))
    } else {
        Ok(())
    }
}

fn conflict(message: impl Into<String>) -> LibraryError {
    LibraryError::ImportConflict(message.into())
}

fn corrupt(message: impl Into<String>) -> LibraryError {
    LibraryError::Corrupt(message.into())
}

fn keyed_by<T, K: std::hash::Hash + Eq>(rows: Vec<T>, key: impl Fn(&T) -> K) -> HashMap<K, T> {
    rows.into_iter()
        .map(|row| (key(&row), row))
        .collect::<HashMap<K, T>>()
}

fn grouped_by<T, K: std::hash::Hash + Eq>(
    rows: Vec<T>,
    key: impl Fn(&T) -> K,
) -> HashMap<K, Vec<T>> {
    let mut groups: HashMap<K, Vec<T>> = HashMap::new();
    for row in rows {
        groups.entry(key(&row)).or_default().push(row);
    }
    groups
}
