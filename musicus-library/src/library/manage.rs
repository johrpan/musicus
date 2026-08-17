//! Library management: listing what is in the library, so that it can be
//! reviewed and worked on in bulk.
//!
//! This is deliberately separate from [`super::query`]. Searching merges in
//! results from the downloaded metadata database and reports them as
//! [`SearchItem`](super::SearchItem)s that are not in the library yet; a manager
//! may only ever act on items that really are in the library, so nothing here
//! ever looks at the metadata database.

use std::collections::HashMap;

use anyhow::Result;
use diesel::{prelude::*, sql_query, sql_types, SqliteConnection};

use super::Library;
use crate::db::{models, schema::*, tables, TranslatedString};

#[derive(QueryableByName)]
struct WorkPersonRow {
    #[diesel(sql_type = sql_types::Text)]
    work_id: String,
    #[diesel(sql_type = sql_types::Text)]
    name: TranslatedString,
}

/// A work together with the persons credited on it.
#[derive(Clone, Debug)]
pub struct WorkListItem {
    pub work: tables::Work,
    /// The names of the work's credited persons, in their stored order.
    pub composers: Vec<TranslatedString>,
}

/// An ensemble together with the persons in it.
#[derive(Clone, Debug)]
pub struct EnsembleListItem {
    pub ensemble: tables::Ensemble,
    /// The names of the ensemble's members, in their stored order.
    pub members: Vec<TranslatedString>,
}

/// A recording has no name of its own, so a list row carries the work it is a
/// recording of, who wrote it, who performed it, and how many tracks it has.
#[derive(Clone, Debug)]
pub struct RecordingListItem {
    pub recording: tables::Recording,
    pub work_name: TranslatedString,
    /// The names of the persons credited on the work.
    pub composers: Vec<TranslatedString>,
    /// The names of the performers, persons and ensembles together, in their
    /// stored order. Which of the two a name came from does not survive; a
    /// listing only ever shows them side by side.
    pub performers: Vec<TranslatedString>,
    pub n_tracks: i64,
}

/// The names credited on each work, keyed by work ID, in their stored order.
///
/// A work with no credits of its own still gets an entry here whenever an ancestor
/// or descendant of it does (e.g. movements that inherit their composer from the
/// parent work, or a parent work only credited through a single movement) — the
/// same "ancestor or descendant" rule `query.rs`'s search uses, walking
/// `parent_work_id` rather than anchoring on the exact work row.
fn composers_by_work(
    connection: &mut SqliteConnection,
) -> Result<HashMap<String, Vec<TranslatedString>>> {
    let mut composers: HashMap<String, Vec<TranslatedString>> = HashMap::new();

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
        composers.entry(row.work_id).or_default().push(row.name);
    }

    Ok(composers)
}

impl Library {
    pub fn list_persons(&self) -> Result<Vec<tables::Person>> {
        let connection = &mut *self.conn();
        Ok(persons::table
            .select(tables::Person::as_select())
            .load(connection)?)
    }

    pub fn list_roles(&self) -> Result<Vec<tables::Role>> {
        let connection = &mut *self.conn();
        Ok(roles::table
            .select(tables::Role::as_select())
            .load(connection)?)
    }

    pub fn list_instruments(&self) -> Result<Vec<tables::Instrument>> {
        let connection = &mut *self.conn();
        Ok(instruments::table
            .select(tables::Instrument::as_select())
            .load(connection)?)
    }

    pub fn list_tags(&self) -> Result<Vec<tables::Tag>> {
        let connection = &mut *self.conn();
        Ok(tags::table
            .select(tables::Tag::as_select())
            .load(connection)?)
    }

    pub fn list_albums(&self) -> Result<Vec<tables::Album>> {
        let connection = &mut *self.conn();
        Ok(albums::table
            .select(tables::Album::as_select())
            .load(connection)?)
    }

    pub fn list_ensembles(&self) -> Result<Vec<EnsembleListItem>> {
        let connection = &mut *self.conn();

        let ensembles = ensembles::table
            .select(tables::Ensemble::as_select())
            .load(connection)?;

        // One grouped query rather than one per row.
        let mut members: HashMap<String, Vec<TranslatedString>> = HashMap::new();

        for (ensemble_id, name) in ensemble_persons::table
            .inner_join(persons::table)
            .order((
                ensemble_persons::ensemble_id,
                ensemble_persons::sequence_number,
            ))
            .select((ensemble_persons::ensemble_id, persons::name))
            .load::<(String, TranslatedString)>(connection)?
        {
            members.entry(ensemble_id).or_default().push(name);
        }

        Ok(ensembles
            .into_iter()
            .map(|ensemble| EnsembleListItem {
                members: members.remove(&ensemble.ensemble_id).unwrap_or_default(),
                ensemble,
            })
            .collect())
    }

    /// List all works.
    pub fn list_works(&self) -> Result<Vec<WorkListItem>> {
        let connection = &mut *self.conn();

        let works = works::table
            .filter(works::parent_work_id.is_null())
            .select(tables::Work::as_select())
            .load(connection)?;

        let mut composers = composers_by_work(connection)?;

        Ok(works
            .into_iter()
            .map(|work| WorkListItem {
                composers: composers.remove(&work.work_id).unwrap_or_default(),
                work,
            })
            .collect())
    }

    /// List recordings, together with the name of the work each is a
    /// recording of.
    ///
    /// The composers include the recording's own work's credits plus any credited
    /// on an ancestor or descendant of it (see [`composers_by_work`]), so a
    /// recording of a single movement still shows the parent work's composer, and
    /// vice versa.
    pub fn list_recordings(&self) -> Result<Vec<RecordingListItem>> {
        let connection = &mut *self.conn();

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

        let composers = composers_by_work(connection)?;

        // Performing persons and ensembles are two tables but one column, so
        // they are gathered together, each keeping its own stored order.
        let mut performers: HashMap<String, Vec<TranslatedString>> = HashMap::new();

        for (recording_id, name) in recording_persons::table
            .inner_join(persons::table)
            .order((
                recording_persons::recording_id,
                recording_persons::sequence_number,
            ))
            .select((recording_persons::recording_id, persons::name))
            .load::<(String, TranslatedString)>(connection)?
        {
            performers.entry(recording_id).or_default().push(name);
        }

        for (recording_id, name) in recording_ensembles::table
            .inner_join(ensembles::table)
            .order((
                recording_ensembles::recording_id,
                recording_ensembles::sequence_number,
            ))
            .select((recording_ensembles::recording_id, ensembles::name))
            .load::<(String, TranslatedString)>(connection)?
        {
            performers.entry(recording_id).or_default().push(name);
        }

        Ok(recordings
            .into_iter()
            .map(|(recording, work_name)| RecordingListItem {
                n_tracks: n_tracks.remove(&recording.recording_id).unwrap_or(0),
                composers: composers
                    .get(&recording.work_id)
                    .cloned()
                    .unwrap_or_default(),
                performers: performers
                    .remove(&recording.recording_id)
                    .unwrap_or_default(),
                recording,
                work_name,
            })
            .collect())
    }

    pub fn load_person(&self, person_id: &str) -> Result<tables::Person> {
        let connection = &mut *self.conn();

        Ok(persons::table
            .filter(persons::person_id.eq(person_id))
            .select(tables::Person::as_select())
            .first(connection)?)
    }

    pub fn load_role(&self, role_id: &str) -> Result<tables::Role> {
        let connection = &mut *self.conn();

        Ok(roles::table
            .filter(roles::role_id.eq(role_id))
            .select(tables::Role::as_select())
            .first(connection)?)
    }

    pub fn load_instrument(&self, instrument_id: &str) -> Result<tables::Instrument> {
        let connection = &mut *self.conn();

        Ok(instruments::table
            .filter(instruments::instrument_id.eq(instrument_id))
            .select(tables::Instrument::as_select())
            .first(connection)?)
    }

    pub fn load_tag(&self, tag_id: &str) -> Result<tables::Tag> {
        let connection = &mut *self.conn();

        Ok(tags::table
            .filter(tags::tag_id.eq(tag_id))
            .select(tables::Tag::as_select())
            .first(connection)?)
    }

    pub fn load_work(&self, work_id: &str) -> Result<models::Work> {
        let connection = &mut *self.conn();

        let data = works::table
            .filter(works::work_id.eq(work_id))
            .first::<tables::Work>(connection)?;

        models::Work::from_table(data, connection)
    }

    pub fn load_ensemble(&self, ensemble_id: &str) -> Result<models::Ensemble> {
        let connection = &mut *self.conn();

        let data = ensembles::table
            .filter(ensembles::ensemble_id.eq(ensemble_id))
            .first::<tables::Ensemble>(connection)?;

        models::Ensemble::from_table(data, connection)
    }

    pub fn load_recording(&self, recording_id: &str) -> Result<models::Recording> {
        let connection = &mut *self.conn();

        let data = recordings::table
            .filter(recordings::recording_id.eq(recording_id))
            .select(tables::Recording::as_select())
            .first::<tables::Recording>(connection)?;

        models::Recording::from_table(data, connection)
    }

    pub fn load_album(&self, album_id: &str) -> Result<models::Album> {
        let connection = &mut *self.conn();

        let data = albums::table
            .filter(albums::album_id.eq(album_id))
            .first::<tables::Album>(connection)?;

        models::Album::from_table(data, connection)
    }
}
