//! List library contents.
//!
//! This module contains code for listing library content. This is separate
//! from search (which includes the metadata database) and query (which is for
//! item discovery).

use std::collections::HashMap;

use anyhow::Result;
use diesel::{prelude::*, sql_query, sql_types, SqliteConnection};

use super::Library;
use crate::{
    db::{schema::*, tables, TranslatedString},
    library::load::Loader,
    models::*,
};

#[derive(QueryableByName)]
struct WorkPersonRow {
    #[diesel(sql_type = sql_types::Text)]
    work_id: String,
    #[diesel(sql_type = sql_types::Text)]
    name: TranslatedString,
}

#[derive(Clone, Debug)]
pub struct WorkListItem {
    pub work: Work,
    pub work_persons: Vec<TranslatedString>,
    pub tags: Vec<(TranslatedString, Option<TranslatedString>)>,
}

#[derive(Clone, Debug)]
pub struct RecordingListItem {
    pub recording: Recording,
    pub work_name: TranslatedString,
    pub work_persons: Vec<TranslatedString>,
    pub recording_persons: Vec<TranslatedString>,
    pub n_tracks: i64,
    pub tags: Vec<(TranslatedString, Option<TranslatedString>)>,
}

/// The work persons credited on each work, keyed by work ID, in their stored
/// order.
///
/// A work with no credits of its own still gets an entry here whenever an
/// ancestor or descendant of it does (e.g. movements that inherit their
/// composer from the parent work, or a parent work only credited through a
/// single movement) — the same "ancestor or descendant" rule `query.rs`'s
/// search uses, walking `parent_work_id` rather than anchoring on the exact
/// work row.
fn work_persons_by_work(
    connection: &mut SqliteConnection,
) -> Result<HashMap<String, Vec<TranslatedString>>> {
    let mut work_persons: HashMap<String, Vec<TranslatedString>> = HashMap::new();

    for row in sql_query(
        "WITH RECURSIVE \
            ancestor_closure(work_id, related_id) AS ( \
                SELECT work_id, work_id FROM works \
                UNION \
                SELECT ancestor_closure.work_id, works.parent_work_id \
                FROM ancestor_closure \
                JOIN works ON works.work_id = ancestor_closure.related_id \
                WHERE works.parent_work_id IS NOT NULL \
            ), \
            descendant_closure(work_id, related_id) AS ( \
                SELECT work_id, work_id FROM works \
                UNION \
                SELECT descendant_closure.work_id, works.work_id \
                FROM descendant_closure \
                JOIN works ON works.parent_work_id = descendant_closure.related_id \
            ), \
            closure(work_id, related_id) AS ( \
                SELECT work_id, related_id FROM ancestor_closure \
                UNION \
                SELECT work_id, related_id FROM descendant_closure \
            ) \
        SELECT closure.work_id AS work_id, persons.name AS name \
        FROM closure \
        JOIN work_persons ON work_persons.work_id = closure.related_id \
        JOIN persons ON persons.person_id = work_persons.person_id \
        ORDER BY closure.work_id, (closure.related_id <> closure.work_id), \
            work_persons.sequence_number",
    )
    .load::<WorkPersonRow>(connection)?
    {
        work_persons.entry(row.work_id).or_default().push(row.name);
    }

    Ok(work_persons)
}

/// The tags applied to each work, keyed by work ID, in their stored order.
fn tags_by_work(
    connection: &mut SqliteConnection,
) -> Result<HashMap<String, Vec<(TranslatedString, Option<TranslatedString>)>>> {
    let mut tags: HashMap<String, Vec<(TranslatedString, Option<TranslatedString>)>> =
        HashMap::new();

    for (work_id, name, value) in work_tags::table
        .inner_join(tags::table)
        .order((work_tags::work_id, work_tags::sequence_number))
        .select((work_tags::work_id, tags::name, work_tags::value))
        .load::<(String, TranslatedString, Option<TranslatedString>)>(connection)?
    {
        tags.entry(work_id).or_default().push((name, value));
    }

    Ok(tags)
}

/// The tags applied to each recording, keyed by recording ID, in their stored order.
fn tags_by_recording(
    connection: &mut SqliteConnection,
) -> Result<HashMap<String, Vec<(TranslatedString, Option<TranslatedString>)>>> {
    let mut tags: HashMap<String, Vec<(TranslatedString, Option<TranslatedString>)>> =
        HashMap::new();

    for (recording_id, name, value) in recording_tags::table
        .inner_join(tags::table)
        .order((
            recording_tags::recording_id,
            recording_tags::sequence_number,
        ))
        .select((
            recording_tags::recording_id,
            tags::name,
            recording_tags::value,
        ))
        .load::<(String, TranslatedString, Option<TranslatedString>)>(connection)?
    {
        tags.entry(recording_id).or_default().push((name, value));
    }

    Ok(tags)
}

impl Library {
    pub fn list_persons(&self) -> Result<Vec<Person>> {
        let connection = &mut *self.conn();
        Ok(persons::table
            .select(tables::Person::as_select())
            .load::<tables::Person>(connection)?
            .into_iter()
            .map(Person::from)
            .collect())
    }

    pub fn list_roles(&self) -> Result<Vec<Role>> {
        let connection = &mut *self.conn();
        Ok(roles::table
            .select(tables::Role::as_select())
            .load::<tables::Role>(connection)?
            .into_iter()
            .map(Role::from)
            .collect())
    }

    pub fn list_instruments(&self) -> Result<Vec<Instrument>> {
        let connection = &mut *self.conn();
        Ok(instruments::table
            .select(tables::Instrument::as_select())
            .load::<tables::Instrument>(connection)?
            .into_iter()
            .map(Instrument::from)
            .collect())
    }

    pub fn list_tags(&self) -> Result<Vec<Tag>> {
        let connection = &mut *self.conn();
        Ok(tags::table
            .select(tables::Tag::as_select())
            .load::<tables::Tag>(connection)?
            .into_iter()
            .map(Tag::from)
            .collect())
    }

    pub fn list_albums(&self) -> Result<Vec<Album>> {
        let connection = &mut *self.conn();

        let loader = Loader::load(connection)?;

        let album_rows = albums::table
            .select(tables::Album::as_select())
            .load::<tables::Album>(connection)?;

        album_rows
            .into_iter()
            .map(|row| loader.album(&row.album_id))
            .collect()
    }

    pub fn list_ensembles(&self) -> Result<Vec<Ensemble>> {
        let connection = &mut *self.conn();

        let loader = Loader::load(connection)?;

        let ensembles = ensembles::table
            .select(tables::Ensemble::as_select())
            .load::<tables::Ensemble>(connection)?;

        ensembles
            .into_iter()
            .map(|row| loader.ensemble(&row.ensemble_id))
            .collect()
    }

    /// List all works.
    pub fn list_works(&self) -> Result<Vec<WorkListItem>> {
        let connection = &mut *self.conn();

        let loader = Loader::load(connection)?;

        let works = works::table
            .filter(works::parent_work_id.is_null())
            .select(tables::Work::as_select())
            .load::<tables::Work>(connection)?;

        let mut work_persons = work_persons_by_work(connection)?;
        let mut tags = tags_by_work(connection)?;

        works
            .into_iter()
            .map(|work| {
                let work = loader.work(&work.work_id)?;

                Ok(WorkListItem {
                    work_persons: work_persons.remove(&work.id).unwrap_or_default(),
                    tags: tags.remove(&work.id).unwrap_or_default(),
                    work,
                })
            })
            .collect()
    }

    /// List recordings, together with the name of the work each is a
    /// recording of.
    ///
    /// The work persons include the recording's own work's credits plus any credited
    /// on an ancestor or descendant of it (see [`work_persons_by_work`]), so a
    /// recording of a single movement still shows the parent work's composer, and
    /// vice versa.
    pub fn list_recordings(&self) -> Result<Vec<RecordingListItem>> {
        let connection = &mut *self.conn();

        let loader = Loader::load(connection)?;

        let recordings = recordings::table
            .inner_join(works::table.on(works::work_id.eq(recordings::work_id)))
            .select((tables::Recording::as_select(), works::name))
            .load::<(tables::Recording, TranslatedString)>(connection)?;

        // One grouped query rather than one per row.
        let mut n_tracks = tracks::table
            .group_by(tracks::recording_id)
            .select((tracks::recording_id, diesel::dsl::count_star()))
            .load::<(String, i64)>(connection)?
            .into_iter()
            .collect::<HashMap<String, i64>>();

        let work_persons = work_persons_by_work(connection)?;
        let mut tags = tags_by_recording(connection)?;

        recordings
            .into_iter()
            .map(|(recording, work_name)| {
                let recording = loader.recording(&recording.recording_id)?;

                let mut recording_persons = recording
                    .recording_persons
                    .iter()
                    .map(|p| p.person.name.clone())
                    .collect::<Vec<TranslatedString>>();

                recording_persons.append(
                    &mut recording
                        .recording_ensembles
                        .iter()
                        .map(|e| e.ensemble.name.clone())
                        .collect::<Vec<TranslatedString>>(),
                );

                Ok(RecordingListItem {
                    n_tracks: n_tracks.remove(&recording.id).unwrap_or(0),
                    work_persons: work_persons
                        .get(&recording.work.id)
                        .cloned()
                        .unwrap_or_default(),
                    recording_persons,
                    tags: tags.remove(&recording.id).unwrap_or_default(),
                    recording,
                    work_name,
                })
            })
            .collect()
    }

    pub fn load_person(&self, person_id: &str) -> Result<Person> {
        let connection = &mut *self.conn();

        Ok(Person::from(
            persons::table
                .filter(persons::person_id.eq(person_id))
                .select(tables::Person::as_select())
                .first::<tables::Person>(connection)?,
        ))
    }

    pub fn load_role(&self, role_id: &str) -> Result<Role> {
        let connection = &mut *self.conn();

        Ok(Role::from(
            roles::table
                .filter(roles::role_id.eq(role_id))
                .select(tables::Role::as_select())
                .first::<tables::Role>(connection)?,
        ))
    }

    pub fn load_instrument(&self, instrument_id: &str) -> Result<Instrument> {
        let connection = &mut *self.conn();

        Ok(Instrument::from(
            instruments::table
                .filter(instruments::instrument_id.eq(instrument_id))
                .select(tables::Instrument::as_select())
                .first::<tables::Instrument>(connection)?,
        ))
    }

    pub fn load_tag(&self, tag_id: &str) -> Result<Tag> {
        let connection = &mut *self.conn();

        Ok(Tag::from(
            tags::table
                .filter(tags::tag_id.eq(tag_id))
                .select(tables::Tag::as_select())
                .first::<tables::Tag>(connection)?,
        ))
    }

    pub fn load_work(&self, work_id: &str) -> Result<Work> {
        let connection = &mut *self.conn();

        let loader = Loader::load(connection)?;
        loader.work(work_id)
    }

    pub fn load_ensemble(&self, ensemble_id: &str) -> Result<Ensemble> {
        let connection = &mut *self.conn();

        let loader = Loader::load(connection)?;
        loader.ensemble(ensemble_id)
    }

    pub fn load_recording(&self, recording_id: &str) -> Result<Recording> {
        let connection = &mut *self.conn();

        let loader = Loader::load(connection)?;
        loader.recording(recording_id)
    }

    pub fn load_album(&self, album_id: &str) -> Result<Album> {
        let connection = &mut *self.conn();

        let loader = Loader::load(connection)?;
        loader.album(album_id)
    }
}
