use std::collections::HashSet;

use anyhow::Result;
use diesel::{dsl::sql, prelude::*, sql_types, QueryDsl};

use gettextrs::gettext;

use super::{metadata::SearchItem, Library};
use crate::{
    db::{self, models::*, schema::*, tables, views::*},
    format_translated,
};

/// A single item that a [`LibraryQuery`] can be about, or that search results can be
/// highlighted with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Facet {
    Composer(Person),
    Performer(Person),
    Ensemble(Ensemble),
    Instrument(Instrument),
    Work(Work),
    Tag(TagValue),
}

#[derive(Clone, Default, Debug)]
pub struct LibraryQuery {
    pub composer: Option<Person>,
    pub performer: Option<Person>,
    pub ensemble: Option<Ensemble>,
    pub instrument: Option<Instrument>,
    pub work: Option<Work>,
    pub tag: Option<TagValue>,
}

impl LibraryQuery {
    pub fn is_empty(&self) -> bool {
        self.composer.is_none()
            && self.performer.is_none()
            && self.ensemble.is_none()
            && self.instrument.is_none()
            && self.work.is_none()
            && self.tag.is_none()
    }

    /// The item that the query is mainly about, if there is one.
    pub fn highlight(&self) -> Option<Facet> {
        if let Some(work) = &self.work {
            Some(Facet::Work(work.to_owned()))
        } else if let Some(person) = &self.composer {
            Some(Facet::Composer(person.to_owned()))
        } else if let Some(person) = &self.performer {
            Some(Facet::Performer(person.to_owned()))
        } else if let Some(ensemble) = &self.ensemble {
            Some(Facet::Ensemble(ensemble.to_owned()))
        } else if let Some(instrument) = &self.instrument {
            Some(Facet::Instrument(instrument.to_owned()))
        } else if let Some(tag) = &self.tag {
            Some(Facet::Tag(tag.to_owned()))
        } else {
            None
        }
    }

    /// A short name for what the query selects, based on its [`Self::highlight`].
    pub fn title(&self) -> Option<String> {
        Some(match self.highlight()? {
            Facet::Work(work) => work.name.get().to_owned(),
            Facet::Composer(person) | Facet::Performer(person) => person.name.get().to_owned(),
            Facet::Ensemble(ensemble) => ensemble.name.get().to_owned(),
            Facet::Instrument(instrument) => {
                format_translated!(gettext("Music for {}"), instrument.name.get())
            }
            Facet::Tag(tag) => tag.to_string(),
        })
    }

    /// The parts of the query that are not already covered by its [`Self::title`].
    pub fn description(&self) -> Option<String> {
        let mut details = Vec::new();

        match self.highlight()? {
            Facet::Work(work) => {
                if let Some(composers) = work.composers_string() {
                    details.push(composers);
                }
            }
            Facet::Composer(_) => {
                if let Some(instrument) = &self.instrument {
                    details.push(format_translated!(gettext("Works with {}"), instrument));
                }

                if let (Some(person), Some(ensemble)) = (&self.performer, &self.ensemble) {
                    details.push(format_translated!(
                        gettext("Performed by {} and {}"),
                        person,
                        ensemble
                    ));
                } else if let Some(person) = &self.performer {
                    details.push(format_translated!(gettext("Performed by {}"), person));
                } else if let Some(ensemble) = &self.ensemble {
                    details.push(format_translated!(gettext("Performed by {}"), ensemble));
                }
            }
            Facet::Performer(_) => {
                if let Some(instrument) = &self.instrument {
                    details.push(format_translated!(gettext("Works with {}"), instrument));
                }

                if let Some(ensemble) = &self.ensemble {
                    details.push(format_translated!(gettext("Performed with {}"), ensemble));
                }
            }
            Facet::Ensemble(ensemble) => {
                if let Some(instrument) = &self.instrument {
                    details.push(format_translated!(gettext("Works with {}"), instrument));
                }

                if let Some(members) = ensemble.members_string() {
                    details.push(format_translated!(gettext("Members: {}"), members));
                }
            }
            Facet::Instrument(_) => (),
            Facet::Tag(_) => {
                if let Some(instrument) = &self.instrument {
                    details.push(format_translated!(gettext("Works with {}"), instrument));
                }

                if let Some(person) = &self.performer {
                    details.push(format_translated!(gettext("Performed by {}"), person));
                } else if let Some(ensemble) = &self.ensemble {
                    details.push(format_translated!(gettext("Performed by {}"), ensemble));
                }
            }
        }

        if let Some(tag) = &self.tag {
            if !matches!(self.highlight(), Some(Facet::Tag(_))) {
                details.push(tag.to_string());
            }
        }

        if details.is_empty() {
            None
        } else {
            Some(details.join(", "))
        }
    }
}

#[derive(Default, Debug)]
pub struct LibraryResults {
    pub composers: Vec<Person>,
    pub performers: Vec<Person>,
    pub ensembles: Vec<Ensemble>,
    pub instruments: Vec<Instrument>,
    pub works: Vec<Work>,
    pub recordings: Vec<Recording>,
    pub albums: Vec<Album>,
    pub tags: Vec<TagValue>,
    /// The work a selected work is a part of, if it is a part of anything.
    pub parent_work: Option<Work>,
    /// The selected work's place in the part structure: its own movements if it
    /// has any, or otherwise its siblings under [`Self::parent_work`], if it has a
    /// parent.
    pub structure: Vec<Work>,
}

impl LibraryResults {
    pub fn is_empty(&self) -> bool {
        self.composers.is_empty()
            && self.performers.is_empty()
            && self.ensembles.is_empty()
            && self.instruments.is_empty()
            && self.works.is_empty()
            && self.recordings.is_empty()
            && self.albums.is_empty()
            && self.tags.is_empty()
            && self.structure.is_empty()
    }
}

/// A `{col} IN (...)` SQL fragment matching `col` against the exact work referenced
/// by `works.work_id` in the enclosing query, any of its ancestors up to the root, or
/// any of its descendants.
fn in_work_subtree(col: &str) -> String {
    format!(
        "{col} IN (\
            WITH RECURSIVE \
                ancestors(work_id) AS (\
                    SELECT works.work_id \
                    UNION \
                    SELECT w.parent_work_id FROM works w \
                    JOIN ancestors a ON w.work_id = a.work_id \
                    WHERE w.parent_work_id IS NOT NULL\
                ), \
                descendants(work_id) AS (\
                    SELECT works.work_id \
                    UNION \
                    SELECT w.work_id FROM works w \
                    JOIN descendants d ON w.parent_work_id = d.work_id\
                ) \
            SELECT work_id FROM ancestors UNION SELECT work_id FROM descendants\
        )"
    )
}

/// A tag matches if it is on the work itself (or an ancestor/descendant of it, see
/// [`in_work_subtree`]) or on any of its recordings. Written out because the
/// subquery needs its own alias for `recordings`, which is already in the outer
/// query. A valued tag matches only that exact value; a label tag binds NULL, which
/// matches any assignment.
fn tag_condition<QS>(
    tag: &TagValue,
) -> impl Expression<SqlType = sql_types::Bool>
       + AppearsOnTable<QS>
       + diesel::expression::ValidGrouping<(), IsAggregate = diesel::expression::is_aggregate::Never>
       + diesel::query_builder::QueryFragment<diesel::sqlite::Sqlite> {
    sql::<sql_types::Bool>(&format!(
        "(EXISTS (SELECT 1 FROM work_tags WHERE {} AND work_tags.tag_id = ",
        in_work_subtree("work_tags.work_id"),
    ))
    .bind::<sql_types::Text, _>(tag.tag.tag_id.clone())
    .sql(" AND (")
    .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
    .sql(" IS NULL OR work_tags.value = ")
    .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
    .sql(&format!(
        ")) OR EXISTS (SELECT 1 FROM recording_tags \
          JOIN recordings AS tagged_recordings \
          ON tagged_recordings.recording_id = recording_tags.recording_id \
          WHERE {} \
          AND recording_tags.tag_id = ",
        in_work_subtree("tagged_recordings.work_id"),
    ))
    .bind::<sql_types::Text, _>(tag.tag.tag_id.clone())
    .sql(" AND (")
    .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
    .sql(" IS NULL OR recording_tags.value = ")
    .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
    .sql(")))")
}

/// True if `person` is credited as a composer on the work referenced by
/// `works.work_id` in the enclosing query, or an ancestor/descendant of it (see
/// [`in_work_subtree`]).
fn composer_condition<QS>(
    person: &Person,
) -> impl Expression<SqlType = sql_types::Bool>
       + AppearsOnTable<QS>
       + diesel::expression::ValidGrouping<(), IsAggregate = diesel::expression::is_aggregate::Never>
       + diesel::query_builder::QueryFragment<diesel::sqlite::Sqlite> {
    sql::<sql_types::Bool>("EXISTS (SELECT 1 FROM work_persons WHERE work_persons.person_id = ")
        .bind::<sql_types::Text, _>(person.person_id.clone())
        .sql(&format!(
            " AND {})",
            in_work_subtree("work_persons.work_id")
        ))
}

/// True if `instrument` is included in the work referenced by `works.work_id` in the
/// enclosing query, or an ancestor/descendant of it (see [`in_work_subtree`]).
fn work_instrument_condition<QS>(
    instrument: &Instrument,
) -> impl Expression<SqlType = sql_types::Bool>
       + AppearsOnTable<QS>
       + diesel::expression::ValidGrouping<(), IsAggregate = diesel::expression::is_aggregate::Never>
       + diesel::query_builder::QueryFragment<diesel::sqlite::Sqlite> {
    sql::<sql_types::Bool>(
        "EXISTS (SELECT 1 FROM work_instruments WHERE work_instruments.instrument_id = ",
    )
    .bind::<sql_types::Text, _>(instrument.instrument_id.clone())
    .sql(&format!(
        " AND {})",
        in_work_subtree("work_instruments.work_id")
    ))
}

impl Library {
    pub fn search(&self, query: &LibraryQuery, search: &str) -> Result<LibraryResults> {
        let search = format!("%{}%", search);
        let connection = &mut *self.conn();

        Ok(match query {
            LibraryQuery { work: None, .. } => {
                let composers = if query.composer.is_none() {
                    let mut statement = persons::table
                        .inner_join(
                            work_persons::table.inner_join(
                                works::table
                                    .on(sql::<sql_types::Bool>(&in_work_subtree(
                                        "work_persons.work_id",
                                    )))
                                    .inner_join(
                                        recordings::table
                                            .left_join(recording_ensembles::table.inner_join(
                                                ensembles::table.left_join(ensemble_persons::table),
                                            ))
                                            .left_join(recording_persons::table),
                                    )
                                    .left_join(work_instruments::table),
                            ),
                        )
                        .filter(persons::name.like(&search))
                        .into_boxed();

                    if let Some(person) = &query.performer {
                        statement = statement.filter(
                            recording_persons::person_id
                                .eq(&person.person_id)
                                .or(ensemble_persons::person_id.eq(&person.person_id)),
                        );
                    }

                    if let Some(ensemble) = &query.ensemble {
                        statement = statement
                            .filter(recording_ensembles::ensemble_id.eq(&ensemble.ensemble_id));
                    }

                    if let Some(instrument) = &query.instrument {
                        statement = statement
                            .filter(work_instrument_condition(instrument).or(
                                recording_persons::instrument_id.eq(&instrument.instrument_id),
                            ));
                    }

                    if let Some(tag) = &query.tag {
                        statement = statement.filter(tag_condition(tag));
                    }

                    statement
                        .order_by(
                            person_last_played::table
                                .filter(person_last_played::person_id.eq(persons::person_id))
                                .select(person_last_played::last_played_at)
                                .single_value()
                                .desc(),
                        )
                        .limit(9)
                        .select(persons::all_columns)
                        .distinct()
                        .load::<Person>(connection)?
                } else {
                    Vec::new()
                };

                let performers = if query.performer.is_none() {
                    let mut statement = persons::table
                        .inner_join(
                            recording_persons::table.inner_join(
                                recordings::table
                                    .inner_join(
                                        works::table
                                            .left_join(work_persons::table)
                                            .left_join(work_instruments::table),
                                    )
                                    .left_join(recording_ensembles::table),
                            ),
                        )
                        .filter(persons::name.like(&search))
                        .into_boxed();

                    if let Some(person) = &query.composer {
                        statement = statement.filter(composer_condition(person));
                    }

                    if let Some(ensemble) = &query.ensemble {
                        statement = statement
                            .filter(recording_ensembles::ensemble_id.eq(&ensemble.ensemble_id));
                    }

                    if let Some(instrument) = &query.instrument {
                        statement = statement
                            .filter(work_instrument_condition(instrument).or(
                                recording_persons::instrument_id.eq(&instrument.instrument_id),
                            ));
                    }

                    if let Some(tag) = &query.tag {
                        statement = statement.filter(tag_condition(tag));
                    }

                    statement
                        .order_by(
                            performer_last_played::table
                                .filter(performer_last_played::person_id.eq(persons::person_id))
                                .select(performer_last_played::last_played_at)
                                .single_value()
                                .desc(),
                        )
                        .limit(9)
                        .select(persons::all_columns)
                        .distinct()
                        .load::<Person>(connection)?
                } else {
                    Vec::new()
                };

                let ensembles = if query.ensemble.is_none() {
                    let mut statement = ensembles::table
                        .inner_join(
                            recording_ensembles::table.inner_join(
                                recordings::table
                                    .inner_join(
                                        works::table
                                            .left_join(work_persons::table)
                                            .left_join(work_instruments::table),
                                    )
                                    .left_join(recording_persons::table),
                            ),
                        )
                        .left_join(ensemble_persons::table.inner_join(persons::table))
                        .filter(
                            ensembles::name
                                .like(&search)
                                .or(persons::name.like(&search)),
                        )
                        .into_boxed();

                    if let Some(person) = &query.composer {
                        statement = statement.filter(composer_condition(person));
                    }

                    if let Some(person) = &query.performer {
                        statement = statement.filter(
                            recording_persons::person_id
                                .eq(&person.person_id)
                                .or(ensemble_persons::person_id.eq(&person.person_id)),
                        );
                    }

                    if let Some(instrument) = &query.instrument {
                        statement = statement.filter(
                            work_instrument_condition(instrument)
                                .or(ensemble_persons::instrument_id.eq(&instrument.instrument_id)),
                        );
                    }

                    if let Some(tag) = &query.tag {
                        statement = statement.filter(tag_condition(tag));
                    }

                    statement
                        .order_by(
                            ensemble_last_played::table
                                .filter(
                                    ensemble_last_played::ensemble_id.eq(ensembles::ensemble_id),
                                )
                                .select(ensemble_last_played::last_played_at)
                                .single_value()
                                .desc(),
                        )
                        .limit(9)
                        .select(ensembles::all_columns)
                        .distinct()
                        .load::<tables::Ensemble>(connection)?
                        .into_iter()
                        .map(|e| Ensemble::from_table(e, connection))
                        .collect::<Result<Vec<Ensemble>>>()?
                } else {
                    Vec::new()
                };

                let instruments = if query.instrument.is_none() {
                    let mut statement = instruments::table
                        .left_join(
                            work_instruments::table.inner_join(
                                works::table
                                    .on(sql::<sql_types::Bool>(&in_work_subtree(
                                        "work_instruments.work_id",
                                    )))
                                    .left_join(work_persons::table),
                            ),
                        )
                        .left_join(recording_persons::table)
                        .left_join(ensemble_persons::table)
                        .filter(instruments::name.like(&search))
                        .into_boxed();

                    if let Some(person) = &query.composer {
                        statement = statement.filter(composer_condition(person));
                    }

                    if let Some(person) = &query.performer {
                        statement = statement.filter(
                            recording_persons::person_id
                                .eq(&person.person_id)
                                .or(ensemble_persons::person_id.eq(&person.person_id)),
                        );
                    }

                    if let Some(ensemble) = &query.ensemble {
                        statement = statement
                            .filter(ensemble_persons::ensemble_id.eq(&ensemble.ensemble_id));
                    }

                    if let Some(tag) = &query.tag {
                        statement = statement.filter(tag_condition(tag));
                    }

                    statement
                        .order_by(
                            instrument_last_played::table
                                .filter(
                                    instrument_last_played::instrument_id
                                        .eq(instruments::instrument_id),
                                )
                                .select(instrument_last_played::last_played_at)
                                .single_value()
                                .desc(),
                        )
                        .limit(9)
                        .select(instruments::all_columns)
                        .distinct()
                        .load::<Instrument>(connection)?
                } else {
                    Vec::new()
                };

                let works = if query.work.is_none() {
                    let mut statement = works::table
                        .left_join(work_persons::table)
                        .inner_join(
                            recordings::table
                                .left_join(recording_persons::table)
                                .left_join(recording_ensembles::table.left_join(
                                    ensembles::table.inner_join(ensemble_persons::table),
                                )),
                        )
                        .left_join(work_instruments::table)
                        .filter(works::name.like(&search))
                        .into_boxed();

                    if let Some(person) = &query.composer {
                        statement = statement.filter(composer_condition(person));
                    }

                    if let Some(person) = &query.performer {
                        statement = statement.filter(
                            recording_persons::person_id
                                .eq(&person.person_id)
                                .or(ensemble_persons::person_id.eq(&person.person_id)),
                        );
                    }

                    if let Some(instrument) = &query.instrument {
                        statement = statement.filter(
                            work_instrument_condition(instrument)
                                .or(recording_persons::instrument_id.eq(&instrument.instrument_id))
                                .or(ensemble_persons::instrument_id.eq(&instrument.instrument_id)),
                        );
                    }

                    if let Some(ensemble) = &query.ensemble {
                        statement = statement
                            .filter(recording_ensembles::ensemble_id.eq(&ensemble.ensemble_id));
                    }

                    if let Some(tag) = &query.tag {
                        statement = statement.filter(tag_condition(tag));
                    }

                    statement
                        .order_by(
                            work_last_played::table
                                .filter(work_last_played::work_id.eq(works::work_id))
                                .select(work_last_played::last_played_at)
                                .single_value()
                                .desc(),
                        )
                        .limit(9)
                        .select(works::all_columns)
                        .distinct()
                        .load::<tables::Work>(connection)?
                        .into_iter()
                        .map(|w| Work::from_table(w, connection))
                        .collect::<Result<Vec<Work>>>()?
                } else {
                    Vec::new()
                };

                // Only search recordings in special cases. Works will always be searched and
                // directly lead to recordings. The special case of a work in the query is already
                // handled in another branch of the top-level match expression.
                let recordings = if query.performer.is_some() || query.ensemble.is_some() {
                    let mut statement = recordings::table
                        .inner_join(
                            works::table
                                .left_join(work_persons::table)
                                .left_join(work_instruments::table),
                        )
                        .left_join(recording_persons::table)
                        .left_join(
                            recording_ensembles::table
                                .inner_join(ensembles::table.left_join(ensemble_persons::table)),
                        )
                        .filter(works::name.like(&search))
                        .into_boxed();

                    if let Some(person) = &query.composer {
                        statement = statement.filter(composer_condition(person));
                    }

                    if let Some(person) = &query.performer {
                        statement = statement.filter(
                            recording_persons::person_id
                                .eq(&person.person_id)
                                .or(ensemble_persons::person_id.eq(&person.person_id)),
                        );
                    }

                    if let Some(instrument) = &query.instrument {
                        statement = statement.filter(
                            work_instrument_condition(instrument)
                                .or(recording_persons::instrument_id.eq(&instrument.instrument_id))
                                .or(ensemble_persons::instrument_id.eq(&instrument.instrument_id)),
                        );
                    }

                    if let Some(ensemble) = &query.ensemble {
                        statement = statement
                            .filter(recording_ensembles::ensemble_id.eq(&ensemble.ensemble_id));
                    }

                    if let Some(tag) = &query.tag {
                        // For a recording the assignment on the recording itself is what counts,
                        // falling back to its work. Going through the work alone would match every
                        // sibling recording, which is wrong once a tag carries a value.
                        statement = statement.filter(
                            sql::<sql_types::Bool>(
                                "(EXISTS (SELECT 1 FROM recording_tags \
                                  WHERE recording_tags.recording_id = recordings.recording_id \
                                  AND recording_tags.tag_id = ",
                            )
                            .bind::<sql_types::Text, _>(tag.tag.tag_id.clone())
                            .sql(" AND (")
                            .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
                            .sql(" IS NULL OR recording_tags.value = ")
                            .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
                            .sql(
                                ")) OR EXISTS (SELECT 1 FROM work_tags \
                                 WHERE work_tags.work_id = recordings.work_id \
                                 AND work_tags.tag_id = ",
                            )
                            .bind::<sql_types::Text, _>(tag.tag.tag_id.clone())
                            .sql(" AND (")
                            .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
                            .sql(" IS NULL OR work_tags.value = ")
                            .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
                            .sql(")))"),
                        );
                    }

                    statement
                        .order_by(
                            recording_last_played::table
                                .filter(
                                    recording_last_played::recording_id
                                        .eq(recordings::recording_id),
                                )
                                .select(recording_last_played::last_played_at)
                                .single_value()
                                .desc(),
                        )
                        .limit(9)
                        .select(recordings::all_columns)
                        .distinct()
                        .load::<tables::Recording>(connection)?
                        .into_iter()
                        .map(|r| Recording::from_table(r, connection))
                        .collect::<Result<Vec<Recording>>>()?
                } else {
                    Vec::new()
                };

                let mut statement = albums::table
                    .inner_join(
                        album_recordings::table.inner_join(
                            recordings::table
                                .inner_join(
                                    works::table
                                        .left_join(work_persons::table)
                                        .left_join(work_instruments::table),
                                )
                                .left_join(recording_persons::table)
                                .left_join(recording_ensembles::table.inner_join(
                                    ensembles::table.left_join(ensemble_persons::table),
                                )),
                        ),
                    )
                    .filter(albums::name.like(&search))
                    .into_boxed();

                if let Some(person) = &query.composer {
                    statement = statement.filter(composer_condition(person));
                }

                if let Some(person) = &query.performer {
                    statement = statement.filter(
                        recording_persons::person_id
                            .eq(&person.person_id)
                            .or(ensemble_persons::person_id.eq(&person.person_id)),
                    );
                }

                if let Some(instrument) = &query.instrument {
                    statement = statement.filter(
                        work_instrument_condition(instrument)
                            .or(recording_persons::instrument_id.eq(&instrument.instrument_id))
                            .or(ensemble_persons::instrument_id.eq(&instrument.instrument_id)),
                    );
                }

                if let Some(ensemble) = &query.ensemble {
                    statement = statement
                        .filter(recording_ensembles::ensemble_id.eq(&ensemble.ensemble_id));
                }

                if let Some(tag) = &query.tag {
                    statement = statement.filter(tag_condition(tag));
                }

                let albums = statement
                    .order_by(
                        album_last_played::table
                            .filter(album_last_played::album_id.eq(albums::album_id))
                            .select(album_last_played::last_played_at)
                            .single_value()
                            .desc(),
                    )
                    .limit(9)
                    .select(albums::all_columns)
                    .distinct()
                    .load::<tables::Album>(connection)?
                    .into_iter()
                    .map(|r| Album::from_table(r, connection))
                    .collect::<Result<Vec<Album>>>()?;

                // Tags are facets like any other, so they have to come from items
                // the rest of the query actually selects. Reading them straight
                // out of `tags` would offer every tag in the library on every
                // page, and picking one would then produce a query that matches
                // nothing.
                //
                // A label tag is a category and matches on its own name. A valued
                // tag names a property rather than a category, so it is its values
                // that are worth offering: searching "1963" finds the year, while
                // searching "Year" finds nothing useful.
                let tags = if query.tag.is_none() {
                    let mut statement = works::table
                        .left_join(work_persons::table)
                        .inner_join(
                            recordings::table
                                .left_join(recording_persons::table)
                                .left_join(recording_ensembles::table.left_join(
                                    ensembles::table.inner_join(ensemble_persons::table),
                                )),
                        )
                        .left_join(work_instruments::table)
                        .inner_join(work_tags::table.inner_join(tags::table))
                        .filter(
                            tags::takes_value
                                .eq(false)
                                .and(tags::name.like(&search))
                                .or(tags::takes_value
                                    .eq(true)
                                    .and(work_tags::value.like(&search))),
                        )
                        .into_boxed();

                    if let Some(person) = &query.composer {
                        statement = statement.filter(composer_condition(person));
                    }

                    if let Some(person) = &query.performer {
                        statement = statement.filter(
                            recording_persons::person_id
                                .eq(&person.person_id)
                                .or(ensemble_persons::person_id.eq(&person.person_id)),
                        );
                    }

                    if let Some(instrument) = &query.instrument {
                        statement = statement.filter(
                            work_instrument_condition(instrument)
                                .or(recording_persons::instrument_id.eq(&instrument.instrument_id))
                                .or(ensemble_persons::instrument_id.eq(&instrument.instrument_id)),
                        );
                    }

                    if let Some(ensemble) = &query.ensemble {
                        statement = statement
                            .filter(recording_ensembles::ensemble_id.eq(&ensemble.ensemble_id));
                    }

                    let mut found = statement
                        .order_by(tags::last_used_at.desc())
                        .limit(9)
                        .select((tables::Tag::as_select(), work_tags::value))
                        .distinct()
                        .load::<(Tag, Option<String>)>(connection)?;

                    let mut statement = recordings::table
                        .inner_join(
                            works::table
                                .left_join(work_persons::table)
                                .left_join(work_instruments::table),
                        )
                        .left_join(recording_persons::table)
                        .left_join(
                            recording_ensembles::table
                                .inner_join(ensembles::table.left_join(ensemble_persons::table)),
                        )
                        .inner_join(recording_tags::table.inner_join(tags::table))
                        .filter(
                            tags::takes_value
                                .eq(false)
                                .and(tags::name.like(&search))
                                .or(tags::takes_value
                                    .eq(true)
                                    .and(recording_tags::value.like(&search))),
                        )
                        .into_boxed();

                    if let Some(person) = &query.composer {
                        statement = statement.filter(composer_condition(person));
                    }

                    if let Some(person) = &query.performer {
                        statement = statement.filter(
                            recording_persons::person_id
                                .eq(&person.person_id)
                                .or(ensemble_persons::person_id.eq(&person.person_id)),
                        );
                    }

                    if let Some(instrument) = &query.instrument {
                        statement = statement.filter(
                            work_instrument_condition(instrument)
                                .or(recording_persons::instrument_id.eq(&instrument.instrument_id))
                                .or(ensemble_persons::instrument_id.eq(&instrument.instrument_id)),
                        );
                    }

                    if let Some(ensemble) = &query.ensemble {
                        statement = statement
                            .filter(recording_ensembles::ensemble_id.eq(&ensemble.ensemble_id));
                    }

                    found.extend(
                        statement
                            .order_by(tags::last_used_at.desc())
                            .limit(9)
                            .select((tables::Tag::as_select(), recording_tags::value))
                            .distinct()
                            .load::<(Tag, Option<String>)>(connection)?,
                    );

                    // The same tag can sit on a work and on its recordings, and the
                    // two statements cannot be deduplicated against each other in SQL.
                    let mut seen = HashSet::new();
                    let mut tags = Vec::new();
                    for (tag, value) in found {
                        if seen.insert((tag.tag_id.clone(), value.clone())) {
                            tags.push(TagValue { tag, value });
                        }
                    }

                    tags.truncate(9);
                    tags
                } else {
                    Vec::new()
                };

                LibraryResults {
                    composers,
                    performers,
                    ensembles,
                    instruments,
                    works,
                    recordings,
                    albums,
                    tags,
                    parent_work: None,
                    structure: Vec::new(),
                }
            }
            LibraryQuery {
                work: Some(work), ..
            } => {
                let mut statement = recordings::table
                    .filter(recordings::work_id.eq(&work.work_id))
                    .into_boxed();

                if let Some(tag) = &query.tag {
                    // For a recording the assignment on the recording itself is what counts,
                    // falling back to its work. Going through the work alone would match every
                    // sibling recording, which is wrong once a tag carries a value.
                    statement = statement.filter(
                        sql::<sql_types::Bool>(
                            "(EXISTS (SELECT 1 FROM recording_tags \
                              WHERE recording_tags.recording_id = recordings.recording_id \
                              AND recording_tags.tag_id = ",
                        )
                        .bind::<sql_types::Text, _>(tag.tag.tag_id.clone())
                        .sql(" AND (")
                        .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
                        .sql(" IS NULL OR recording_tags.value = ")
                        .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
                        .sql(
                            ")) OR EXISTS (SELECT 1 FROM work_tags \
                             WHERE work_tags.work_id = recordings.work_id \
                             AND work_tags.tag_id = ",
                        )
                        .bind::<sql_types::Text, _>(tag.tag.tag_id.clone())
                        .sql(" AND (")
                        .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
                        .sql(" IS NULL OR work_tags.value = ")
                        .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
                        .sql(")))"),
                    );
                }

                let recordings = statement
                    .order_by(
                        recording_last_played::table
                            .filter(
                                recording_last_played::recording_id.eq(recordings::recording_id),
                            )
                            .select(recording_last_played::last_played_at)
                            .single_value()
                            .desc(),
                    )
                    .load::<tables::Recording>(connection)?
                    .into_iter()
                    .map(|r| Recording::from_table(r, connection))
                    .collect::<Result<Vec<Recording>>>()?;

                // Related works in both directions: what this work was derived from, and
                // whatever else was derived from it (e.g. an arrangement alongside the
                // original, or siblings sharing an original).
                let mut works = Vec::new();

                if let Some(relates_to) = &work.relates_to {
                    works.push((**relates_to).clone());
                }

                works.extend(
                    works::table
                        .filter(works::relates_to.eq(&work.work_id))
                        .load::<tables::Work>(connection)?
                        .into_iter()
                        .map(|w| Work::from_table(w, connection))
                        .collect::<Result<Vec<Work>>>()?,
                );

                // Where the work sits in the part structure: its own movements if it
                // has any, or its siblings under the parent otherwise.
                let parent_work_id = works::table
                    .filter(works::work_id.eq(&work.work_id))
                    .select(works::parent_work_id)
                    .first::<Option<String>>(connection)?;

                let (parent_work, structure) = if !work.parts.is_empty() {
                    (None, work.parts.clone())
                } else if let Some(parent_work_id) = parent_work_id {
                    let parent = Work::from_table(
                        works::table
                            .filter(works::work_id.eq(&parent_work_id))
                            .first::<tables::Work>(connection)?,
                        connection,
                    )?;

                    let siblings = works::table
                        .filter(
                            works::parent_work_id
                                .eq(&parent_work_id)
                                .and(works::work_id.ne(&work.work_id)),
                        )
                        .order(works::sequence_number)
                        .load::<tables::Work>(connection)?
                        .into_iter()
                        .map(|w| Work::from_table(w, connection))
                        .collect::<Result<Vec<Work>>>()?;

                    (Some(parent), siblings)
                } else {
                    (None, Vec::new())
                };

                LibraryResults {
                    works,
                    recordings,
                    parent_work,
                    structure,
                    ..Default::default()
                }
            }
        })
    }

    pub fn tracks_for_recording(&self, recording_id: &str) -> Result<Vec<Track>> {
        let connection = &mut *self.conn();

        let tracks = tracks::table
            .order(tracks::recording_index)
            .filter(tracks::recording_id.eq(&recording_id))
            .select(tables::Track::as_select())
            .load::<tables::Track>(connection)?
            .into_iter()
            .map(|t| Track::from_table(t, connection))
            .collect::<Result<Vec<Track>>>()?;

        Ok(tracks)
    }

    pub fn search_persons(&self, search: &str) -> Result<Vec<SearchItem<Person>>> {
        let search = format!("%{}%", search);
        let connection = &mut *self.conn();

        let persons: Vec<Person> = persons::table
            .order(persons::last_used_at.desc())
            .filter(persons::name.like(&search))
            .limit(20)
            .load(connection)?;

        let mut results: Vec<SearchItem<Person>> = persons
            .into_iter()
            .map(|item| SearchItem {
                item,
                in_library: true,
            })
            .collect();

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

            for item in metadata_persons {
                if !existing.contains(&item.person_id) {
                    results.push(SearchItem {
                        item,
                        in_library: false,
                    });
                }
            }
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

        let mut results: Vec<SearchItem<Role>> = roles
            .into_iter()
            .map(|item| SearchItem {
                item,
                in_library: true,
            })
            .collect();

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

            for item in metadata_roles {
                if !existing.contains(&item.role_id) {
                    results.push(SearchItem {
                        item,
                        in_library: false,
                    });
                }
            }
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

        let mut results: Vec<SearchItem<Tag>> = tags
            .into_iter()
            .map(|item| SearchItem {
                item,
                in_library: true,
            })
            .collect();

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

            for item in metadata_tags {
                if !existing.contains(&item.tag_id) {
                    results.push(SearchItem {
                        item,
                        in_library: false,
                    });
                }
            }
        }

        Ok(results)
    }

    pub fn search_instruments(&self, search: &str) -> Result<Vec<SearchItem<Instrument>>> {
        let search = format!("%{}%", search);
        let connection = &mut *self.conn();

        let instruments: Vec<Instrument> = instruments::table
            .order(instruments::last_used_at.desc())
            .filter(instruments::name.like(&search))
            .limit(20)
            .load(connection)?;

        let mut results: Vec<SearchItem<Instrument>> = instruments
            .into_iter()
            .map(|item| SearchItem {
                item,
                in_library: true,
            })
            .collect();

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

            for item in metadata_instruments {
                if !existing.contains(&item.instrument_id) {
                    results.push(SearchItem {
                        item,
                        in_library: false,
                    });
                }
            }
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

            for work in metadata_works {
                if !existing.contains(&work.work_id) {
                    let item = Work::from_table(work, metadata_connection)?;
                    results.push(SearchItem {
                        item,
                        in_library: false,
                    });
                }
            }
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

            for recording in metadata_recordings {
                if !existing.contains(&recording.recording_id) {
                    let item = Recording::from_table(recording, metadata_connection)?;
                    results.push(SearchItem {
                        item,
                        in_library: false,
                    });
                }
            }
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

            for ensemble in metadata_ensembles {
                if !existing.contains(&ensemble.ensemble_id) {
                    let item = Ensemble::from_table(ensemble, metadata_connection)?;
                    results.push(SearchItem {
                        item,
                        in_library: false,
                    });
                }
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use tempfile::TempDir;

    use super::*;
    use crate::db::{models::Composer, TranslatedString};

    fn translated(name: &str) -> TranslatedString {
        let mut translations = HashMap::new();
        translations.insert("generic".to_string(), name.to_string());
        TranslatedString(translations)
    }

    fn library(dir: &TempDir, cache_dir: &TempDir) -> Library {
        Library::new(dir.path(), cache_dir.path()).unwrap()
    }

    /// Two recordings of one work, one tagged `Year: 1963` and the other
    /// `Year: 1964`, with the work itself tagged `Baroque`.
    fn tagged_library(library: &Library) -> (Tag, Tag, Recording, Recording) {
        let year = library
            .create_tag(translated("Year"), true, false, true)
            .unwrap();
        let baroque = library
            .create_tag(translated("Baroque"), false, false, true)
            .unwrap();

        let person = library.create_person(translated("Bach"), true).unwrap();

        let work = library
            .create_work(
                translated("Brandenburg Concerto No. 3"),
                Vec::new(),
                vec![Composer { person, role: None }],
                Vec::new(),
                vec![TagValue {
                    tag: baroque.clone(),
                    value: None,
                }],
                None,
                true,
            )
            .unwrap();

        let first = library
            .create_recording(
                work.clone(),
                Vec::new(),
                Vec::new(),
                vec![TagValue {
                    tag: year.clone(),
                    value: Some("1963".to_string()),
                }],
                true,
            )
            .unwrap();

        let second = library
            .create_recording(
                work,
                Vec::new(),
                Vec::new(),
                vec![TagValue {
                    tag: year.clone(),
                    value: Some("1964".to_string()),
                }],
                true,
            )
            .unwrap();

        (year, baroque, first, second)
    }

    /// A valued tag is offered as one facet per value, found by searching the
    /// value rather than the tag's name.
    #[test]
    fn searching_a_valued_tag_matches_its_values() {
        let dir = TempDir::new().unwrap();
        let cache_dir = TempDir::new().unwrap();
        let library = library(&dir, &cache_dir);
        let (year, _, _, _) = tagged_library(&library);

        let results = library.search(&LibraryQuery::default(), "1963").unwrap();
        assert_eq!(
            results.tags,
            vec![TagValue {
                tag: year.clone(),
                value: Some("1963".to_string()),
            }],
        );

        // The tag's own name is not what identifies the facet.
        let results = library.search(&LibraryQuery::default(), "Year").unwrap();
        assert!(
            results.tags.is_empty(),
            "a valued tag must not match its own name: {:?}",
            results.tags
        );
    }

    /// A label tag matches on its name and carries no value, and both kinds of
    /// tag come back from the same search.
    #[test]
    fn searching_a_label_tag_matches_its_name() {
        let dir = TempDir::new().unwrap();
        let cache_dir = TempDir::new().unwrap();
        let library = library(&dir, &cache_dir);
        let (_, baroque, _, _) = tagged_library(&library);

        let results = library.search(&LibraryQuery::default(), "Baroque").unwrap();
        assert_eq!(
            results.tags,
            vec![TagValue {
                tag: baroque,
                value: None,
            }],
        );
    }

    /// Filtering by a valued tag must not drag in sibling recordings of the same
    /// work that carry a different value.
    #[test]
    fn filtering_by_a_valued_tag_matches_only_that_value() {
        let dir = TempDir::new().unwrap();
        let cache_dir = TempDir::new().unwrap();
        let library = library(&dir, &cache_dir);
        let (year, baroque, first, second) = tagged_library(&library);

        let work = first.work.clone();

        let query = LibraryQuery {
            work: Some(work.clone()),
            tag: Some(TagValue {
                tag: year,
                value: Some("1963".to_string()),
            }),
            ..Default::default()
        };

        let results = library.search(&query, "").unwrap();
        assert_eq!(
            results.recordings.iter().collect::<Vec<_>>(),
            vec![&first],
            "only the 1963 recording carries that value"
        );

        // A label tag on the work applies to every recording of it.
        let query = LibraryQuery {
            work: Some(work),
            tag: Some(TagValue {
                tag: baroque,
                value: None,
            }),
            ..Default::default()
        };

        let results = library.search(&query, "").unwrap();
        assert_eq!(results.recordings.len(), 2);
        assert!(results.recordings.contains(&first));
        assert!(results.recordings.contains(&second));
    }
    /// A tag stays visible in the header once the query is narrowed further,
    /// rather than silently still applying.
    #[test]
    fn a_tag_stays_named_once_it_is_no_longer_the_title() {
        let dir = TempDir::new().unwrap();
        let cache_dir = TempDir::new().unwrap();
        let library = library(&dir, &cache_dir);
        let (year, baroque, first, _) = tagged_library(&library);

        // On its own, the tag is the title.
        let query = LibraryQuery {
            tag: Some(TagValue {
                tag: baroque.clone(),
                value: None,
            }),
            ..Default::default()
        };
        assert_eq!(query.title().as_deref(), Some("Baroque"));

        // Narrowed by a composer, the composer takes the title and the tag moves
        // into the description.
        let query = LibraryQuery {
            composer: Some(first.work.persons[0].person.clone()),
            tag: Some(TagValue {
                tag: baroque,
                value: None,
            }),
            ..Default::default()
        };
        assert_eq!(query.title().as_deref(), Some("Bach"));
        assert_eq!(query.description().as_deref(), Some("Tagged Baroque"));

        // A valued tag names its value too.
        let query = LibraryQuery {
            work: Some(first.work.clone()),
            tag: Some(TagValue {
                tag: year,
                value: Some("1963".to_string()),
            }),
            ..Default::default()
        };
        assert_eq!(
            query.description().as_deref(),
            Some("Bach, Tagged Year: 1963")
        );
    }

    /// A tag must only be offered on pages whose items actually carry it.
    /// Otherwise picking it produces a query that matches nothing, which shows
    /// an empty page and makes the play button fail.
    #[test]
    fn tags_are_scoped_to_the_rest_of_the_query() {
        let dir = TempDir::new().unwrap();
        let cache_dir = TempDir::new().unwrap();
        let library = library(&dir, &cache_dir);
        let (_, baroque, first, _) = tagged_library(&library);

        // A second composer, whose work carries no tag at all.
        let other_person = library.create_person(translated("Ligeti"), true).unwrap();
        let other_work = library
            .create_work(
                translated("Atmosphères"),
                Vec::new(),
                vec![Composer {
                    person: other_person.clone(),
                    role: None,
                }],
                Vec::new(),
                Vec::new(),
                None,
                true,
            )
            .unwrap();
        library
            .create_recording(other_work, Vec::new(), Vec::new(), Vec::new(), true)
            .unwrap();

        let bach = first.work.persons[0].person.clone();

        let results = library
            .search(
                &LibraryQuery {
                    composer: Some(bach),
                    ..Default::default()
                },
                "",
            )
            .unwrap();

        // The label on the work and both of its recordings' years.
        assert!(results.tags.contains(&TagValue {
            tag: baroque,
            value: None,
        }));
        assert!(results
            .tags
            .iter()
            .any(|t| t.value.as_deref() == Some("1963")));
        assert!(results
            .tags
            .iter()
            .any(|t| t.value.as_deref() == Some("1964")));
        assert_eq!(results.tags.len(), 3);

        let results = library
            .search(
                &LibraryQuery {
                    composer: Some(other_person),
                    ..Default::default()
                },
                "",
            )
            .unwrap();
        assert!(
            results.tags.is_empty(),
            "an untagged composer must offer no tags: {:?}",
            results.tags
        );
    }
}
