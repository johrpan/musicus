//! Assemble the supported domain models from database rows.
//!
//! All loading goes through [`Loader`], which reads the tables once into
//! memory and indexes the rows by owner, so that an aggregate (a work tree, a
//! recording, an album) never triggers a query per child.

use std::collections::{HashMap, HashSet};

use anyhow::Result;
use diesel::prelude::*;

use crate::{
    db::{schema::*, tables},
    error::{EntityKind, LibraryError},
    models::*,
};

/// Everything needed to build domain models, loaded once and indexed.
pub(crate) struct Loader {
    persons: HashMap<String, tables::Person>,
    roles: HashMap<String, tables::Role>,
    instruments: HashMap<String, tables::Instrument>,
    tags: HashMap<String, tables::Tag>,
    works: HashMap<String, tables::Work>,
    work_persons: HashMap<String, Vec<tables::WorkPerson>>,
    work_instruments: HashMap<String, Vec<tables::WorkInstrument>>,
    work_tags: HashMap<String, Vec<tables::WorkTag>>,
    work_relations: HashMap<String, Vec<tables::WorkRelation>>,
    /// Child work IDs per parent, in their stored order.
    children: HashMap<String, Vec<String>>,
    ensembles: HashMap<String, tables::Ensemble>,
    ensemble_persons: HashMap<String, Vec<tables::EnsemblePerson>>,
    recordings: HashMap<String, tables::Recording>,
    recording_persons: HashMap<String, Vec<tables::RecordingPerson>>,
    recording_ensembles: HashMap<String, Vec<tables::RecordingEnsemble>>,
    recording_tags: HashMap<String, Vec<tables::RecordingTag>>,
    albums: HashMap<String, tables::Album>,
    album_recordings: HashMap<String, Vec<tables::AlbumRecording>>,
    tracks: HashMap<String, tables::Track>,
    track_works: HashMap<String, Vec<tables::TrackWork>>,
}

impl Loader {
    /// Read every table the domain models need and index the rows.
    pub(crate) fn load(connection: &mut SqliteConnection) -> Result<Self> {
        let works = works::table.load::<tables::Work>(connection)?;
        let work_persons = work_persons::table.load::<tables::WorkPerson>(connection)?;
        let work_instruments =
            work_instruments::table.load::<tables::WorkInstrument>(connection)?;
        let work_tags = work_tags::table.load::<tables::WorkTag>(connection)?;
        let work_relations = work_relations::table.load::<tables::WorkRelation>(connection)?;

        let mut children: HashMap<String, Vec<String>> = HashMap::new();
        for work in &works {
            if let Some(parent) = &work.parent_work_id {
                children
                    .entry(parent.clone())
                    .or_default()
                    .push(work.work_id.clone());
            }
        }
        for ids in children.values_mut() {
            ids.sort();
        }

        Ok(Self {
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
            work_persons: grouped_by(work_persons, |row| row.work_id.clone()),
            work_instruments: grouped_by(work_instruments, |row| row.work_id.clone()),
            work_tags: grouped_by(work_tags, |row| row.work_id.clone()),
            work_relations: grouped_by(work_relations, |row| row.work_id.clone()),
            children,
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
            albums: keyed_by(albums::table.load::<tables::Album>(connection)?, |row| {
                row.album_id.clone()
            }),
            album_recordings: grouped_by(
                album_recordings::table.load::<tables::AlbumRecording>(connection)?,
                |row| row.album_id.clone(),
            ),
            tracks: keyed_by(tracks::table.load::<tables::Track>(connection)?, |row| {
                row.track_id.clone()
            }),
            track_works: grouped_by(
                track_works::table.load::<tables::TrackWork>(connection)?,
                |row| row.track_id.clone(),
            ),
        })
    }

    /// The work identified by `id`, with its parts loaded recursively.
    pub(crate) fn work(&self, id: &str) -> Result<Work> {
        self.work_with_cycle_guard(id, &mut HashSet::new())
    }

    fn work_with_cycle_guard(&self, id: &str, active: &mut HashSet<String>) -> Result<Work> {
        if !active.insert(id.to_owned()) {
            return Err(
                LibraryError::Corrupt(format!("Work {id} is part of a parent cycle")).into(),
            );
        }

        let row = self
            .works
            .get(id)
            .cloned()
            .ok_or_else(|| not_found(EntityKind::Work, id))?;

        let mut parts = Vec::new();
        if let Some(child_ids) = self.children.get(id) {
            for child_id in child_ids {
                parts.push(self.work_with_cycle_guard(child_id, active)?);
            }
        }

        let mut related_works = Vec::new();
        if let Some(relations) = self.work_relations.get(id) {
            let mut relations = relations.clone();
            relations.sort_by(|a, b| a.related_work_id.cmp(&b.related_work_id));

            for relation in relations {
                let related = self
                    .works
                    .get(&relation.related_work_id)
                    .cloned()
                    .ok_or_else(|| not_found(EntityKind::Work, &relation.related_work_id))?;

                related_works.push(RelatedWork {
                    id: related.work_id,
                    name: related.name,
                    work_persons: self.work_persons_of(&relation.related_work_id)?,
                });
            }
        }

        active.remove(id);

        Ok(Work {
            id: row.work_id,
            name: row.name,
            source: row.source,
            last_used_at: row.last_used_at,
            enable_updates: row.enable_updates,
            parts,
            work_persons: self.work_persons_of(id)?,
            instruments: self.instruments_of(id)?,
            tags: self.tags_of(id)?,
            related_works,
        })
    }

    fn work_persons_of(&self, work_id: &str) -> Result<Vec<WorkPerson>> {
        let mut rows = self.work_persons.get(work_id).cloned().unwrap_or_default();
        rows.sort_by(|a, b| a.sequence_number.cmp(&b.sequence_number));

        rows.into_iter()
            .map(|row| {
                Ok(WorkPerson {
                    person: self.person(&row.person_id)?,
                    role: row
                        .role_id
                        .as_deref()
                        .map(|role_id| self.role(role_id))
                        .transpose()?,
                })
            })
            .collect()
    }

    fn instruments_of(&self, work_id: &str) -> Result<Vec<Instrument>> {
        let mut rows = self
            .work_instruments
            .get(work_id)
            .cloned()
            .unwrap_or_default();
        rows.sort_by(|a, b| a.sequence_number.cmp(&b.sequence_number));

        rows.into_iter()
            .map(|row| self.instrument(&row.instrument_id))
            .collect()
    }

    fn tags_of(&self, work_id: &str) -> Result<Vec<TagValue>> {
        let mut rows = self.work_tags.get(work_id).cloned().unwrap_or_default();
        rows.sort_by(|a, b| a.sequence_number.cmp(&b.sequence_number));

        rows.into_iter()
            .map(|row| {
                Ok(TagValue {
                    tag: self.tag(&row.tag_id)?,
                    value: row.value,
                })
            })
            .collect()
    }

    /// The ensemble identified by `id`, with its members.
    pub(crate) fn ensemble(&self, id: &str) -> Result<Ensemble> {
        let row = self
            .ensembles
            .get(id)
            .cloned()
            .ok_or_else(|| not_found(EntityKind::Ensemble, id))?;

        let mut members = Vec::new();
        if let Some(member_rows) = self.ensemble_persons.get(id) {
            let mut member_rows = member_rows.clone();
            member_rows.sort_by(|a, b| a.sequence_number.cmp(&b.sequence_number));

            for member in member_rows {
                members.push(RecordingPerson {
                    person: self.person(&member.person_id)?,
                    role: member
                        .role_id
                        .as_deref()
                        .map(|role_id| self.role(role_id))
                        .transpose()?,
                    instrument: member
                        .instrument_id
                        .as_deref()
                        .map(|instrument_id| self.instrument(instrument_id))
                        .transpose()?,
                });
            }
        }

        Ok(Ensemble {
            id: row.ensemble_id,
            name: row.name,
            source: row.source,
            last_used_at: row.last_used_at,
            enable_updates: row.enable_updates,
            members,
        })
    }

    /// The recording identified by `id`, with its work tree and credits.
    pub(crate) fn recording(&self, id: &str) -> Result<Recording> {
        let row = self
            .recordings
            .get(id)
            .cloned()
            .ok_or_else(|| not_found(EntityKind::Recording, id))?;

        Ok(Recording {
            id: row.recording_id,
            work: self.work(&row.work_id)?,
            source: row.source,
            last_used_at: row.last_used_at,
            enable_updates: row.enable_updates,
            recording_persons: self.recording_persons_of(id)?,
            recording_ensembles: self.recording_ensembles_of(id)?,
            tags: self.recording_tags_of(id)?,
            comment: row.comment,
        })
    }

    fn recording_persons_of(&self, recording_id: &str) -> Result<Vec<RecordingPerson>> {
        let mut rows = self
            .recording_persons
            .get(recording_id)
            .cloned()
            .unwrap_or_default();
        rows.sort_by(|a, b| a.sequence_number.cmp(&b.sequence_number));

        rows.into_iter()
            .map(|row| {
                Ok(RecordingPerson {
                    person: self.person(&row.person_id)?,
                    role: row
                        .role_id
                        .as_deref()
                        .map(|role_id| self.role(role_id))
                        .transpose()?,
                    instrument: row
                        .instrument_id
                        .as_deref()
                        .map(|instrument_id| self.instrument(instrument_id))
                        .transpose()?,
                })
            })
            .collect()
    }

    fn recording_ensembles_of(&self, recording_id: &str) -> Result<Vec<RecordingEnsemble>> {
        let mut rows = self
            .recording_ensembles
            .get(recording_id)
            .cloned()
            .unwrap_or_default();
        rows.sort_by(|a, b| a.sequence_number.cmp(&b.sequence_number));

        rows.into_iter()
            .map(|row| {
                Ok(RecordingEnsemble {
                    ensemble: self.ensemble(&row.ensemble_id)?,
                    role: row
                        .role_id
                        .as_deref()
                        .map(|role_id| self.role(role_id))
                        .transpose()?,
                })
            })
            .collect()
    }

    fn recording_tags_of(&self, recording_id: &str) -> Result<Vec<TagValue>> {
        let mut rows = self
            .recording_tags
            .get(recording_id)
            .cloned()
            .unwrap_or_default();
        rows.sort_by(|a, b| a.sequence_number.cmp(&b.sequence_number));

        rows.into_iter()
            .map(|row| {
                Ok(TagValue {
                    tag: self.tag(&row.tag_id)?,
                    value: row.value,
                })
            })
            .collect()
    }

    /// Build a track model from a loaded row.
    fn track_from_row(&self, row: tables::Track) -> Result<Track> {
        let mut works = Vec::new();
        if let Some(work_rows) = self.track_works.get(&row.track_id) {
            let mut work_rows = work_rows.clone();
            work_rows.sort_by(|a, b| a.sequence_number.cmp(&b.sequence_number));

            for work_row in work_rows {
                works.push(self.work(&work_row.work_id)?);
            }
        }

        Ok(Track {
            id: row.track_id,
            recording_id: row.recording_id,
            recording_index: row.recording_index,
            path: row.path,
            works,
        })
    }

    /// The tracks of a recording, in their stored order.
    pub(crate) fn tracks_of_recording(&self, recording_id: &str) -> Result<Vec<Track>> {
        let mut rows = self
            .tracks
            .values()
            .filter(|track| track.recording_id == recording_id)
            .cloned()
            .collect::<Vec<_>>();
        rows.sort_by(|a, b| a.recording_index.cmp(&b.recording_index));

        rows.into_iter()
            .map(|row| self.track_from_row(row))
            .collect()
    }

    /// The album identified by `id`, with its recordings in order.
    pub(crate) fn album(&self, id: &str) -> Result<Album> {
        let row = self
            .albums
            .get(id)
            .cloned()
            .ok_or_else(|| not_found(EntityKind::Album, id))?;

        let mut recordings = Vec::new();
        if let Some(recording_rows) = self.album_recordings.get(id) {
            let mut recording_rows = recording_rows.clone();
            recording_rows.sort_by(|a, b| a.sequence_number.cmp(&b.sequence_number));

            for recording_row in recording_rows {
                recordings.push(self.recording(&recording_row.recording_id)?);
            }
        }

        Ok(Album {
            id: row.album_id,
            name: row.name,
            source: row.source,
            last_used_at: row.last_used_at,
            enable_updates: row.enable_updates,
            recordings,
        })
    }

    pub(crate) fn person(&self, id: &str) -> Result<Person> {
        match self.persons.get(id) {
            Some(row) => Ok(Person::from(row.clone())),
            None => Err(not_found(EntityKind::Person, id).into()),
        }
    }

    pub(crate) fn role(&self, id: &str) -> Result<Role> {
        match self.roles.get(id) {
            Some(row) => Ok(Role::from(row.clone())),
            None => Err(not_found(EntityKind::Role, id).into()),
        }
    }

    pub(crate) fn instrument(&self, id: &str) -> Result<Instrument> {
        match self.instruments.get(id) {
            Some(row) => Ok(Instrument::from(row.clone())),
            None => Err(not_found(EntityKind::Instrument, id).into()),
        }
    }

    pub(crate) fn tag(&self, id: &str) -> Result<Tag> {
        match self.tags.get(id) {
            Some(row) => Ok(Tag::from(row.clone())),
            None => Err(not_found(EntityKind::Tag, id).into()),
        }
    }
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

fn not_found(kind: EntityKind, id: &str) -> LibraryError {
    LibraryError::NotFound(kind, id.to_owned())
}
