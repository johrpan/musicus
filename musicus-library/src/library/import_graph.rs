//! A validated snapshot of a source database.
//!
//! Import paths read their source (a `.muslib` archive or the metadata
//! catalogue) through [`ImportGraph`]: every table is loaded once and indexed
//! by ID and owner, and the whole graph is validated before any row is written
//! to a destination. A malformed source is rejected here, before the first
//! mutation, instead of halfway through an import that has already changed the
//! destination.

use std::collections::{HashMap, HashSet};

use anyhow::Result;
use diesel::prelude::*;

use crate::{
    db::{schema::*, tables},
    error::{EntityKind, LibraryError},
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

fn corrupt(message: impl Into<String>) -> anyhow::Error {
    LibraryError::Corrupt(message.into()).into()
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
