use std::collections::HashSet;

use anyhow::Result;
use diesel::{
    dsl::{exists, sql},
    prelude::*,
    sql_types, QueryDsl,
};

use gettextrs::gettext;

use super::{metadata::SearchItem, Library};
use crate::{
    db::{self, models::*, schema::*, tables},
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

/// Parameters for [`Library::generate_recording`], describing a playback "program".
#[derive(Clone, Default, Debug)]
pub struct GenerateRecordingParams {
    pub composer_id: Option<String>,
    pub performer_id: Option<String>,
    pub ensemble_id: Option<String>,
    pub instrument_id: Option<String>,
    pub work_id: Option<String>,
    pub album_id: Option<String>,
    pub tag_id: Option<String>,
    pub tag_value: Option<String>,
    pub prefer_recently_added: f64,
    pub prefer_least_recently_played: f64,
    pub avoid_repeated_composers: i32,
    pub avoid_repeated_instruments: i32,
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
                        statement = statement.filter(
                            work_instruments::instrument_id
                                .eq(&instrument.instrument_id)
                                .or(recording_persons::instrument_id.eq(&instrument.instrument_id)),
                        );
                    }

                    if let Some(tag) = &query.tag {
                        // A tag matches if it is on the work itself or on any of its recordings.
                        // Written out because the subquery needs its own alias for `recordings`,
                        // which is already in the outer query. A valued tag matches only that
                        // exact value; a label tag binds NULL, which matches any assignment.
                        statement = statement.filter(
                            sql::<sql_types::Bool>(
                                "(EXISTS (SELECT 1 FROM work_tags \
                                  WHERE work_tags.work_id = works.work_id AND work_tags.tag_id = ",
                            )
                            .bind::<sql_types::Text, _>(tag.tag.tag_id.clone())
                            .sql(" AND (")
                            .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
                            .sql(" IS NULL OR work_tags.value = ")
                            .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
                            .sql(
                                ")) OR EXISTS (SELECT 1 FROM recording_tags \
                                 JOIN recordings AS tagged_recordings \
                                 ON tagged_recordings.recording_id = recording_tags.recording_id \
                                 WHERE tagged_recordings.work_id = works.work_id \
                                 AND recording_tags.tag_id = ",
                            )
                            .bind::<sql_types::Text, _>(tag.tag.tag_id.clone())
                            .sql(" AND (")
                            .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
                            .sql(" IS NULL OR recording_tags.value = ")
                            .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
                            .sql(")))"),
                        );
                    }

                    statement
                        .order_by(persons::last_played_at.desc())
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
                        statement = statement.filter(work_persons::person_id.eq(&person.person_id));
                    }

                    if let Some(ensemble) = &query.ensemble {
                        statement = statement
                            .filter(recording_ensembles::ensemble_id.eq(&ensemble.ensemble_id));
                    }

                    if let Some(instrument) = &query.instrument {
                        statement = statement.filter(
                            work_instruments::instrument_id
                                .eq(&instrument.instrument_id)
                                .or(recording_persons::instrument_id.eq(&instrument.instrument_id)),
                        );
                    }

                    if let Some(tag) = &query.tag {
                        // A tag matches if it is on the work itself or on any of its recordings.
                        // Written out because the subquery needs its own alias for `recordings`,
                        // which is already in the outer query. A valued tag matches only that
                        // exact value; a label tag binds NULL, which matches any assignment.
                        statement = statement.filter(
                            sql::<sql_types::Bool>(
                                "(EXISTS (SELECT 1 FROM work_tags \
                                  WHERE work_tags.work_id = works.work_id AND work_tags.tag_id = ",
                            )
                            .bind::<sql_types::Text, _>(tag.tag.tag_id.clone())
                            .sql(" AND (")
                            .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
                            .sql(" IS NULL OR work_tags.value = ")
                            .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
                            .sql(
                                ")) OR EXISTS (SELECT 1 FROM recording_tags \
                                 JOIN recordings AS tagged_recordings \
                                 ON tagged_recordings.recording_id = recording_tags.recording_id \
                                 WHERE tagged_recordings.work_id = works.work_id \
                                 AND recording_tags.tag_id = ",
                            )
                            .bind::<sql_types::Text, _>(tag.tag.tag_id.clone())
                            .sql(" AND (")
                            .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
                            .sql(" IS NULL OR recording_tags.value = ")
                            .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
                            .sql(")))"),
                        );
                    }

                    statement
                        .order_by(persons::last_played_at.desc())
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
                        statement = statement.filter(work_persons::person_id.eq(&person.person_id));
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
                            work_instruments::instrument_id
                                .eq(&instrument.instrument_id)
                                .or(ensemble_persons::instrument_id.eq(&instrument.instrument_id)),
                        );
                    }

                    if let Some(tag) = &query.tag {
                        // A tag matches if it is on the work itself or on any of its recordings.
                        // Written out because the subquery needs its own alias for `recordings`,
                        // which is already in the outer query. A valued tag matches only that
                        // exact value; a label tag binds NULL, which matches any assignment.
                        statement = statement.filter(
                            sql::<sql_types::Bool>(
                                "(EXISTS (SELECT 1 FROM work_tags \
                                  WHERE work_tags.work_id = works.work_id AND work_tags.tag_id = ",
                            )
                            .bind::<sql_types::Text, _>(tag.tag.tag_id.clone())
                            .sql(" AND (")
                            .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
                            .sql(" IS NULL OR work_tags.value = ")
                            .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
                            .sql(
                                ")) OR EXISTS (SELECT 1 FROM recording_tags \
                                 JOIN recordings AS tagged_recordings \
                                 ON tagged_recordings.recording_id = recording_tags.recording_id \
                                 WHERE tagged_recordings.work_id = works.work_id \
                                 AND recording_tags.tag_id = ",
                            )
                            .bind::<sql_types::Text, _>(tag.tag.tag_id.clone())
                            .sql(" AND (")
                            .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
                            .sql(" IS NULL OR recording_tags.value = ")
                            .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
                            .sql(")))"),
                        );
                    }

                    statement
                        .order_by(ensembles::last_played_at.desc())
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
                            work_instruments::table
                                .inner_join(works::table.left_join(work_persons::table)),
                        )
                        .left_join(recording_persons::table)
                        .left_join(ensemble_persons::table)
                        .filter(instruments::name.like(&search))
                        .into_boxed();

                    if let Some(person) = &query.composer {
                        statement = statement.filter(work_persons::person_id.eq(&person.person_id));
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
                        // A tag matches if it is on the work itself or on any of its recordings.
                        // Written out because the subquery needs its own alias for `recordings`,
                        // which is already in the outer query. A valued tag matches only that
                        // exact value; a label tag binds NULL, which matches any assignment.
                        statement = statement.filter(
                            sql::<sql_types::Bool>(
                                "(EXISTS (SELECT 1 FROM work_tags \
                                  WHERE work_tags.work_id = works.work_id AND work_tags.tag_id = ",
                            )
                            .bind::<sql_types::Text, _>(tag.tag.tag_id.clone())
                            .sql(" AND (")
                            .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
                            .sql(" IS NULL OR work_tags.value = ")
                            .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
                            .sql(
                                ")) OR EXISTS (SELECT 1 FROM recording_tags \
                                 JOIN recordings AS tagged_recordings \
                                 ON tagged_recordings.recording_id = recording_tags.recording_id \
                                 WHERE tagged_recordings.work_id = works.work_id \
                                 AND recording_tags.tag_id = ",
                            )
                            .bind::<sql_types::Text, _>(tag.tag.tag_id.clone())
                            .sql(" AND (")
                            .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
                            .sql(" IS NULL OR recording_tags.value = ")
                            .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
                            .sql(")))"),
                        );
                    }

                    statement
                        .order_by(instruments::last_played_at.desc())
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
                        statement = statement.filter(work_persons::person_id.eq(&person.person_id));
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
                            work_instruments::instrument_id
                                .eq(&instrument.instrument_id)
                                .or(recording_persons::instrument_id.eq(&instrument.instrument_id))
                                .or(ensemble_persons::instrument_id.eq(&instrument.instrument_id)),
                        );
                    }

                    if let Some(ensemble) = &query.ensemble {
                        statement = statement
                            .filter(recording_ensembles::ensemble_id.eq(&ensemble.ensemble_id));
                    }

                    if let Some(tag) = &query.tag {
                        // A tag matches if it is on the work itself or on any of its recordings.
                        // Written out because the subquery needs its own alias for `recordings`,
                        // which is already in the outer query. A valued tag matches only that
                        // exact value; a label tag binds NULL, which matches any assignment.
                        statement = statement.filter(
                            sql::<sql_types::Bool>(
                                "(EXISTS (SELECT 1 FROM work_tags \
                                  WHERE work_tags.work_id = works.work_id AND work_tags.tag_id = ",
                            )
                            .bind::<sql_types::Text, _>(tag.tag.tag_id.clone())
                            .sql(" AND (")
                            .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
                            .sql(" IS NULL OR work_tags.value = ")
                            .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
                            .sql(
                                ")) OR EXISTS (SELECT 1 FROM recording_tags \
                                 JOIN recordings AS tagged_recordings \
                                 ON tagged_recordings.recording_id = recording_tags.recording_id \
                                 WHERE tagged_recordings.work_id = works.work_id \
                                 AND recording_tags.tag_id = ",
                            )
                            .bind::<sql_types::Text, _>(tag.tag.tag_id.clone())
                            .sql(" AND (")
                            .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
                            .sql(" IS NULL OR recording_tags.value = ")
                            .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
                            .sql(")))"),
                        );
                    }

                    statement
                        .order_by(works::last_played_at.desc())
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
                        statement = statement.filter(work_persons::person_id.eq(&person.person_id));
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
                            work_instruments::instrument_id
                                .eq(&instrument.instrument_id)
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
                        .order_by(recordings::last_played_at.desc())
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
                    statement = statement.filter(work_persons::person_id.eq(&person.person_id));
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
                        work_instruments::instrument_id
                            .eq(&instrument.instrument_id)
                            .or(recording_persons::instrument_id.eq(&instrument.instrument_id))
                            .or(ensemble_persons::instrument_id.eq(&instrument.instrument_id)),
                    );
                }

                if let Some(ensemble) = &query.ensemble {
                    statement = statement
                        .filter(recording_ensembles::ensemble_id.eq(&ensemble.ensemble_id));
                }

                if let Some(tag) = &query.tag {
                    // A tag matches if it is on the work itself or on any of its recordings.
                    // Written out because the subquery needs its own alias for `recordings`,
                    // which is already in the outer query. A valued tag matches only that
                    // exact value; a label tag binds NULL, which matches any assignment.
                    statement = statement.filter(
                        sql::<sql_types::Bool>(
                            "(EXISTS (SELECT 1 FROM work_tags \
                              WHERE work_tags.work_id = works.work_id AND work_tags.tag_id = ",
                        )
                        .bind::<sql_types::Text, _>(tag.tag.tag_id.clone())
                        .sql(" AND (")
                        .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
                        .sql(" IS NULL OR work_tags.value = ")
                        .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
                        .sql(
                            ")) OR EXISTS (SELECT 1 FROM recording_tags \
                             JOIN recordings AS tagged_recordings \
                             ON tagged_recordings.recording_id = recording_tags.recording_id \
                             WHERE tagged_recordings.work_id = works.work_id \
                             AND recording_tags.tag_id = ",
                        )
                        .bind::<sql_types::Text, _>(tag.tag.tag_id.clone())
                        .sql(" AND (")
                        .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
                        .sql(" IS NULL OR recording_tags.value = ")
                        .bind::<sql_types::Nullable<sql_types::Text>, _>(tag.value.clone())
                        .sql(")))"),
                    );
                }

                let albums = statement
                    .order_by(albums::last_played_at.desc())
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
                        statement = statement.filter(work_persons::person_id.eq(&person.person_id));
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
                            work_instruments::instrument_id
                                .eq(&instrument.instrument_id)
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
                        statement = statement.filter(work_persons::person_id.eq(&person.person_id));
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
                            work_instruments::instrument_id
                                .eq(&instrument.instrument_id)
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
                    .order_by(recordings::last_played_at.desc())
                    .load::<tables::Recording>(connection)?
                    .into_iter()
                    .map(|r| Recording::from_table(r, connection))
                    .collect::<Result<Vec<Recording>>>()?;

                LibraryResults {
                    recordings,
                    ..Default::default()
                }
            }
        })
    }

    pub fn generate_recording(&self, params: &GenerateRecordingParams) -> Result<Recording> {
        let connection = &mut *self.conn();

        let composer_id = params.composer_id.clone();
        let performer_id = params.performer_id.clone();
        let ensemble_id = params.ensemble_id.clone();
        let instrument_id = params.instrument_id.clone();
        let work_id = params.work_id.clone();
        let album_id = params.album_id.clone();
        let tag_id = params.tag_id.clone();
        let tag_value = params.tag_value.clone();

        let mut query = recordings::table
            .inner_join(
                works::table
                    .left_join(work_persons::table.inner_join(persons::table))
                    .left_join(work_instruments::table.inner_join(instruments::table)),
            )
            .left_join(recording_persons::table)
            .left_join(
                recording_ensembles::table
                    .left_join(ensembles::table.inner_join(ensemble_persons::table)),
            )
            .left_join(album_recordings::table)
            .into_boxed();

        if let Some(composer_id) = &composer_id {
            query = query.filter(work_persons::person_id.eq(composer_id));
        }

        if let Some(performer_id) = &performer_id {
            query = query.filter(
                recording_persons::person_id
                    .eq(performer_id)
                    .or(ensemble_persons::person_id.eq(performer_id)),
            );
        }

        if let Some(ensemble_id) = &ensemble_id {
            query = query.filter(recording_ensembles::ensemble_id.eq(ensemble_id));
        }

        if let Some(instrument_id) = &instrument_id {
            query = query.filter(
                work_instruments::instrument_id
                    .eq(instrument_id)
                    .or(recording_persons::instrument_id.eq(instrument_id))
                    .or(ensemble_persons::instrument_id.eq(instrument_id)),
            );
        }

        if let Some(work_id) = &work_id {
            query = query.filter(recordings::work_id.eq(work_id));
        }

        if let Some(album_id) = &album_id {
            query = query.filter(album_recordings::album_id.eq(album_id));
        }

        // As in `search`, a tag counts if it is on the recording or on its work,
        // and a valued tag only counts for the exact value.
        if let Some(tag_id) = &tag_id {
            query = query.filter(
                sql::<sql_types::Bool>(
                    "(EXISTS (SELECT 1 FROM recording_tags \
                      WHERE recording_tags.recording_id = recordings.recording_id \
                      AND recording_tags.tag_id = ",
                )
                .bind::<sql_types::Text, _>(tag_id.clone())
                .sql(" AND (")
                .bind::<sql_types::Nullable<sql_types::Text>, _>(tag_value.clone())
                .sql(" IS NULL OR recording_tags.value = ")
                .bind::<sql_types::Nullable<sql_types::Text>, _>(tag_value.clone())
                .sql(
                    ")) OR EXISTS (SELECT 1 FROM work_tags \
                     WHERE work_tags.work_id = recordings.work_id \
                     AND work_tags.tag_id = ",
                )
                .bind::<sql_types::Text, _>(tag_id.clone())
                .sql(" AND (")
                .bind::<sql_types::Nullable<sql_types::Text>, _>(tag_value.clone())
                .sql(" IS NULL OR work_tags.value = ")
                .bind::<sql_types::Nullable<sql_types::Text>, _>(tag_value.clone())
                .sql(")))"),
            );
        }

        // Orders recordings using a dynamically calculated priority score that includes:
        //  - a random base value between 0.0 and 1.0 giving equal probability to each recording
        //  - weighted by the average of two scores between 0.0 and 1.0 based on
        //    1. how long ago the last playback is
        //    2. how recently the recording was added to the library
        // Both scores are individually modified based on the following formula:
        //   e^(10 * a * (score - 1))
        // This assigns a new score between 0.0 and 1.0 that favors higher scores with "a" being
        // a user defined constant to determine the bias.
        query = query.order(
            diesel::dsl::sql::<sql_types::Untyped>("( \
                WITH global_bounds AS (
                    SELECT MIN(UNIXEPOCH(last_played_at)) AS min_last_played_at,
                        NULLIF(
                            MAX(UNIXEPOCH(last_played_at)) - MIN(UNIXEPOCH(last_played_at)),
                            0.0
                        ) AS last_played_at_range,
                        MIN(UNIXEPOCH(created_at)) AS min_created_at,
                        NULLIF(
                            MAX(UNIXEPOCH(created_at)) - MIN(UNIXEPOCH(created_at)),
                            0.0
                        ) AS created_at_range
                    FROM recordings
                ),
                normalized AS (
                    SELECT IFNULL(
                            1.0 - (
                                UNIXEPOCH(recordings.last_played_at) - min_last_played_at
                            ) * 1.0 / last_played_at_range,
                            1.0
                        ) AS least_recently_played,
                        IFNULL(
                            (
                                UNIXEPOCH(recordings.created_at) - min_created_at
                            ) * 1.0 / created_at_range,
                            1.0
                        ) AS recently_created
                    FROM global_bounds
                )
                SELECT (RANDOM() / 9223372036854775808.0 + 1.0) / 2.0 * MIN(
                        (
                            EXP(10.0 * ")
                                .bind::<sql_types::Double, _>(params.prefer_least_recently_played)
                                .sql(" * (least_recently_played - 1.0)) + EXP(10.0 * ")
                                .bind::<sql_types::Double, _>(params.prefer_recently_added)
                                .sql(" * (recently_created - 1.0))
                        ) / 2.0,
                        FIRST_VALUE(
                            MIN(
                                IFNULL(
                                    (
                                        UNIXEPOCH('now', 'localtime') - UNIXEPOCH(instruments.last_played_at)
                                    ) * 1.0 / ")
                                        .bind::<sql_types::Integer, _>(params.avoid_repeated_instruments)
                                        .sql(",
                                    1.0
                                ),
                                IFNULL(
                                    (
                                        UNIXEPOCH('now', 'localtime') - UNIXEPOCH(persons.last_played_at)
                                    ) * 1.0 / ").bind::<sql_types::Integer, _>(params.avoid_repeated_composers).sql(",
                                    1.0
                                ),
                                1.0
                            )
                        ) OVER (
                            PARTITION BY recordings.recording_id
                            ORDER BY MAX(
                                    IFNULL(instruments.last_played_at, 0),
                                    IFNULL(persons.last_played_at, 0)
                                )
                        )
                    )
                FROM normalized
            ) DESC")
        );

        let row = query
            .select(tables::Recording::as_select())
            .distinct()
            .first::<tables::Recording>(connection)?;

        Recording::from_table(row, connection)
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

    pub fn track_played(&self, track_id: &str) -> Result<()> {
        let connection = &mut *self.conn();

        let now = db::now();

        diesel::update(tracks::table)
            .filter(tracks::track_id.eq(track_id))
            .set(tracks::last_played_at.eq(now))
            .execute(connection)?;

        diesel::update(recordings::table)
            .filter(exists(
                tracks::table.filter(
                    tracks::track_id
                        .eq(track_id)
                        .and(tracks::recording_id.eq(recordings::recording_id)),
                ),
            ))
            .set(recordings::last_played_at.eq(now))
            .execute(connection)?;

        diesel::update(works::table)
            .filter(exists(
                recordings::table.inner_join(tracks::table).filter(
                    tracks::track_id
                        .eq(track_id)
                        .and(recordings::work_id.eq(works::work_id)),
                ),
            ))
            .set(works::last_played_at.eq(now))
            .execute(connection)?;

        diesel::update(instruments::table)
            .filter(exists(
                work_instruments::table
                    .inner_join(
                        works::table.inner_join(recordings::table.inner_join(tracks::table)),
                    )
                    .filter(
                        tracks::track_id
                            .eq(track_id)
                            .and(work_instruments::instrument_id.eq(instruments::instrument_id)),
                    ),
            ))
            .set(instruments::last_played_at.eq(now))
            .execute(connection)?;

        diesel::update(persons::table)
            .filter(
                exists(
                    work_persons::table
                        .inner_join(
                            works::table.inner_join(recordings::table.inner_join(tracks::table)),
                        )
                        .filter(
                            tracks::track_id
                                .eq(track_id)
                                .and(work_persons::person_id.eq(persons::person_id)),
                        ),
                )
                .or(exists(
                    recording_persons::table
                        .inner_join(recordings::table.inner_join(tracks::table))
                        .filter(
                            tracks::track_id
                                .eq(track_id)
                                .and(recording_persons::person_id.eq(persons::person_id)),
                        ),
                )),
            )
            .set(persons::last_played_at.eq(now))
            .execute(connection)?;

        diesel::update(ensembles::table)
            .filter(exists(
                recording_ensembles::table
                    .inner_join(recordings::table.inner_join(tracks::table))
                    .filter(
                        tracks::track_id
                            .eq(track_id)
                            .and(recording_ensembles::ensemble_id.eq(ensembles::ensemble_id)),
                    ),
            ))
            .set(ensembles::last_played_at.eq(now))
            .execute(connection)?;

        diesel::update(mediums::table)
            .filter(exists(
                tracks::table.filter(
                    tracks::track_id
                        .eq(track_id)
                        .and(tracks::medium_id.eq(mediums::medium_id.nullable())),
                ),
            ))
            .set(mediums::last_played_at.eq(now))
            .execute(connection)?;

        diesel::update(albums::table)
            .filter(
                exists(
                    album_recordings::table
                        .inner_join(recordings::table.inner_join(tracks::table))
                        .filter(
                            tracks::track_id
                                .eq(track_id)
                                .and(album_recordings::album_id.eq(albums::album_id)),
                        ),
                )
                .or(exists(
                    album_mediums::table
                        .inner_join(mediums::table.inner_join(tracks::table))
                        .filter(
                            tracks::track_id
                                .eq(track_id)
                                .and(album_mediums::album_id.eq(albums::album_id)),
                        ),
                )),
            )
            .set(albums::last_played_at.eq(now))
            .execute(connection)?;

        Ok(())
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
            .left_join(work_persons::table)
            .filter(
                works::name
                    .like(&search)
                    .and(work_persons::person_id.eq(&composer.person_id)),
            )
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
                .left_join(work_persons::table)
                .filter(
                    works::name
                        .like(&search)
                        .and(work_persons::person_id.eq(&composer.person_id)),
                )
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
        let year = library.create_tag(translated("Year"), true, true).unwrap();
        let baroque = library
            .create_tag(translated("Baroque"), false, true)
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

    /// Playing a tag-filtered search page must generate a recording.
    #[test]
    fn generating_a_recording_honours_a_tag() {
        let dir = TempDir::new().unwrap();
        let cache_dir = TempDir::new().unwrap();
        let library = library(&dir, &cache_dir);
        let (year, baroque, first, _) = tagged_library(&library);

        let params = GenerateRecordingParams {
            tag_id: Some(year.tag_id.clone()),
            tag_value: Some("1963".to_string()),
            ..Default::default()
        };
        let generated = library.generate_recording(&params).unwrap();
        assert_eq!(generated, first, "valued tag");

        // The weights a real program carries put binds in the ORDER BY as well
        // as the WHERE clause.
        let params = GenerateRecordingParams {
            tag_id: Some(year.tag_id.clone()),
            tag_value: Some("1963".to_string()),
            prefer_recently_added: 0.5,
            prefer_least_recently_played: 0.5,
            avoid_repeated_composers: 30,
            avoid_repeated_instruments: 30,
            ..Default::default()
        };
        let generated = library.generate_recording(&params).unwrap();
        assert_eq!(generated, first, "valued tag with program weights");

        let params = GenerateRecordingParams {
            tag_id: Some(baroque.tag_id.clone()),
            ..Default::default()
        };
        library
            .generate_recording(&params)
            .expect("label tag on the work must generate");
    }
}
