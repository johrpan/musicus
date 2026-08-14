use anyhow::Result;
use diesel::{
    dsl::{exists, sql},
    prelude::*,
    sql_types, QueryDsl,
};

use super::Library;
use crate::db::{self, models::*, schema::*, tables};

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

impl Library {
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
    }}
