use adw::subclass::prelude::*;
use anyhow::Result;
use chrono::prelude::*;
use diesel::{dsl::exists, prelude::*, sql_types, QueryDsl};

use super::Library;
use crate::{
    db::{models::*, schema::*, tables},
    program::Program,
};

#[derive(Clone, Default, Debug)]
pub struct LibraryQuery {
    pub composer: Option<Person>,
    pub performer: Option<Person>,
    pub ensemble: Option<Ensemble>,
    pub instrument: Option<Instrument>,
    pub work: Option<Work>,
}

impl LibraryQuery {
    pub fn is_empty(&self) -> bool {
        self.composer.is_none()
            && self.performer.is_none()
            && self.ensemble.is_none()
            && self.instrument.is_none()
            && self.work.is_none()
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
    }
}

impl Library {
    pub fn search(&self, query: &LibraryQuery, search: &str) -> Result<LibraryResults> {
        let search = format!("%{}%", search);
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

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

                let albums = statement
                    .order_by(albums::last_played_at.desc())
                    .limit(9)
                    .select(albums::all_columns)
                    .distinct()
                    .load::<tables::Album>(connection)?
                    .into_iter()
                    .map(|r| Album::from_table(r, connection))
                    .collect::<Result<Vec<Album>>>()?;

                LibraryResults {
                    composers,
                    performers,
                    ensembles,
                    instruments,
                    works,
                    recordings,
                    albums,
                }
            }
            LibraryQuery {
                work: Some(work), ..
            } => {
                let recordings = recordings::table
                    .filter(recordings::work_id.eq(&work.work_id))
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

    pub fn generate_recording(&self, program: &Program) -> Result<Recording> {
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        let composer_id = program.composer_id();
        let performer_id = program.performer_id();
        let ensemble_id = program.ensemble_id();
        let instrument_id = program.instrument_id();
        let work_id = program.work_id();
        let album_id = program.album_id();

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
                                .bind::<sql_types::Double, _>(program.prefer_least_recently_played())
                                .sql(" * (least_recently_played - 1.0)) + EXP(10.0 * ")
                                .bind::<sql_types::Double, _>(program.prefer_recently_added())
                                .sql(" * (recently_created - 1.0))
                        ) / 2.0,
                        FIRST_VALUE(
                            MIN(
                                IFNULL(
                                    (
                                        UNIXEPOCH('now', 'localtime') - UNIXEPOCH(instruments.last_played_at)
                                    ) * 1.0 / ")
                                        .bind::<sql_types::Integer, _>(program.avoid_repeated_instruments())
                                        .sql(",
                                    1.0
                                ),
                                IFNULL(
                                    (
                                        UNIXEPOCH('now', 'localtime') - UNIXEPOCH(persons.last_played_at)
                                    ) * 1.0 / ").bind::<sql_types::Integer, _>(program.avoid_repeated_composers()).sql(",
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
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

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
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        let now = Local::now().naive_local();

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

    pub fn search_persons(&self, search: &str) -> Result<Vec<Person>> {
        let search = format!("%{}%", search);
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        let persons = persons::table
            .order(persons::last_used_at.desc())
            .filter(persons::name.like(&search))
            .limit(20)
            .load(connection)?;

        Ok(persons)
    }

    pub fn search_roles(&self, search: &str) -> Result<Vec<Role>> {
        let search = format!("%{}%", search);
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        let roles = roles::table
            .order(roles::last_used_at.desc())
            .filter(roles::name.like(&search))
            .limit(20)
            .load(connection)?;

        Ok(roles)
    }

    pub fn search_instruments(&self, search: &str) -> Result<Vec<Instrument>> {
        let search = format!("%{}%", search);
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        let instruments = instruments::table
            .order(instruments::last_used_at.desc())
            .filter(instruments::name.like(&search))
            .limit(20)
            .load(connection)?;

        Ok(instruments)
    }

    pub fn search_works(&self, composer: &Person, search: &str) -> Result<Vec<Work>> {
        let search = format!("%{}%", search);
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        let works: Vec<Work> = works::table
            .left_join(work_persons::table)
            .filter(
                works::name
                    .like(&search)
                    .and(work_persons::person_id.eq(&composer.person_id)),
            )
            .limit(9)
            .select(works::all_columns)
            .distinct()
            .load::<tables::Work>(connection)?
            .into_iter()
            .map(|w| Work::from_table(w, connection))
            .collect::<Result<Vec<Work>>>()?;

        Ok(works)
    }

    pub fn search_recordings(&self, work: &Work, search: &str) -> Result<Vec<Recording>> {
        let search = format!("%{}%", search);
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        let recordings = recordings::table
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
            .load::<tables::Recording>(connection)?
            .into_iter()
            .map(|r| Recording::from_table(r, connection))
            .collect::<Result<Vec<Recording>>>()?;

        Ok(recordings)
    }

    pub fn search_ensembles(&self, search: &str) -> Result<Vec<Ensemble>> {
        let search = format!("%{}%", search);
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        let ensembles = ensembles::table
            .order(ensembles::last_used_at.desc())
            .left_join(ensemble_persons::table.inner_join(persons::table))
            .filter(
                ensembles::name
                    .like(&search)
                    .or(persons::name.like(&search)),
            )
            .limit(20)
            .select(ensembles::all_columns)
            .load::<tables::Ensemble>(connection)?
            .into_iter()
            .map(|e| Ensemble::from_table(e, connection))
            .collect::<Result<Vec<Ensemble>>>()?;

        Ok(ensembles)
    }
}
