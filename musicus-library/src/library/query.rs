use std::collections::HashSet;

use anyhow::Result;
use diesel::{dsl::sql, prelude::*, sql_types, QueryDsl};

use gettextrs::gettext;

use super::Library;
use crate::{
    db::{models::*, schema::*, tables, views::*},
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
    pub parent_work: Option<Work>,
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
pub(crate) fn composer_condition<QS>(
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

/// True if the recording referenced by `recordings.recording_id`/`recordings.work_id`
/// in the enclosing query is "of" `work_id`: either directly (its own work is
/// `work_id` or one of `work_id`'s descendants, at any depth), or one of its tracks is
/// explicitly tagged (via `track_works`) with `work_id` or one of its descendants.
///
/// A recording of an *ancestor* of `work_id` does not count on its own — a recording
/// of the whole work does not necessarily include every one of its parts, so only a
/// track explicitly assigned to the part (or something within it) counts. See
/// [`crate::db::models::Work::contains`] and [`Player::recording_to_playlist_for_work`]
/// for the same rule applied once a recording has already been chosen.
pub(crate) fn recording_covers_work_condition<QS>(
    work_id: &str,
) -> impl Expression<SqlType = sql_types::Bool>
       + AppearsOnTable<QS>
       + diesel::expression::ValidGrouping<(), IsAggregate = diesel::expression::is_aggregate::Never>
       + diesel::query_builder::QueryFragment<diesel::sqlite::Sqlite> {
    const DESCENDANTS_OF_START: &str = "( \
        WITH RECURSIVE descendants(work_id) AS ( \
            SELECT ";
    const DESCENDANTS_OF_END: &str = " \
            UNION \
            SELECT works.work_id FROM works \
            JOIN descendants ON works.parent_work_id = descendants.work_id \
        ) SELECT work_id FROM descendants \
    )";

    sql::<sql_types::Bool>(&format!("(recordings.work_id IN {DESCENDANTS_OF_START}"))
        .bind::<sql_types::Text, _>(work_id.to_owned())
        .sql(&format!(
            "{DESCENDANTS_OF_END} \
              OR EXISTS ( \
                  SELECT 1 FROM tracks \
                  JOIN track_works ON track_works.track_id = tracks.track_id \
                  WHERE tracks.recording_id = recordings.recording_id \
                  AND track_works.work_id IN {DESCENDANTS_OF_START}"
        ))
        .bind::<sql_types::Text, _>(work_id.to_owned())
        .sql(&format!("{DESCENDANTS_OF_END} ))"))
}

impl Library {
    /// The work identified by `work_id`, with its parts loaded recursively.
    pub fn work(&self, work_id: &str) -> Result<Work> {
        let connection = &mut *self.conn();

        Work::from_table(
            works::table
                .filter(works::work_id.eq(work_id))
                .first::<tables::Work>(connection)?,
            connection,
        )
    }

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
                }
            }
            LibraryQuery {
                work: Some(work), ..
            } => {
                let mut statement = recordings::table
                    .filter(recording_covers_work_condition(&work.work_id))
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

                // Walk up to the root ancestor, this should be a single-digit
                // number, so it is easier than a recursive query.
                let mut root_id = work.work_id.clone();
                loop {
                    let parent_id = works::table
                        .filter(works::work_id.eq(&root_id))
                        .select(works::parent_work_id)
                        .first::<Option<String>>(connection)?;

                    match parent_id {
                        Some(parent_id) => root_id = parent_id,
                        None => break,
                    }
                }

                let parent_work = if root_id != work.work_id {
                    Some(Work::from_table(
                        works::table
                            .filter(works::work_id.eq(&root_id))
                            .first::<tables::Work>(connection)?,
                        connection,
                    )?)
                } else {
                    None
                };

                LibraryResults {
                    works,
                    recordings,
                    parent_work,
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
}
