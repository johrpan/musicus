use std::collections::HashSet;

use anyhow::Result;
use diesel::prelude::*;

use super::{query::translated_like, query::work_person_condition, Library};
use crate::{
    db::{self, schema::*, tables},
    library::load::Loader,
    models::*,
};

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
            .filter(translated_like("persons.name", &search))
            .limit(20)
            .load::<tables::Person>(connection)?
            .into_iter()
            .map(Person::from)
            .collect();

        let mut results = in_library_items(persons);

        if let Some(metadata_connection) = self.metadata_connection() {
            let metadata_connection = &mut *db::lock_connection(&metadata_connection);

            let metadata_persons: Vec<tables::Person> = persons::table
                .filter(translated_like("persons.name", &search))
                .limit(20)
                .load(metadata_connection)?;

            let existing_ids: HashSet<String> = persons::table
                .filter(persons::person_id.eq_any(metadata_persons.iter().map(|p| &p.person_id)))
                .select(persons::person_id)
                .load(connection)?
                .into_iter()
                .collect();

            merge_metadata_only(
                &mut results,
                metadata_persons,
                &existing_ids,
                |p| &p.person_id,
                |p| Ok(Person::from(p)),
            )?;
        }

        Ok(results)
    }

    pub fn search_roles(&self, search: &str) -> Result<Vec<SearchItem<Role>>> {
        let search = format!("%{}%", search);
        let connection = &mut *self.conn();

        let roles: Vec<Role> = roles::table
            .order(roles::last_used_at.desc())
            .filter(translated_like("roles.name", &search))
            .limit(20)
            .load::<tables::Role>(connection)?
            .into_iter()
            .map(Role::from)
            .collect();

        let mut results = in_library_items(roles);

        if let Some(metadata_connection) = self.metadata_connection() {
            let metadata_connection = &mut *db::lock_connection(&metadata_connection);

            let metadata_roles: Vec<tables::Role> = roles::table
                .filter(translated_like("roles.name", &search))
                .limit(20)
                .load(metadata_connection)?;

            let existing_ids: HashSet<String> = roles::table
                .filter(roles::role_id.eq_any(metadata_roles.iter().map(|r| &r.role_id)))
                .select(roles::role_id)
                .load(connection)?
                .into_iter()
                .collect();

            merge_metadata_only(
                &mut results,
                metadata_roles,
                &existing_ids,
                |r| &r.role_id,
                |r| Ok(Role::from(r)),
            )?;
        }

        Ok(results)
    }

    pub fn search_tags(&self, search: &str) -> Result<Vec<SearchItem<Tag>>> {
        let search = format!("%{}%", search);
        let connection = &mut *self.conn();

        let tags: Vec<Tag> = tags::table
            .order(tags::last_used_at.desc())
            .filter(translated_like("tags.name", &search))
            .limit(20)
            .load::<tables::Tag>(connection)?
            .into_iter()
            .map(Tag::from)
            .collect();

        let mut results = in_library_items(tags);

        if let Some(metadata_connection) = self.metadata_connection() {
            let metadata_connection = &mut *db::lock_connection(&metadata_connection);

            let metadata_tags: Vec<tables::Tag> = tags::table
                .filter(translated_like("tags.name", &search))
                .limit(20)
                .load(metadata_connection)?;

            let existing_ids: HashSet<String> = tags::table
                .filter(tags::tag_id.eq_any(metadata_tags.iter().map(|t| &t.tag_id)))
                .select(tags::tag_id)
                .load(connection)?
                .into_iter()
                .collect();

            merge_metadata_only(
                &mut results,
                metadata_tags,
                &existing_ids,
                |t| &t.tag_id,
                |t| Ok(Tag::from(t)),
            )?;
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
            .load::<tables::Tag>(connection)?
            .into_iter()
            .map(Tag::from)
            .collect())
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
            .load::<tables::Tag>(connection)?
            .into_iter()
            .map(Tag::from)
            .collect())
    }

    pub fn search_instruments(&self, search: &str) -> Result<Vec<SearchItem<Instrument>>> {
        let search = format!("%{}%", search);
        let connection = &mut *self.conn();

        let instruments: Vec<Instrument> = instruments::table
            .order(instruments::last_used_at.desc())
            .filter(translated_like("instruments.name", &search))
            .limit(20)
            .load::<tables::Instrument>(connection)?
            .into_iter()
            .map(Instrument::from)
            .collect();

        let mut results = in_library_items(instruments);

        if let Some(metadata_connection) = self.metadata_connection() {
            let metadata_connection = &mut *db::lock_connection(&metadata_connection);

            let metadata_instruments: Vec<tables::Instrument> = instruments::table
                .filter(translated_like("instruments.name", &search))
                .limit(20)
                .load(metadata_connection)?;

            let existing_ids: HashSet<String> = instruments::table
                .filter(
                    instruments::instrument_id
                        .eq_any(metadata_instruments.iter().map(|i| &i.instrument_id)),
                )
                .select(instruments::instrument_id)
                .load(connection)?
                .into_iter()
                .collect();

            merge_metadata_only(
                &mut results,
                metadata_instruments,
                &existing_ids,
                |i| &i.instrument_id,
                |i| Ok(Instrument::from(i)),
            )?;
        }

        Ok(results)
    }

    pub fn search_works(
        &self,
        work_person: &Person,
        search: &str,
    ) -> Result<Vec<SearchItem<Work>>> {
        let search = format!("%{}%", search);
        let connection = &mut *self.conn();

        let loader = Loader::load(connection)?;

        let works: Vec<tables::Work> = works::table
            .filter(translated_like("works.name", &search).and(work_person_condition(work_person)))
            .into_boxed()
            .limit(9)
            .select(works::all_columns)
            .distinct()
            .load::<tables::Work>(connection)?;

        let mut results: Vec<SearchItem<Work>> = works
            .into_iter()
            .map(|w| {
                loader.work(&w.work_id).map(|item| SearchItem {
                    item,
                    in_library: true,
                })
            })
            .collect::<Result<Vec<SearchItem<Work>>>>()?;

        if let Some(metadata_connection) = self.metadata_connection() {
            let metadata_connection = &mut *db::lock_connection(&metadata_connection);

            let metadata_works: Vec<tables::Work> = works::table
                .filter(
                    translated_like("works.name", &search).and(work_person_condition(work_person)),
                )
                .into_boxed()
                .limit(9)
                .select(works::all_columns)
                .distinct()
                .load::<tables::Work>(metadata_connection)?;

            let existing_ids: HashSet<String> = works::table
                .filter(works::work_id.eq_any(metadata_works.iter().map(|w| &w.work_id)))
                .select(works::work_id)
                .load(connection)?
                .into_iter()
                .collect();

            let metadata_loader = Loader::load(metadata_connection)?;

            merge_metadata_only(
                &mut results,
                metadata_works,
                &existing_ids,
                |w| &w.work_id,
                |w| metadata_loader.work(&w.work_id),
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

        let loader = Loader::load(connection)?;

        let recordings: Vec<tables::Recording> = recordings::table
            .left_join(recording_persons::table.inner_join(persons::table))
            .left_join(recording_ensembles::table.inner_join(ensembles::table))
            .filter(
                recordings::work_id.eq(&work.id).and(
                    translated_like("persons.name", &search)
                        .or(translated_like("ensembles.name", &search)),
                ),
            )
            .limit(9)
            .select(recordings::all_columns)
            .distinct()
            .load::<tables::Recording>(connection)?;

        let mut results: Vec<SearchItem<Recording>> = recordings
            .into_iter()
            .map(|r| {
                loader.recording(&r.recording_id).map(|item| SearchItem {
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
                    recordings::work_id.eq(&work.id).and(
                        translated_like("persons.name", &search)
                            .or(translated_like("ensembles.name", &search)),
                    ),
                )
                .limit(9)
                .select(recordings::all_columns)
                .distinct()
                .load::<tables::Recording>(metadata_connection)?;

            let existing_ids: HashSet<String> = recordings::table
                .filter(
                    recordings::recording_id
                        .eq_any(metadata_recordings.iter().map(|r| &r.recording_id)),
                )
                .select(recordings::recording_id)
                .load(connection)?
                .into_iter()
                .collect();

            let metadata_loader = Loader::load(metadata_connection)?;

            merge_metadata_only(
                &mut results,
                metadata_recordings,
                &existing_ids,
                |r| &r.recording_id,
                |r| metadata_loader.recording(&r.recording_id),
            )?;
        }

        Ok(results)
    }

    pub fn search_ensembles(&self, search: &str) -> Result<Vec<SearchItem<Ensemble>>> {
        let search = format!("%{}%", search);
        let connection = &mut *self.conn();

        let loader = Loader::load(connection)?;

        let ensembles: Vec<tables::Ensemble> = ensembles::table
            .order(ensembles::last_used_at.desc())
            .left_join(ensemble_persons::table.inner_join(persons::table))
            .filter(
                translated_like("ensembles.name", &search)
                    .or(translated_like("persons.name", &search)),
            )
            .limit(20)
            .select(ensembles::all_columns)
            .load::<tables::Ensemble>(connection)?;

        let mut results: Vec<SearchItem<Ensemble>> = ensembles
            .into_iter()
            .map(|e| {
                loader.ensemble(&e.ensemble_id).map(|item| SearchItem {
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
                    translated_like("ensembles.name", &search)
                        .or(translated_like("persons.name", &search)),
                )
                .limit(20)
                .select(ensembles::all_columns)
                .load::<tables::Ensemble>(metadata_connection)?;

            let existing_ids: HashSet<String> = ensembles::table
                .filter(
                    ensembles::ensemble_id
                        .eq_any(metadata_ensembles.iter().map(|e| &e.ensemble_id)),
                )
                .select(ensembles::ensemble_id)
                .load(connection)?
                .into_iter()
                .collect();

            let metadata_loader = Loader::load(metadata_connection)?;

            merge_metadata_only(
                &mut results,
                metadata_ensembles,
                &existing_ids,
                |e| &e.ensemble_id,
                |e| metadata_loader.ensemble(&e.ensemble_id),
            )?;
        }

        Ok(results)
    }
}
