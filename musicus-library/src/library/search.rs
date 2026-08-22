use std::collections::HashSet;

use anyhow::Result;
use diesel::prelude::*;

use super::{query::composer_condition, Library};
use crate::db::{self, models::*, schema::*, tables};

/// A search result item that is either already part of the library or only
/// available from the separate metadata database.
#[derive(Clone, Debug)]
pub struct SearchItem<T> {
    pub item: T,
    pub in_library: bool,
}

fn in_library_items<T>(items: Vec<T>) -> Vec<SearchItem<T>> {
    items
        .into_iter()
        .map(|item| SearchItem {
            item,
            in_library: true,
        })
        .collect()
}

/// Append the metadata-database rows not already covered by `existing_ids`
/// to `results`, marked as not yet in the library.
fn merge_metadata_only<Raw, T>(
    results: &mut Vec<SearchItem<T>>,
    metadata_rows: Vec<Raw>,
    existing_ids: &HashSet<String>,
    id: impl Fn(&Raw) -> &str,
    mut convert: impl FnMut(Raw) -> Result<T>,
) -> Result<()> {
    for row in metadata_rows {
        if !existing_ids.contains(id(&row)) {
            results.push(SearchItem {
                item: convert(row)?,
                in_library: false,
            });
        }
    }

    Ok(())
}

impl Library {
    pub fn search_persons(&self, search: &str) -> Result<Vec<SearchItem<Person>>> {
        let search = format!("%{}%", search);
        let connection = &mut *self.conn();

        let persons: Vec<Person> = persons::table
            .order(persons::last_used_at.desc())
            .filter(persons::name.like(&search))
            .limit(20)
            .load(connection)?;

        let mut results = in_library_items(persons);

        if let Some(metadata_connection) = self.metadata_connection() {
            let metadata_connection = &mut *db::lock_connection(&metadata_connection);

            let metadata_persons: Vec<Person> = persons::table
                .filter(persons::name.like(&search))
                .limit(20)
                .load(metadata_connection)?;

            let candidate_ids: Vec<String> = metadata_persons
                .iter()
                .map(|p| p.person_id.clone())
                .collect();
            let existing: HashSet<String> = persons::table
                .filter(persons::person_id.eq_any(&candidate_ids))
                .select(persons::person_id)
                .load(connection)?
                .into_iter()
                .collect();

            merge_metadata_only(&mut results, metadata_persons, &existing, |p| &p.person_id, Ok)?;
        }

        Ok(results)
    }

    pub fn search_roles(&self, search: &str) -> Result<Vec<SearchItem<Role>>> {
        let search = format!("%{}%", search);
        let connection = &mut *self.conn();

        let roles: Vec<Role> = roles::table
            .order(roles::last_used_at.desc())
            .filter(roles::name.like(&search))
            .limit(20)
            .load(connection)?;

        let mut results = in_library_items(roles);

        if let Some(metadata_connection) = self.metadata_connection() {
            let metadata_connection = &mut *db::lock_connection(&metadata_connection);

            let metadata_roles: Vec<Role> = roles::table
                .filter(roles::name.like(&search))
                .limit(20)
                .load(metadata_connection)?;

            let candidate_ids: Vec<String> =
                metadata_roles.iter().map(|r| r.role_id.clone()).collect();
            let existing: HashSet<String> = roles::table
                .filter(roles::role_id.eq_any(&candidate_ids))
                .select(roles::role_id)
                .load(connection)?
                .into_iter()
                .collect();

            merge_metadata_only(&mut results, metadata_roles, &existing, |r| &r.role_id, Ok)?;
        }

        Ok(results)
    }

    pub fn search_tags(&self, search: &str) -> Result<Vec<SearchItem<Tag>>> {
        let search = format!("%{}%", search);
        let connection = &mut *self.conn();

        let tags: Vec<Tag> = tags::table
            .order(tags::last_used_at.desc())
            .filter(tags::name.like(&search))
            .limit(20)
            .load(connection)?;

        let mut results = in_library_items(tags);

        if let Some(metadata_connection) = self.metadata_connection() {
            let metadata_connection = &mut *db::lock_connection(&metadata_connection);

            let metadata_tags: Vec<Tag> = tags::table
                .filter(tags::name.like(&search))
                .limit(20)
                .load(metadata_connection)?;

            let candidate_ids: Vec<String> =
                metadata_tags.iter().map(|t| t.tag_id.clone()).collect();
            let existing: HashSet<String> = tags::table
                .filter(tags::tag_id.eq_any(&candidate_ids))
                .select(tags::tag_id)
                .load(connection)?
                .into_iter()
                .collect();

            merge_metadata_only(&mut results, metadata_tags, &existing, |t| &t.tag_id, Ok)?;
        }

        Ok(results)
    }

    /// The tags currently applied to any of the given works.
    pub fn tags_used_by_works(&self, work_ids: &[&str]) -> Result<Vec<Tag>> {
        let connection = &mut *self.conn();

        let tag_ids: Vec<String> = work_tags::table
            .filter(work_tags::work_id.eq_any(work_ids))
            .select(work_tags::tag_id)
            .distinct()
            .load(connection)?;

        Ok(tags::table
            .filter(tags::tag_id.eq_any(&tag_ids))
            .order(tags::last_used_at.desc())
            .load(connection)?)
    }

    /// The tags currently applied to any of the given recordings.
    pub fn tags_used_by_recordings(&self, recording_ids: &[&str]) -> Result<Vec<Tag>> {
        let connection = &mut *self.conn();

        let tag_ids: Vec<String> = recording_tags::table
            .filter(recording_tags::recording_id.eq_any(recording_ids))
            .select(recording_tags::tag_id)
            .distinct()
            .load(connection)?;

        Ok(tags::table
            .filter(tags::tag_id.eq_any(&tag_ids))
            .order(tags::last_used_at.desc())
            .load(connection)?)
    }

    pub fn search_instruments(&self, search: &str) -> Result<Vec<SearchItem<Instrument>>> {
        let search = format!("%{}%", search);
        let connection = &mut *self.conn();

        let instruments: Vec<Instrument> = instruments::table
            .order(instruments::last_used_at.desc())
            .filter(instruments::name.like(&search))
            .limit(20)
            .load(connection)?;

        let mut results = in_library_items(instruments);

        if let Some(metadata_connection) = self.metadata_connection() {
            let metadata_connection = &mut *db::lock_connection(&metadata_connection);

            let metadata_instruments: Vec<Instrument> = instruments::table
                .filter(instruments::name.like(&search))
                .limit(20)
                .load(metadata_connection)?;

            let candidate_ids: Vec<String> = metadata_instruments
                .iter()
                .map(|i| i.instrument_id.clone())
                .collect();
            let existing: HashSet<String> = instruments::table
                .filter(instruments::instrument_id.eq_any(&candidate_ids))
                .select(instruments::instrument_id)
                .load(connection)?
                .into_iter()
                .collect();

            merge_metadata_only(
                &mut results,
                metadata_instruments,
                &existing,
                |i| &i.instrument_id,
                Ok,
            )?;
        }

        Ok(results)
    }

    pub fn search_works(&self, composer: &Person, search: &str) -> Result<Vec<SearchItem<Work>>> {
        let search = format!("%{}%", search);
        let connection = &mut *self.conn();

        let works: Vec<tables::Work> = works::table
            .filter(works::name.like(&search).and(composer_condition(composer)))
            .into_boxed()
            .limit(9)
            .select(works::all_columns)
            .distinct()
            .load::<tables::Work>(connection)?;

        let mut results: Vec<SearchItem<Work>> = works
            .into_iter()
            .map(|w| {
                Work::from_table(w, connection).map(|item| SearchItem {
                    item,
                    in_library: true,
                })
            })
            .collect::<Result<Vec<SearchItem<Work>>>>()?;

        if let Some(metadata_connection) = self.metadata_connection() {
            let metadata_connection = &mut *db::lock_connection(&metadata_connection);

            let metadata_works: Vec<tables::Work> = works::table
                .filter(works::name.like(&search).and(composer_condition(composer)))
                .into_boxed()
                .limit(9)
                .select(works::all_columns)
                .distinct()
                .load::<tables::Work>(metadata_connection)?;

            let candidate_ids: Vec<String> =
                metadata_works.iter().map(|w| w.work_id.clone()).collect();
            let existing: HashSet<String> = works::table
                .filter(works::work_id.eq_any(&candidate_ids))
                .select(works::work_id)
                .load(connection)?
                .into_iter()
                .collect();

            merge_metadata_only(
                &mut results,
                metadata_works,
                &existing,
                |w| &w.work_id,
                |w| Work::from_table(w, metadata_connection),
            )?;
        }

        Ok(results)
    }

    pub fn search_recordings(
        &self,
        work: &Work,
        search: &str,
    ) -> Result<Vec<SearchItem<Recording>>> {
        let search = format!("%{}%", search);
        let connection = &mut *self.conn();

        let recordings: Vec<tables::Recording> = recordings::table
            .left_join(recording_persons::table.inner_join(persons::table))
            .left_join(recording_ensembles::table.inner_join(ensembles::table))
            .filter(
                recordings::work_id.eq(&work.work_id).and(
                    persons::name
                        .like(&search)
                        .or(ensembles::name.like(&search)),
                ),
            )
            .limit(9)
            .select(recordings::all_columns)
            .distinct()
            .load::<tables::Recording>(connection)?;

        let mut results: Vec<SearchItem<Recording>> = recordings
            .into_iter()
            .map(|r| {
                Recording::from_table(r, connection).map(|item| SearchItem {
                    item,
                    in_library: true,
                })
            })
            .collect::<Result<Vec<SearchItem<Recording>>>>()?;

        if let Some(metadata_connection) = self.metadata_connection() {
            let metadata_connection = &mut *db::lock_connection(&metadata_connection);

            let metadata_recordings: Vec<tables::Recording> = recordings::table
                .left_join(recording_persons::table.inner_join(persons::table))
                .left_join(recording_ensembles::table.inner_join(ensembles::table))
                .filter(
                    recordings::work_id.eq(&work.work_id).and(
                        persons::name
                            .like(&search)
                            .or(ensembles::name.like(&search)),
                    ),
                )
                .limit(9)
                .select(recordings::all_columns)
                .distinct()
                .load::<tables::Recording>(metadata_connection)?;

            let candidate_ids: Vec<String> = metadata_recordings
                .iter()
                .map(|r| r.recording_id.clone())
                .collect();
            let existing: HashSet<String> = recordings::table
                .filter(recordings::recording_id.eq_any(&candidate_ids))
                .select(recordings::recording_id)
                .load(connection)?
                .into_iter()
                .collect();

            merge_metadata_only(
                &mut results,
                metadata_recordings,
                &existing,
                |r| &r.recording_id,
                |r| Recording::from_table(r, metadata_connection),
            )?;
        }

        Ok(results)
    }

    pub fn search_ensembles(&self, search: &str) -> Result<Vec<SearchItem<Ensemble>>> {
        let search = format!("%{}%", search);
        let connection = &mut *self.conn();

        let ensembles: Vec<tables::Ensemble> = ensembles::table
            .order(ensembles::last_used_at.desc())
            .left_join(ensemble_persons::table.inner_join(persons::table))
            .filter(
                ensembles::name
                    .like(&search)
                    .or(persons::name.like(&search)),
            )
            .limit(20)
            .select(ensembles::all_columns)
            .load::<tables::Ensemble>(connection)?;

        let mut results: Vec<SearchItem<Ensemble>> = ensembles
            .into_iter()
            .map(|e| {
                Ensemble::from_table(e, connection).map(|item| SearchItem {
                    item,
                    in_library: true,
                })
            })
            .collect::<Result<Vec<SearchItem<Ensemble>>>>()?;

        if let Some(metadata_connection) = self.metadata_connection() {
            let metadata_connection = &mut *db::lock_connection(&metadata_connection);

            let metadata_ensembles: Vec<tables::Ensemble> = ensembles::table
                .left_join(ensemble_persons::table.inner_join(persons::table))
                .filter(
                    ensembles::name
                        .like(&search)
                        .or(persons::name.like(&search)),
                )
                .limit(20)
                .select(ensembles::all_columns)
                .load::<tables::Ensemble>(metadata_connection)?;

            let candidate_ids: Vec<String> = metadata_ensembles
                .iter()
                .map(|e| e.ensemble_id.clone())
                .collect();
            let existing: HashSet<String> = ensembles::table
                .filter(ensembles::ensemble_id.eq_any(&candidate_ids))
                .select(ensembles::ensemble_id)
                .load(connection)?
                .into_iter()
                .collect();

            merge_metadata_only(
                &mut results,
                metadata_ensembles,
                &existing,
                |e| &e.ensemble_id,
                |e| Ensemble::from_table(e, metadata_connection),
            )?;
        }

        Ok(results)
    }
}
