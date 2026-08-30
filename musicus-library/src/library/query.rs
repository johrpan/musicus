use std::collections::HashSet;

use diesel::{dsl::sql, prelude::*, sql_types, QueryDsl};

use super::Library;
use crate::{
    db::{schema::*, tables, views::*, TranslatedString},
    error::{LibraryError, Result},
    library::load::Loader,
    models::*,
};

/// A single item that search results (or a search query) can be highlighted
/// with.
///
/// A [`Facet`] carries the full model because it is a display and selection
/// value, not a filter: filters in [`LibraryQuery`] use IDs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Facet {
    WorkPerson(Person),
    RecordingPerson(Person),
    Ensemble(Ensemble),
    Instrument(Instrument),
    Work(Work),
    Tag(TagValue),
}

/// A tag filter: the tag to match, plus the value to require if the tag takes
/// one. A value without this tag's ID is meaningless, so the two always travel
/// together.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TagFilter {
    pub tag_id: String,
    pub value: Option<TranslatedString>,
}

/// A query over the library's facets.
///
/// Relationship filters carry IDs, never loaded models, so that a query is a
/// pure description of the data it selects.
#[derive(Clone, Default, Debug)]
pub struct LibraryQuery {
    pub work_person: Option<String>,
    pub recording_person: Option<String>,
    pub ensemble: Option<String>,
    pub instrument: Option<String>,
    pub work: Option<String>,
    pub tag: Option<TagFilter>,
}

impl LibraryQuery {
    pub fn is_empty(&self) -> bool {
        self.work_person.is_none()
            && self.recording_person.is_none()
            && self.ensemble.is_none()
            && self.instrument.is_none()
            && self.work.is_none()
            && self.tag.is_none()
    }
}

#[derive(Default, Debug)]
pub struct LibraryResults {
    pub work_persons: Vec<Person>,
    pub recording_persons: Vec<Person>,
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
        self.work_persons.is_empty()
            && self.recording_persons.is_empty()
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

/// A tag matches a work if it is on the work itself (or an ancestor/descendant
/// of it, see [`in_work_subtree`]) or on any of the work's recordings.
///
/// A valued tag matches only that exact value; a label tag binds NULL, which
/// matches any assignment. This is the single definition of the
/// work-oriented tag predicate used by search; the recording-oriented
/// variant is [`recording_tag_condition`].
fn tag_condition<QS>(
    tag: &TagFilter,
) -> impl Expression<SqlType = sql_types::Bool>
       + AppearsOnTable<QS>
       + diesel::expression::ValidGrouping<(), IsAggregate = diesel::expression::is_aggregate::Never>
       + diesel::query_builder::QueryFragment<diesel::sqlite::Sqlite> {
    sql::<sql_types::Bool>(&format!(
        "(EXISTS (SELECT 1 FROM work_tags WHERE {} AND work_tags.tag_id = ",
        in_work_subtree("work_tags.work_id"),
    ))
    .bind::<sql_types::Text, _>(tag.tag_id.clone())
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
    .bind::<sql_types::Text, _>(tag.tag_id.clone())
    .sql(" AND (")
    .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
    .sql(" IS NULL OR recording_tags.value = ")
    .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
    .sql(")))")
}

/// A tag matches a recording if the assignment is on the recording itself or
/// on its (exact, not tree) work.
///
/// Going through the whole work tree would match every sibling recording,
/// which is wrong once a tag carries a value. This is the single definition of
/// the recording-oriented tag predicate, shared by search and program
/// generation; the work-oriented variant is [`tag_condition`].
fn recording_tag_condition<QS>(
    tag: &TagFilter,
) -> impl Expression<SqlType = sql_types::Bool>
       + AppearsOnTable<QS>
       + diesel::expression::ValidGrouping<(), IsAggregate = diesel::expression::is_aggregate::Never>
       + diesel::query_builder::QueryFragment<diesel::sqlite::Sqlite> {
    sql::<sql_types::Bool>(
        "(EXISTS (SELECT 1 FROM recording_tags \
          WHERE recording_tags.recording_id = recordings.recording_id \
          AND recording_tags.tag_id = ",
    )
    .bind::<sql_types::Text, _>(tag.tag_id.clone())
    .sql(" AND (")
    .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
    .sql(" IS NULL OR recording_tags.value = ")
    .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
    .sql(
        ")) OR EXISTS (SELECT 1 FROM work_tags \
         WHERE work_tags.work_id = recordings.work_id \
         AND work_tags.tag_id = ",
    )
    .bind::<sql_types::Text, _>(tag.tag_id.clone())
    .sql(" AND (")
    .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
    .sql(" IS NULL OR work_tags.value = ")
    .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
    .sql(")))")
}

/// Match a column of serialized [`TranslatedString`] JSON against a LIKE
/// pattern in any of its translated **values** — never against the JSON keys
/// or punctuation. Legacy plain-text values and NULL are checked directly, so
/// old rows keep matching and rows without a value never match.
pub(crate) fn translated_like<QS>(
    column: &str,
    pattern: &str,
) -> impl Expression<SqlType = sql_types::Bool>
       + AppearsOnTable<QS>
       + diesel::expression::ValidGrouping<(), IsAggregate = diesel::expression::is_aggregate::Never>
       + diesel::query_builder::QueryFragment<diesel::sqlite::Sqlite>
       + diesel::query_builder::QueryId {
    sql::<sql_types::Bool>(&format!(
        "(json_valid({column}) AND EXISTS (SELECT 1 FROM json_each({column}) AS je \
         WHERE je.value LIKE "
    ))
    .bind::<sql_types::Text, _>(pattern.to_owned())
    .sql(&format!(") OR NOT json_valid({column}) AND {column} LIKE "))
    .bind::<sql_types::Text, _>(pattern.to_owned())
    .sql(")")
}

/// True if `person_id` is credited on the work referenced by `works.work_id`
/// in the enclosing query, or an ancestor/descendant of it (see
/// [`in_work_subtree`]).
pub(crate) fn work_person_condition<QS>(
    person_id: &str,
) -> impl Expression<SqlType = sql_types::Bool>
       + AppearsOnTable<QS>
       + diesel::expression::ValidGrouping<(), IsAggregate = diesel::expression::is_aggregate::Never>
       + diesel::query_builder::QueryFragment<diesel::sqlite::Sqlite> {
    sql::<sql_types::Bool>("EXISTS (SELECT 1 FROM work_persons WHERE work_persons.person_id = ")
        .bind::<sql_types::Text, _>(person_id.to_owned())
        .sql(&format!(
            " AND {})",
            in_work_subtree("work_persons.work_id")
        ))
}

/// True if `person_id` performs in the recording referenced by
/// `recordings.recording_id` in the enclosing query, directly or as a member
/// of one of its performing ensembles.
pub(crate) fn recording_person_condition<QS>(
    person_id: &str,
) -> impl Expression<SqlType = sql_types::Bool>
       + AppearsOnTable<QS>
       + diesel::expression::ValidGrouping<(), IsAggregate = diesel::expression::is_aggregate::Never>
       + diesel::query_builder::QueryFragment<diesel::sqlite::Sqlite> {
    sql::<sql_types::Bool>("recording_persons.person_id = ")
        .bind::<sql_types::Text, _>(person_id.to_owned())
        .sql(" OR ensemble_persons.person_id = ")
        .bind::<sql_types::Text, _>(person_id.to_owned())
}

/// True if `instrument_id` is played on the recording referenced by
/// `recordings.recording_id` in the enclosing query, directly or as a member
/// of one of its performing ensembles.
pub(crate) fn recording_person_instrument_condition<QS>(
    instrument_id: &str,
) -> impl Expression<SqlType = sql_types::Bool>
       + AppearsOnTable<QS>
       + diesel::expression::ValidGrouping<(), IsAggregate = diesel::expression::is_aggregate::Never>
       + diesel::query_builder::QueryFragment<diesel::sqlite::Sqlite> {
    sql::<sql_types::Bool>("recording_persons.instrument_id = ")
        .bind::<sql_types::Text, _>(instrument_id.to_owned())
        .sql(" OR ensemble_persons.instrument_id = ")
        .bind::<sql_types::Text, _>(instrument_id.to_owned())
}

/// True if `instrument_id` is included in the work referenced by
/// `works.work_id` in the enclosing query, or an ancestor/descendant of it (see
/// [`in_work_subtree`]).
pub(crate) fn work_instrument_condition<QS>(
    instrument_id: &str,
) -> impl Expression<SqlType = sql_types::Bool>
       + AppearsOnTable<QS>
       + diesel::expression::ValidGrouping<(), IsAggregate = diesel::expression::is_aggregate::Never>
       + diesel::query_builder::QueryFragment<diesel::sqlite::Sqlite> {
    sql::<sql_types::Bool>(
        "EXISTS (SELECT 1 FROM work_instruments WHERE work_instruments.instrument_id = ",
    )
    .bind::<sql_types::Text, _>(instrument_id.to_owned())
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
/// track explicitly assigned to the part (or something within it) counts.
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

/// True if the recording referenced by `recordings.recording_id` in the
/// enclosing query is a recording of a work directly related to `work_id` (in
/// either direction). Used for program generation, where a program about a
/// work also matches its arrangements.
pub(crate) fn recorded_work_related_condition<QS>(
    work_id: &str,
) -> impl Expression<SqlType = sql_types::Bool>
       + AppearsOnTable<QS>
       + diesel::expression::ValidGrouping<(), IsAggregate = diesel::expression::is_aggregate::Never>
       + diesel::query_builder::QueryFragment<diesel::sqlite::Sqlite> {
    sql::<sql_types::Bool>(
        "EXISTS (SELECT 1 FROM work_relations \
          WHERE work_relations.work_id = recordings.work_id \
          AND work_relations.related_work_id = ",
    )
    .bind::<sql_types::Text, _>(work_id.to_owned())
    .sql(
        ") OR EXISTS (SELECT 1 FROM work_relations \
         WHERE work_relations.related_work_id = recordings.work_id \
         AND work_relations.work_id = ",
    )
    .bind::<sql_types::Text, _>(work_id.to_owned())
    .sql("))")
}

/// Whether `person_id` is credited on the work of the recording referenced by
/// `recordings.work_id` in the enclosing query. One shared definition of
/// work-person membership, used by program generation.
pub(crate) fn work_person_exists<QS>(
    person_id: &str,
) -> impl Expression<SqlType = sql_types::Bool>
       + AppearsOnTable<QS>
       + diesel::expression::ValidGrouping<(), IsAggregate = diesel::expression::is_aggregate::Never>
       + diesel::query_builder::QueryFragment<diesel::sqlite::Sqlite> {
    sql::<sql_types::Bool>(
        "EXISTS (SELECT 1 FROM work_persons \
         WHERE work_persons.work_id = recordings.work_id \
         AND work_persons.person_id = ",
    )
    .bind::<sql_types::Text, _>(person_id.to_owned())
    .sql(")")
}

/// Whether `person_id` performs on the recording referenced by
/// `recordings.recording_id` in the enclosing query, directly or as a member
/// of one of its ensembles. One shared definition of recording-person
/// membership, used by program generation.
pub(crate) fn recording_person_membership<QS>(
    person_id: &str,
) -> impl Expression<SqlType = sql_types::Bool>
       + AppearsOnTable<QS>
       + diesel::expression::ValidGrouping<(), IsAggregate = diesel::expression::is_aggregate::Never>
       + diesel::query_builder::QueryFragment<diesel::sqlite::Sqlite> {
    sql::<sql_types::Bool>(
        "EXISTS (SELECT 1 FROM recording_persons \
         WHERE recording_persons.recording_id = recordings.recording_id \
         AND recording_persons.person_id = ",
    )
    .bind::<sql_types::Text, _>(person_id.to_owned())
    .sql(
        ") OR EXISTS (SELECT 1 FROM recording_ensembles \
         JOIN ensemble_persons ON ensemble_persons.ensemble_id = recording_ensembles.ensemble_id \
         WHERE recording_ensembles.recording_id = recordings.recording_id \
         AND ensemble_persons.person_id = ",
    )
    .bind::<sql_types::Text, _>(person_id.to_owned())
    .sql(")")
}

/// Whether `instrument_id` is played on the recording referenced by
/// `recordings.recording_id` in the enclosing query: in its work's
/// instrumentation, by one of its performing persons, or through a performing
/// ensemble's members. One shared definition of instrument membership, used by
/// program generation.
pub(crate) fn recording_instrument_membership<QS>(
    instrument_id: &str,
) -> impl Expression<SqlType = sql_types::Bool>
       + AppearsOnTable<QS>
       + diesel::expression::ValidGrouping<(), IsAggregate = diesel::expression::is_aggregate::Never>
       + diesel::query_builder::QueryFragment<diesel::sqlite::Sqlite> {
    sql::<sql_types::Bool>(
        "EXISTS (SELECT 1 FROM work_instruments \
         WHERE work_instruments.work_id = recordings.work_id \
         AND work_instruments.instrument_id = ",
    )
    .bind::<sql_types::Text, _>(instrument_id.to_owned())
    .sql(
        ") OR EXISTS (SELECT 1 FROM recording_persons \
         WHERE recording_persons.recording_id = recordings.recording_id \
         AND recording_persons.instrument_id = ",
    )
    .bind::<sql_types::Text, _>(instrument_id.to_owned())
    .sql(
        ") OR EXISTS (SELECT 1 FROM recording_ensembles \
         JOIN ensemble_persons ON ensemble_persons.ensemble_id = recording_ensembles.ensemble_id \
         WHERE recording_ensembles.recording_id = recordings.recording_id \
         AND ensemble_persons.instrument_id = ",
    )
    .bind::<sql_types::Text, _>(instrument_id.to_owned())
    .sql(")")
}

/// Whether the recording referenced by `recordings.recording_id` in the
/// enclosing query is tagged with `tag`, on the recording or on its exact
/// work. One shared definition of recording tag matching, used by program
/// generation.
pub(crate) fn recording_tag_membership<QS>(
    tag: &TagFilter,
) -> impl Expression<SqlType = sql_types::Bool>
       + AppearsOnTable<QS>
       + diesel::expression::ValidGrouping<(), IsAggregate = diesel::expression::is_aggregate::Never>
       + diesel::query_builder::QueryFragment<diesel::sqlite::Sqlite> {
    sql::<sql_types::Bool>(
        "(EXISTS (SELECT 1 FROM recording_tags \
          WHERE recording_tags.recording_id = recordings.recording_id \
          AND recording_tags.tag_id = ",
    )
    .bind::<sql_types::Text, _>(tag.tag_id.clone())
    .sql(" AND (")
    .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
    .sql(" IS NULL OR recording_tags.value = ")
    .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
    .sql(
        ")) OR EXISTS (SELECT 1 FROM work_tags \
         WHERE work_tags.work_id = recordings.work_id \
         AND work_tags.tag_id = ",
    )
    .bind::<sql_types::Text, _>(tag.tag_id.clone())
    .sql(" AND (")
    .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
    .sql(" IS NULL OR work_tags.value = ")
    .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
    .sql(")))")
}

impl Library {
    /// The work identified by `work_id`, with its parts loaded recursively.
    pub fn work(&self, work_id: &str) -> Result<Work> {
        let connection = &mut *self.conn();

        let loader = Loader::load(connection)?;
        Ok(loader.work(work_id)?)
    }

    /// The person identified by `id`.
    pub fn person(&self, id: &str) -> Result<Person> {
        let connection = &mut *self.conn();
        Ok(Loader::load(connection)?.person(id)?)
    }

    /// The role identified by `id`.
    pub fn role(&self, id: &str) -> Result<Role> {
        let connection = &mut *self.conn();
        Ok(Loader::load(connection)?.role(id)?)
    }

    /// The instrument identified by `id`.
    pub fn instrument(&self, id: &str) -> Result<Instrument> {
        let connection = &mut *self.conn();
        Ok(Loader::load(connection)?.instrument(id)?)
    }

    /// The tag identified by `id`.
    pub fn tag(&self, id: &str) -> Result<Tag> {
        let connection = &mut *self.conn();
        Ok(Loader::load(connection)?.tag(id)?)
    }

    /// The ensemble identified by `id`, with its members.
    pub fn ensemble(&self, id: &str) -> Result<Ensemble> {
        let connection = &mut *self.conn();
        Ok(Loader::load(connection)?.ensemble(id)?)
    }

    /// Search the library for the `limit` most recently used items matching
    /// `query`, skipping the first `offset`.
    pub fn search(
        &self,
        query: &LibraryQuery,
        search: &str,
        limit: usize,
        offset: usize,
    ) -> Result<LibraryResults> {
        let search = format!("%{}%", search);
        let connection = &mut *self.conn();
        let loader = Loader::load(connection)?;

        let limit = i64::try_from(limit).map_err(|_| {
            LibraryError::InvalidInput(format!("Search limit {limit} is too large"))
        })?;
        let offset = i64::try_from(offset).map_err(|_| {
            LibraryError::InvalidInput(format!("Search offset {offset} is too large"))
        })?;

        Ok(match query {
            LibraryQuery { work: None, .. } => {
                let work_persons = if query.work_person.is_none() {
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
                        .filter(translated_like("persons.name", &search))
                        .into_boxed();

                    if let Some(person_id) = &query.recording_person {
                        statement = statement.filter(recording_person_condition(person_id));
                    }

                    if let Some(ensemble_id) = &query.ensemble {
                        statement =
                            statement.filter(recording_ensembles::ensemble_id.eq(ensemble_id));
                    }

                    if let Some(instrument_id) = &query.instrument {
                        statement = statement.filter(
                            work_instrument_condition(instrument_id)
                                .or(recording_person_instrument_condition(instrument_id)),
                        );
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
                        .then_order_by(persons::person_id)
                        .limit(limit)
                        .offset(offset)
                        .select(persons::all_columns)
                        .distinct()
                        .load::<tables::Person>(connection)?
                        .into_iter()
                        .map(Person::from)
                        .collect::<Vec<Person>>()
                } else {
                    Vec::new()
                };

                let recording_persons = if query.recording_person.is_none() {
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
                        .filter(translated_like("persons.name", &search))
                        .into_boxed();

                    if let Some(person_id) = &query.work_person {
                        statement = statement.filter(work_person_condition(person_id));
                    }

                    if let Some(ensemble_id) = &query.ensemble {
                        statement =
                            statement.filter(recording_ensembles::ensemble_id.eq(ensemble_id));
                    }

                    if let Some(instrument_id) = &query.instrument {
                        statement = statement.filter(
                            work_instrument_condition(instrument_id)
                                .or(recording_person_instrument_condition(instrument_id)),
                        );
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
                        .then_order_by(persons::person_id)
                        .limit(limit)
                        .offset(offset)
                        .select(persons::all_columns)
                        .distinct()
                        .load::<tables::Person>(connection)?
                        .into_iter()
                        .map(Person::from)
                        .collect::<Vec<Person>>()
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
                            translated_like("ensembles.name", &search)
                                .or(translated_like("persons.name", &search)),
                        )
                        .into_boxed();

                    if let Some(person_id) = &query.work_person {
                        statement = statement.filter(work_person_condition(person_id));
                    }

                    if let Some(person_id) = &query.recording_person {
                        statement = statement.filter(recording_person_condition(person_id));
                    }

                    if let Some(instrument_id) = &query.instrument {
                        statement = statement.filter(
                            work_instrument_condition(instrument_id)
                                .or(recording_person_instrument_condition(instrument_id)),
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
                        .then_order_by(ensembles::ensemble_id)
                        .limit(limit)
                        .offset(offset)
                        .select(ensembles::all_columns)
                        .distinct()
                        .load::<tables::Ensemble>(connection)?
                        .into_iter()
                        .map(|e| loader.ensemble(&e.ensemble_id))
                        .collect::<anyhow::Result<Vec<Ensemble>>>()?
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
                        .filter(translated_like("instruments.name", &search))
                        .into_boxed();

                    if let Some(person_id) = &query.work_person {
                        statement = statement.filter(work_person_condition(person_id));
                    }

                    if let Some(person_id) = &query.recording_person {
                        statement = statement.filter(recording_person_condition(person_id));
                    }

                    if let Some(ensemble_id) = &query.ensemble {
                        statement = statement.filter(ensemble_persons::ensemble_id.eq(ensemble_id));
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
                        .then_order_by(instruments::instrument_id)
                        .limit(limit)
                        .offset(offset)
                        .select(instruments::all_columns)
                        .distinct()
                        .load::<tables::Instrument>(connection)?
                        .into_iter()
                        .map(Instrument::from)
                        .collect::<Vec<Instrument>>()
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
                        .filter(translated_like("works.name", &search))
                        .into_boxed();

                    if let Some(person_id) = &query.work_person {
                        statement = statement.filter(work_person_condition(person_id));
                    }

                    if let Some(person_id) = &query.recording_person {
                        statement = statement.filter(recording_person_condition(person_id));
                    }

                    if let Some(instrument_id) = &query.instrument {
                        statement = statement.filter(
                            work_instrument_condition(instrument_id)
                                .or(recording_person_instrument_condition(instrument_id)),
                        );
                    }

                    if let Some(ensemble_id) = &query.ensemble {
                        statement =
                            statement.filter(recording_ensembles::ensemble_id.eq(ensemble_id));
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
                        .then_order_by(works::work_id)
                        .limit(limit)
                        .offset(offset)
                        .select(works::all_columns)
                        .distinct()
                        .load::<tables::Work>(connection)?
                        .into_iter()
                        .map(|w| loader.work(&w.work_id))
                        .collect::<anyhow::Result<Vec<Work>>>()?
                } else {
                    Vec::new()
                };

                // Only search recordings in special cases. Works will always be searched and
                // directly lead to recordings. The special case of a work in the query is already
                // handled in another branch of the top-level match expression.
                let recordings = if query.recording_person.is_some() || query.ensemble.is_some() {
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
                        .filter(translated_like("works.name", &search))
                        .into_boxed();

                    if let Some(person_id) = &query.work_person {
                        statement = statement.filter(work_person_condition(person_id));
                    }

                    if let Some(person_id) = &query.recording_person {
                        statement = statement.filter(recording_person_condition(person_id));
                    }

                    if let Some(instrument_id) = &query.instrument {
                        statement = statement.filter(
                            work_instrument_condition(instrument_id)
                                .or(recording_person_instrument_condition(instrument_id)),
                        );
                    }

                    if let Some(ensemble_id) = &query.ensemble {
                        statement =
                            statement.filter(recording_ensembles::ensemble_id.eq(ensemble_id));
                    }

                    if let Some(tag) = &query.tag {
                        statement = statement.filter(recording_tag_condition(tag));
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
                        .then_order_by(recordings::recording_id)
                        .limit(limit)
                        .offset(offset)
                        .select(recordings::all_columns)
                        .distinct()
                        .load::<tables::Recording>(connection)?
                        .into_iter()
                        .map(|r| loader.recording(&r.recording_id))
                        .collect::<anyhow::Result<Vec<Recording>>>()?
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
                    .filter(translated_like("albums.name", &search))
                    .into_boxed();

                if let Some(person_id) = &query.work_person {
                    statement = statement.filter(work_person_condition(person_id));
                }

                if let Some(person_id) = &query.recording_person {
                    statement = statement.filter(recording_person_condition(person_id));
                }

                if let Some(instrument_id) = &query.instrument {
                    statement = statement.filter(
                        work_instrument_condition(instrument_id)
                            .or(recording_person_instrument_condition(instrument_id)),
                    );
                }

                if let Some(ensemble_id) = &query.ensemble {
                    statement = statement.filter(recording_ensembles::ensemble_id.eq(ensemble_id));
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
                    .then_order_by(albums::album_id)
                    .limit(limit)
                    .offset(offset)
                    .select(albums::all_columns)
                    .distinct()
                    .load::<tables::Album>(connection)?
                    .into_iter()
                    .map(|r| loader.album(&r.album_id))
                    .collect::<anyhow::Result<Vec<Album>>>()?;

                // Tags are facets like any other, so they have to come from items
                // the rest of the query actually selects. A label tag is a category
                // and matches on its own name; a valued tag names a property, so it
                // is its values that are worth offering.
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
                                .and(translated_like("tags.name", &search))
                                .or(tags::takes_value
                                    .eq(true)
                                    .and(translated_like("work_tags.value", &search))),
                        )
                        .into_boxed();

                    if let Some(person_id) = &query.work_person {
                        statement = statement.filter(work_person_condition(person_id));
                    }

                    if let Some(person_id) = &query.recording_person {
                        statement = statement.filter(recording_person_condition(person_id));
                    }

                    if let Some(instrument_id) = &query.instrument {
                        statement = statement.filter(
                            work_instrument_condition(instrument_id)
                                .or(recording_person_instrument_condition(instrument_id)),
                        );
                    }

                    if let Some(ensemble_id) = &query.ensemble {
                        statement =
                            statement.filter(recording_ensembles::ensemble_id.eq(ensemble_id));
                    }

                    let mut found = statement
                        .order_by(tags::last_used_at.desc())
                        .then_order_by(tags::tag_id)
                        .limit(limit)
                        .offset(offset)
                        .select((tables::Tag::as_select(), work_tags::value))
                        .distinct()
                        .load::<(tables::Tag, Option<TranslatedString>)>(connection)?;

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
                                .and(translated_like("tags.name", &search))
                                .or(tags::takes_value
                                    .eq(true)
                                    .and(translated_like("recording_tags.value", &search))),
                        )
                        .into_boxed();

                    if let Some(person_id) = &query.work_person {
                        statement = statement.filter(work_person_condition(person_id));
                    }

                    if let Some(person_id) = &query.recording_person {
                        statement = statement.filter(recording_person_condition(person_id));
                    }

                    if let Some(instrument_id) = &query.instrument {
                        statement = statement.filter(
                            work_instrument_condition(instrument_id)
                                .or(recording_person_instrument_condition(instrument_id)),
                        );
                    }

                    if let Some(ensemble_id) = &query.ensemble {
                        statement =
                            statement.filter(recording_ensembles::ensemble_id.eq(ensemble_id));
                    }

                    found.extend(
                        statement
                            .order_by(tags::last_used_at.desc())
                            .then_order_by(tags::tag_id)
                            .limit(limit)
                            .offset(offset)
                            .select((tables::Tag::as_select(), recording_tags::value))
                            .distinct()
                            .load::<(tables::Tag, Option<TranslatedString>)>(connection)?,
                    );

                    // The same tag can sit on a work and on its recordings, and the
                    // two statements cannot be deduplicated against each other in SQL.
                    let mut seen = HashSet::new();
                    let mut tags = Vec::new();
                    for (tag, value) in found {
                        if seen.insert((tag.tag_id.clone(), value.clone())) {
                            tags.push(TagValue {
                                tag: tag.into(),
                                value,
                            });
                        }
                    }

                    tags.truncate(limit as usize);
                    tags
                } else {
                    Vec::new()
                };

                LibraryResults {
                    work_persons,
                    recording_persons,
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
                work: Some(work_id),
                ..
            } => {
                let mut statement = recordings::table
                    .filter(recording_covers_work_condition(work_id))
                    .into_boxed();

                if let Some(tag) = &query.tag {
                    statement = statement.filter(recording_tag_condition(tag));
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
                    .then_order_by(recordings::recording_id)
                    .limit(limit)
                    .offset(offset)
                    .load::<tables::Recording>(connection)?
                    .into_iter()
                    .map(|r| loader.recording(&r.recording_id))
                    .collect::<anyhow::Result<Vec<Recording>>>()?;

                // Related works in both directions: what this work was derived from, and
                // whatever else was derived from it. The relation graph may cycle, so
                // the references are loaded one level at a time.
                let mut works = Vec::new();

                let outgoing_ids = work_relations::table
                    .filter(work_relations::work_id.eq(work_id))
                    .order(work_relations::related_work_id)
                    .limit(limit)
                    .offset(offset)
                    .select(work_relations::related_work_id)
                    .load::<String>(connection)?;

                for related_work_id in outgoing_ids {
                    works.push(loader.work(&related_work_id)?);
                }

                let remaining = (limit - works.len() as i64).max(0);
                let incoming_ids = work_relations::table
                    .filter(work_relations::related_work_id.eq(work_id))
                    .order(work_relations::work_id)
                    .limit(remaining)
                    .select(work_relations::work_id)
                    .load::<String>(connection)?;

                // Skip works already listed through their outgoing relation.
                let mut seen = works.iter().map(|w| w.id.clone()).collect::<HashSet<_>>();
                for related_work_id in incoming_ids {
                    if seen.insert(related_work_id.clone()) {
                        works.push(loader.work(&related_work_id)?);
                    }
                    if works.len() >= limit as usize {
                        break;
                    }
                }

                // Walk up to the root ancestor, this should be a single-digit
                // number, so it is easier than a recursive query.
                let mut root_id = work_id.clone();
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

                let parent_work = if root_id != *work_id {
                    Some(loader.work(&root_id)?)
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

        let loader = Loader::load(connection)?;
        Ok(loader.tracks_of_recording(recording_id)?)
    }
}
