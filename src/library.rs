use std::{
    cell::OnceCell,
    ffi::OsString,
    fs::{self, File},
    io::{BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
};

use adw::{
    glib::{self, subclass::Signal, Properties},
    prelude::*,
    subclass::prelude::*,
};
use anyhow::{anyhow, Result};
use chrono::prelude::*;
use diesel::{dsl::exists, prelude::*, sql_types, QueryDsl, SqliteConnection};
use once_cell::sync::Lazy;
use tempfile::NamedTempFile;
use zip::{write::SimpleFileOptions, ZipWriter};

use crate::{
    db::{self, models::*, schema::*, tables, TranslatedString},
    program::Program,
};

diesel::define_sql_function! {
    /// Represents the SQL RANDOM() function.
    fn random() -> Integer
}

mod imp {
    use super::*;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::Library)]
    pub struct Library {
        #[property(get, construct_only)]
        pub folder: OnceCell<String>,
        pub connection: OnceCell<Arc<Mutex<SqliteConnection>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Library {
        const NAME: &'static str = "MusicusLibrary";
        type Type = super::Library;
    }

    #[glib::derived_properties]
    impl ObjectImpl for Library {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> =
                Lazy::new(|| vec![Signal::builder("changed").build()]);

            SIGNALS.as_ref()
        }

        fn constructed(&self) {
            self.parent_constructed();

            let db_path = PathBuf::from(&self.folder.get().unwrap()).join("musicus.db");
            let connection = db::connect(db_path.to_str().unwrap()).unwrap();

            if self
                .connection
                .set(Arc::new(Mutex::new(connection)))
                .is_err()
            {
                panic!("connection should not be set");
            }
        }
    }
}

glib::wrapper! {
    pub struct Library(ObjectSubclass<imp::Library>);
}

impl Library {
    pub fn new(path: impl AsRef<Path>) -> Self {
        glib::Object::builder()
            .property("folder", path.as_ref().to_str().unwrap())
            .build()
    }

    /// Import from a library archive.
    pub fn import(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<async_channel::Receiver<LibraryProcessMsg>> {
        let path = path.as_ref().to_owned();
        let library_folder = PathBuf::from(&self.folder());
        let this_connection = self.imp().connection.get().unwrap().clone();

        let (sender, receiver) = async_channel::unbounded::<LibraryProcessMsg>();
        thread::spawn(move || {
            if let Err(err) = sender.send_blocking(LibraryProcessMsg::Result(import_from_zip(
                path,
                library_folder,
                this_connection,
                &sender,
            ))) {
                log::error!("Failed to send library action result: {err:?}");
            }
        });

        Ok(receiver)
    }

    /// Export the whole music library to an archive at `path`. If `path` already exists, it will
    /// be overwritten. The work will be done in a background thread.
    pub fn export(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<async_channel::Receiver<LibraryProcessMsg>> {
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        let path = path.as_ref().to_owned();
        let library_folder = PathBuf::from(&self.folder());
        let tracks = tracks::table.load::<tables::Track>(connection)?;

        let (sender, receiver) = async_channel::unbounded::<LibraryProcessMsg>();
        thread::spawn(move || {
            if let Err(err) = sender.send_blocking(LibraryProcessMsg::Result(write_zip(
                path,
                library_folder,
                tracks,
                &sender,
            ))) {
                log::error!("Failed to send library action result: {err:?}");
            }
        });

        Ok(receiver)
    }

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
                    ..Default::default()
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

    pub fn composer_default_role(&self) -> Result<Role> {
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        Ok(roles::table
            .filter(roles::role_id.eq("380d7e09eb2f49c1a90db2ba4acb6ffd"))
            .first::<Role>(connection)?)
    }

    pub fn performer_default_role(&self) -> Result<Role> {
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        Ok(roles::table
            .filter(roles::role_id.eq("28ff0aeb11c041a6916d93e9b4884eef"))
            .first::<Role>(connection)?)
    }

    pub fn create_person(&self, name: TranslatedString) -> Result<Person> {
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        let now = Local::now().naive_local();

        let person = Person {
            person_id: db::generate_id(),
            name,
            created_at: now,
            edited_at: now,
            last_used_at: now,
            last_played_at: None,
        };

        diesel::insert_into(persons::table)
            .values(&person)
            .execute(connection)?;

        self.changed();

        Ok(person)
    }

    pub fn update_person(&self, id: &str, name: TranslatedString) -> Result<()> {
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        let now = Local::now().naive_local();

        diesel::update(persons::table)
            .filter(persons::person_id.eq(id))
            .set((
                persons::name.eq(name),
                persons::edited_at.eq(now),
                persons::last_used_at.eq(now),
            ))
            .execute(connection)?;

        self.changed();

        Ok(())
    }

    pub fn delete_person(&self, person_id: &str) -> Result<()> {
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        diesel::delete(persons::table)
            .filter(persons::person_id.eq(person_id))
            .execute(connection)?;

        self.changed();

        Ok(())
    }

    pub fn create_instrument(&self, name: TranslatedString) -> Result<Instrument> {
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        let now = Local::now().naive_local();

        let instrument = Instrument {
            instrument_id: db::generate_id(),
            name,
            created_at: now,
            edited_at: now,
            last_used_at: now,
            last_played_at: None,
        };

        diesel::insert_into(instruments::table)
            .values(&instrument)
            .execute(connection)?;

        self.changed();

        Ok(instrument)
    }

    pub fn update_instrument(&self, id: &str, name: TranslatedString) -> Result<()> {
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        let now = Local::now().naive_local();

        diesel::update(instruments::table)
            .filter(instruments::instrument_id.eq(id))
            .set((
                instruments::name.eq(name),
                instruments::edited_at.eq(now),
                instruments::last_used_at.eq(now),
            ))
            .execute(connection)?;

        self.changed();

        Ok(())
    }

    pub fn delete_instrument(&self, instrument_id: &str) -> Result<()> {
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        diesel::delete(instruments::table)
            .filter(instruments::instrument_id.eq(instrument_id))
            .execute(connection)?;

        self.changed();

        Ok(())
    }

    pub fn create_role(&self, name: TranslatedString) -> Result<Role> {
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        let now = Local::now().naive_local();

        let role = Role {
            role_id: db::generate_id(),
            name,
            created_at: now,
            edited_at: now,
            last_used_at: now,
        };

        diesel::insert_into(roles::table)
            .values(&role)
            .execute(connection)?;

        self.changed();

        Ok(role)
    }

    pub fn update_role(&self, id: &str, name: TranslatedString) -> Result<()> {
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        let now = Local::now().naive_local();

        diesel::update(roles::table)
            .filter(roles::role_id.eq(id))
            .set((
                roles::name.eq(name),
                roles::edited_at.eq(now),
                roles::last_used_at.eq(now),
            ))
            .execute(connection)?;

        self.changed();

        Ok(())
    }

    pub fn delete_role(&self, role_id: &str) -> Result<()> {
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        diesel::delete(roles::table)
            .filter(roles::role_id.eq(role_id))
            .execute(connection)?;

        self.changed();

        Ok(())
    }

    pub fn create_work(
        &self,
        name: TranslatedString,
        parts: Vec<Work>,
        persons: Vec<Composer>,
        instruments: Vec<Instrument>,
    ) -> Result<Work> {
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        let work =
            self.create_work_priv(connection, name, parts, persons, instruments, None, None)?;

        self.changed();

        Ok(work)
    }

    fn create_work_priv(
        &self,
        connection: &mut SqliteConnection,
        name: TranslatedString,
        parts: Vec<Work>,
        persons: Vec<Composer>,
        instruments: Vec<Instrument>,
        parent_work_id: Option<&str>,
        sequence_number: Option<i32>,
    ) -> Result<Work> {
        let work_id = db::generate_id();
        let now = Local::now().naive_local();

        let work_data = tables::Work {
            work_id: work_id.clone(),
            parent_work_id: parent_work_id.map(|w| w.to_string()),
            sequence_number: sequence_number,
            name,
            created_at: now,
            edited_at: now,
            last_used_at: now,
            last_played_at: None,
        };

        diesel::insert_into(works::table)
            .values(&work_data)
            .execute(connection)?;

        for (index, part) in parts.into_iter().enumerate() {
            self.create_work_priv(
                connection,
                part.name,
                part.parts,
                part.persons,
                part.instruments,
                Some(&work_id),
                Some(index as i32),
            )?;
        }

        for (index, composer) in persons.into_iter().enumerate() {
            let composer_data = tables::WorkPerson {
                work_id: work_id.clone(),
                person_id: composer.person.person_id,
                role_id: composer.role.role_id,
                sequence_number: index as i32,
            };

            diesel::insert_into(work_persons::table)
                .values(composer_data)
                .execute(connection)?;
        }

        for (index, instrument) in instruments.into_iter().enumerate() {
            let instrument_data = tables::WorkInstrument {
                work_id: work_id.clone(),
                instrument_id: instrument.instrument_id,
                sequence_number: index as i32,
            };

            diesel::insert_into(work_instruments::table)
                .values(instrument_data)
                .execute(connection)?;
        }

        let work = Work::from_table(work_data, connection)?;

        Ok(work)
    }

    pub fn update_work(
        &self,
        work_id: &str,
        name: TranslatedString,
        parts: Vec<Work>,
        persons: Vec<Composer>,
        instruments: Vec<Instrument>,
    ) -> Result<()> {
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        self.update_work_priv(
            connection,
            work_id,
            name,
            parts,
            persons,
            instruments,
            None,
            None,
        )?;

        self.changed();

        Ok(())
    }

    fn update_work_priv(
        &self,
        connection: &mut SqliteConnection,
        work_id: &str,
        name: TranslatedString,
        parts: Vec<Work>,
        persons: Vec<Composer>,
        instruments: Vec<Instrument>,
        parent_work_id: Option<&str>,
        sequence_number: Option<i32>,
    ) -> Result<()> {
        let now = Local::now().naive_local();

        diesel::update(works::table)
            .filter(works::work_id.eq(work_id))
            .set((
                works::parent_work_id.eq(parent_work_id),
                works::sequence_number.eq(sequence_number),
                works::name.eq(name),
                works::edited_at.eq(now),
                works::last_used_at.eq(now),
            ))
            .execute(connection)?;

        diesel::delete(works::table)
            .filter(
                works::parent_work_id
                    .eq(work_id)
                    .and(works::work_id.ne_all(parts.iter().map(|p| p.work_id.clone()))),
            )
            .execute(connection)?;

        for (index, part) in parts.into_iter().enumerate() {
            if works::table
                .filter(works::work_id.eq(&part.work_id))
                .first::<tables::Work>(connection)
                .optional()?
                .is_some()
            {
                self.update_work_priv(
                    connection,
                    &part.work_id,
                    part.name,
                    part.parts,
                    part.persons,
                    part.instruments,
                    Some(work_id),
                    Some(index as i32),
                )?;
            } else {
                // Note: The previously used ID is discarded. This should be OK, because
                // at this point, the part ID should not have been used anywhere.
                self.create_work_priv(
                    connection,
                    part.name,
                    part.parts,
                    part.persons,
                    part.instruments,
                    Some(work_id),
                    Some(index as i32),
                )?;
            }
        }

        diesel::delete(work_persons::table)
            .filter(work_persons::work_id.eq(work_id))
            .execute(connection)?;

        for (index, composer) in persons.into_iter().enumerate() {
            let composer_data = tables::WorkPerson {
                work_id: work_id.to_string(),
                person_id: composer.person.person_id,
                role_id: composer.role.role_id,
                sequence_number: index as i32,
            };

            diesel::insert_into(work_persons::table)
                .values(composer_data)
                .execute(connection)?;
        }

        diesel::delete(work_instruments::table)
            .filter(work_instruments::work_id.eq(work_id))
            .execute(connection)?;

        for (index, instrument) in instruments.into_iter().enumerate() {
            let instrument_data = tables::WorkInstrument {
                work_id: work_id.to_string(),
                instrument_id: instrument.instrument_id,
                sequence_number: index as i32,
            };

            diesel::insert_into(work_instruments::table)
                .values(instrument_data)
                .execute(connection)?;
        }

        Ok(())
    }

    pub fn delete_work(&self, work_id: &str) -> Result<()> {
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        diesel::delete(works::table)
            .filter(works::work_id.eq(work_id))
            .execute(connection)?;

        self.changed();

        Ok(())
    }

    pub fn create_ensemble(&self, name: TranslatedString) -> Result<Ensemble> {
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        let now = Local::now().naive_local();

        let ensemble_data = tables::Ensemble {
            ensemble_id: db::generate_id(),
            name,
            created_at: now,
            edited_at: now,
            last_used_at: now,
            last_played_at: None,
        };

        // TODO: Add persons.

        diesel::insert_into(ensembles::table)
            .values(&ensemble_data)
            .execute(connection)?;

        let ensemble = Ensemble::from_table(ensemble_data, connection)?;

        self.changed();

        Ok(ensemble)
    }

    pub fn update_ensemble(&self, id: &str, name: TranslatedString) -> Result<()> {
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        let now = Local::now().naive_local();

        diesel::update(ensembles::table)
            .filter(ensembles::ensemble_id.eq(id))
            .set((
                ensembles::name.eq(name),
                ensembles::edited_at.eq(now),
                ensembles::last_used_at.eq(now),
            ))
            .execute(connection)?;

        // TODO: Support updating persons.

        self.changed();

        Ok(())
    }

    pub fn delete_ensemble(&self, ensemble_id: &str) -> Result<()> {
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        diesel::delete(ensembles::table)
            .filter(ensembles::ensemble_id.eq(ensemble_id))
            .execute(connection)?;

        self.changed();

        Ok(())
    }

    pub fn create_recording(
        &self,
        work: Work,
        year: Option<i32>,
        performers: Vec<Performer>,
        ensembles: Vec<EnsemblePerformer>,
    ) -> Result<Recording> {
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        let recording_id = db::generate_id();
        let now = Local::now().naive_local();

        let recording_data = tables::Recording {
            recording_id: recording_id.clone(),
            work_id: work.work_id.clone(),
            year,
            created_at: now,
            edited_at: now,
            last_used_at: now,
            last_played_at: None,
        };

        diesel::insert_into(recordings::table)
            .values(&recording_data)
            .execute(connection)?;

        for (index, performer) in performers.into_iter().enumerate() {
            let recording_person_data = tables::RecordingPerson {
                recording_id: recording_id.clone(),
                person_id: performer.person.person_id,
                role_id: performer.role.role_id,
                instrument_id: performer.instrument.map(|i| i.instrument_id),
                sequence_number: index as i32,
            };

            diesel::insert_into(recording_persons::table)
                .values(&recording_person_data)
                .execute(connection)?;
        }

        for (index, ensemble) in ensembles.into_iter().enumerate() {
            let recording_ensemble_data = tables::RecordingEnsemble {
                recording_id: recording_id.clone(),
                ensemble_id: ensemble.ensemble.ensemble_id,
                role_id: ensemble.role.role_id,
                sequence_number: index as i32,
            };

            diesel::insert_into(recording_ensembles::table)
                .values(&recording_ensemble_data)
                .execute(connection)?;
        }

        let recording = Recording::from_table(recording_data, connection)?;

        self.changed();

        Ok(recording)
    }

    pub fn update_recording(
        &self,
        recording_id: &str,
        work: Work,
        year: Option<i32>,
        performers: Vec<Performer>,
        ensembles: Vec<EnsemblePerformer>,
    ) -> Result<()> {
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        let now = Local::now().naive_local();

        diesel::update(recordings::table)
            .filter(recordings::recording_id.eq(recording_id))
            .set((
                recordings::work_id.eq(work.work_id),
                recordings::year.eq(year),
                recordings::edited_at.eq(now),
                recordings::last_used_at.eq(now),
            ))
            .execute(connection)?;

        diesel::delete(recording_persons::table)
            .filter(recording_persons::recording_id.eq(recording_id))
            .execute(connection)?;

        for (index, performer) in performers.into_iter().enumerate() {
            let recording_person_data = tables::RecordingPerson {
                recording_id: recording_id.to_string(),
                person_id: performer.person.person_id,
                role_id: performer.role.role_id,
                instrument_id: performer.instrument.map(|i| i.instrument_id),
                sequence_number: index as i32,
            };

            diesel::insert_into(recording_persons::table)
                .values(&recording_person_data)
                .execute(connection)?;
        }

        diesel::delete(recording_ensembles::table)
            .filter(recording_ensembles::recording_id.eq(recording_id))
            .execute(connection)?;

        for (index, ensemble) in ensembles.into_iter().enumerate() {
            let recording_ensemble_data = tables::RecordingEnsemble {
                recording_id: recording_id.to_string(),
                ensemble_id: ensemble.ensemble.ensemble_id,
                role_id: ensemble.role.role_id,
                sequence_number: index as i32,
            };

            diesel::insert_into(recording_ensembles::table)
                .values(&recording_ensemble_data)
                .execute(connection)?;
        }

        self.changed();

        Ok(())
    }

    pub fn delete_recording(&self, recording_id: &str) -> Result<()> {
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        diesel::delete(recordings::table)
            .filter(recordings::recording_id.eq(recording_id))
            .execute(connection)?;

        self.changed();

        Ok(())
    }

    pub fn create_album(
        &self,
        name: TranslatedString,
        recordings: Vec<Recording>,
    ) -> Result<Album> {
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        let album_id = db::generate_id();
        let now = Local::now().naive_local();

        let album_data = tables::Album {
            album_id: album_id.clone(),
            name,
            created_at: now,
            edited_at: now,
            last_used_at: now,
            last_played_at: None,
        };

        diesel::insert_into(albums::table)
            .values(&album_data)
            .execute(connection)?;

        for (index, recording) in recordings.into_iter().enumerate() {
            let album_recording_data = tables::AlbumRecording {
                album_id: album_id.clone(),
                recording_id: recording.recording_id,
                sequence_number: index as i32,
            };

            diesel::insert_into(album_recordings::table)
                .values(&album_recording_data)
                .execute(connection)?;
        }

        let album = Album::from_table(album_data, connection)?;

        self.changed();

        Ok(album)
    }

    pub fn update_album(
        &self,
        album_id: &str,
        name: TranslatedString,
        recordings: Vec<Recording>,
    ) -> Result<()> {
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        let now = Local::now().naive_local();

        diesel::update(albums::table)
            .filter(albums::album_id.eq(album_id))
            .set((
                albums::name.eq(name),
                albums::edited_at.eq(now),
                albums::last_used_at.eq(now),
            ))
            .execute(connection)?;

        diesel::delete(album_recordings::table)
            .filter(album_recordings::album_id.eq(album_id))
            .execute(connection)?;

        for (index, recording) in recordings.into_iter().enumerate() {
            let album_recording_data = tables::AlbumRecording {
                album_id: album_id.to_owned(),
                recording_id: recording.recording_id,
                sequence_number: index as i32,
            };

            diesel::insert_into(album_recordings::table)
                .values(&album_recording_data)
                .execute(connection)?;
        }

        self.changed();

        Ok(())
    }

    pub fn delete_album(&self, album_id: &str) -> Result<()> {
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        diesel::delete(albums::table)
            .filter(albums::album_id.eq(album_id))
            .execute(connection)?;

        self.changed();

        Ok(())
    }

    /// Import a track into the music library.
    // TODO: Support mediums.
    pub fn import_track(
        &self,
        path: impl AsRef<Path>,
        recording_id: &str,
        recording_index: i32,
        works: Vec<Work>,
    ) -> Result<()> {
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        let track_id = db::generate_id();
        let now = Local::now().naive_local();

        // TODO: Human interpretable filenames?
        let mut filename = OsString::from(recording_id);
        filename.push("_");
        filename.push(OsString::from(format!("{recording_index:02}")));
        if let Some(extension) = path.as_ref().extension() {
            filename.push(".");
            filename.push(extension);
        };

        let mut to_path = PathBuf::from(self.folder());
        to_path.push(&filename);
        let library_path = filename
            .into_string()
            .or(Err(anyhow!("Filename contains invalid Unicode.")))?;

        fs::copy(path, to_path)?;

        let track_data = tables::Track {
            track_id: track_id.clone(),
            recording_id: recording_id.to_owned(),
            recording_index,
            medium_id: None,
            medium_index: None,
            path: library_path,
            created_at: now,
            edited_at: now,
            last_used_at: now,
            last_played_at: None,
        };

        diesel::insert_into(tracks::table)
            .values(&track_data)
            .execute(connection)?;

        for (index, work) in works.into_iter().enumerate() {
            let track_work_data = tables::TrackWork {
                track_id: track_id.clone(),
                work_id: work.work_id,
                sequence_number: index as i32,
            };

            diesel::insert_into(track_works::table)
                .values(&track_work_data)
                .execute(connection)?;
        }

        Ok(())
    }

    // TODO: Support mediums, think about albums.
    pub fn delete_track(&self, track: &Track) -> Result<()> {
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        diesel::delete(track_works::table)
            .filter(track_works::track_id.eq(&track.track_id))
            .execute(connection)?;

        diesel::delete(tracks::table)
            .filter(tracks::track_id.eq(&track.track_id))
            .execute(connection)?;

        let mut path = PathBuf::from(self.folder());
        path.push(&track.path);
        fs::remove_file(path)?;

        Ok(())
    }

    // TODO: Support mediums, think about albums.
    pub fn update_track(
        &self,
        track_id: &str,
        recording_index: i32,
        works: Vec<Work>,
    ) -> Result<()> {
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        let now = Local::now().naive_local();

        diesel::update(tracks::table)
            .filter(tracks::track_id.eq(track_id.to_owned()))
            .set((
                tracks::recording_index.eq(recording_index),
                tracks::edited_at.eq(now),
                tracks::last_used_at.eq(now),
            ))
            .execute(connection)?;

        diesel::delete(track_works::table)
            .filter(track_works::track_id.eq(track_id))
            .execute(connection)?;

        for (index, work) in works.into_iter().enumerate() {
            let track_work_data = tables::TrackWork {
                track_id: track_id.to_owned(),
                work_id: work.work_id,
                sequence_number: index as i32,
            };

            diesel::insert_into(track_works::table)
                .values(&track_work_data)
                .execute(connection)?;
        }

        Ok(())
    }

    pub fn connect_changed<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("changed", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            f(&obj);
            None
        })
    }

    pub fn changed(&self) {
        let obj = self.clone();
        // Note: This is a dirty hack to let the calling function return before
        // signal handlers are called. This is neccessary because RefCells
        // may still be borrowed otherwise.
        glib::spawn_future_local(async move {
            obj.emit_by_name::<()>("changed", &[]);
        });
    }
}

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

fn write_zip(
    zip_path: impl AsRef<Path>,
    library_folder: impl AsRef<Path>,
    tracks: Vec<tables::Track>,
    sender: &async_channel::Sender<LibraryProcessMsg>,
) -> Result<()> {
    let mut zip = zip::ZipWriter::new(BufWriter::new(fs::File::create(zip_path)?));

    // Start with the database:
    add_file_to_zip(&mut zip, &library_folder, "musicus.db")?;

    let n_tracks = tracks.len();

    // Include all tracks that are part of the library.
    for (index, track) in tracks.into_iter().enumerate() {
        add_file_to_zip(&mut zip, &library_folder, &track.path)?;

        // Ignore if the reveiver has been dropped.
        let _ = sender.send_blocking(LibraryProcessMsg::Progress(
            (index + 1) as f64 / n_tracks as f64,
        ));
    }

    zip.finish()?;

    Ok(())
}

// TODO: Cross-platform paths?
fn add_file_to_zip(
    zip: &mut ZipWriter<BufWriter<File>>,
    library_folder: impl AsRef<Path>,
    library_path: &str,
) -> Result<()> {
    let file_path = library_folder.as_ref().join(PathBuf::from(library_path));

    let mut file = File::open(file_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    zip.start_file(library_path, SimpleFileOptions::default())?;
    zip.write_all(&buffer)?;

    Ok(())
}

// TODO: Add options whether to keep stats.
fn import_from_zip(
    zip_path: impl AsRef<Path>,
    library_folder: impl AsRef<Path>,
    this_connection: Arc<Mutex<SqliteConnection>>,
    sender: &async_channel::Sender<LibraryProcessMsg>,
) -> Result<()> {
    let now = Local::now().naive_local();

    let mut archive = zip::ZipArchive::new(BufReader::new(fs::File::open(zip_path)?))?;

    let archive_db_file = archive.by_name("musicus.db")?;
    let tmp_db_file = NamedTempFile::new()?;
    std::io::copy(
        &mut BufReader::new(archive_db_file),
        &mut BufWriter::new(tmp_db_file.as_file()),
    )?;

    let mut other_connection = db::connect(tmp_db_file.path().to_str().unwrap())?;

    // Load all metadata from the archive.
    let persons = persons::table.load::<tables::Person>(&mut other_connection)?;
    let roles = roles::table.load::<tables::Role>(&mut other_connection)?;
    let instruments = instruments::table.load::<tables::Instrument>(&mut other_connection)?;
    let works = works::table.load::<tables::Work>(&mut other_connection)?;
    let work_persons = work_persons::table.load::<tables::WorkPerson>(&mut other_connection)?;
    let work_instruments =
        work_instruments::table.load::<tables::WorkInstrument>(&mut other_connection)?;
    let ensembles = ensembles::table.load::<tables::Ensemble>(&mut other_connection)?;
    let ensemble_persons =
        ensemble_persons::table.load::<tables::EnsemblePerson>(&mut other_connection)?;
    let recordings = recordings::table.load::<tables::Recording>(&mut other_connection)?;
    let recording_persons =
        recording_persons::table.load::<tables::RecordingPerson>(&mut other_connection)?;
    let recording_ensembles =
        recording_ensembles::table.load::<tables::RecordingEnsemble>(&mut other_connection)?;
    let tracks = tracks::table.load::<tables::Track>(&mut other_connection)?;
    let track_works = track_works::table.load::<tables::TrackWork>(&mut other_connection)?;
    let mediums = mediums::table.load::<tables::Medium>(&mut other_connection)?;
    let albums = albums::table.load::<tables::Album>(&mut other_connection)?;
    let album_recordings =
        album_recordings::table.load::<tables::AlbumRecording>(&mut other_connection)?;
    let album_mediums = album_mediums::table.load::<tables::AlbumMedium>(&mut other_connection)?;

    // Import metadata that is not already present.

    for mut person in persons {
        person.created_at = now;
        person.edited_at = now;
        person.last_used_at = now;
        person.last_played_at = None;

        diesel::insert_into(persons::table)
            .values(person)
            .on_conflict_do_nothing()
            .execute(&mut *this_connection.lock().unwrap())?;
    }

    for mut role in roles {
        role.created_at = now;
        role.edited_at = now;
        role.last_used_at = now;

        diesel::insert_into(roles::table)
            .values(role)
            .on_conflict_do_nothing()
            .execute(&mut *this_connection.lock().unwrap())?;
    }

    for mut instrument in instruments {
        instrument.created_at = now;
        instrument.edited_at = now;
        instrument.last_used_at = now;
        instrument.last_played_at = None;

        diesel::insert_into(instruments::table)
            .values(instrument)
            .on_conflict_do_nothing()
            .execute(&mut *this_connection.lock().unwrap())?;
    }

    for mut work in works {
        work.created_at = now;
        work.edited_at = now;
        work.last_used_at = now;
        work.last_played_at = None;

        diesel::insert_into(works::table)
            .values(work)
            .on_conflict_do_nothing()
            .execute(&mut *this_connection.lock().unwrap())?;
    }

    for work_person in work_persons {
        diesel::insert_into(work_persons::table)
            .values(work_person)
            .on_conflict_do_nothing()
            .execute(&mut *this_connection.lock().unwrap())?;
    }

    for work_instrument in work_instruments {
        diesel::insert_into(work_instruments::table)
            .values(work_instrument)
            .on_conflict_do_nothing()
            .execute(&mut *this_connection.lock().unwrap())?;
    }

    for mut ensemble in ensembles {
        ensemble.created_at = now;
        ensemble.edited_at = now;
        ensemble.last_used_at = now;
        ensemble.last_played_at = None;

        diesel::insert_into(ensembles::table)
            .values(ensemble)
            .on_conflict_do_nothing()
            .execute(&mut *this_connection.lock().unwrap())?;
    }

    for ensemble_person in ensemble_persons {
        diesel::insert_into(ensemble_persons::table)
            .values(ensemble_person)
            .on_conflict_do_nothing()
            .execute(&mut *this_connection.lock().unwrap())?;
    }

    for mut recording in recordings {
        recording.created_at = now;
        recording.edited_at = now;
        recording.last_used_at = now;
        recording.last_played_at = None;

        diesel::insert_into(recordings::table)
            .values(recording)
            .on_conflict_do_nothing()
            .execute(&mut *this_connection.lock().unwrap())?;
    }

    for recording_person in recording_persons {
        diesel::insert_into(recording_persons::table)
            .values(recording_person)
            .on_conflict_do_nothing()
            .execute(&mut *this_connection.lock().unwrap())?;
    }

    for recording_ensemble in recording_ensembles {
        diesel::insert_into(recording_ensembles::table)
            .values(recording_ensemble)
            .on_conflict_do_nothing()
            .execute(&mut *this_connection.lock().unwrap())?;
    }

    for mut track in tracks.clone() {
        track.created_at = now;
        track.edited_at = now;
        track.last_used_at = now;
        track.last_played_at = None;

        diesel::insert_into(tracks::table)
            .values(track)
            .on_conflict_do_nothing()
            .execute(&mut *this_connection.lock().unwrap())?;
    }

    for track_work in track_works {
        diesel::insert_into(track_works::table)
            .values(track_work)
            .on_conflict_do_nothing()
            .execute(&mut *this_connection.lock().unwrap())?;
    }

    for mut medium in mediums {
        medium.created_at = now;
        medium.edited_at = now;
        medium.last_used_at = now;
        medium.last_played_at = None;

        diesel::insert_into(mediums::table)
            .values(medium)
            .on_conflict_do_nothing()
            .execute(&mut *this_connection.lock().unwrap())?;
    }

    for mut album in albums {
        album.created_at = now;
        album.edited_at = now;
        album.last_used_at = now;
        album.last_played_at = None;

        diesel::insert_into(albums::table)
            .values(album)
            .on_conflict_do_nothing()
            .execute(&mut *this_connection.lock().unwrap())?;
    }

    for album_recording in album_recordings {
        diesel::insert_into(album_recordings::table)
            .values(album_recording)
            .on_conflict_do_nothing()
            .execute(&mut *this_connection.lock().unwrap())?;
    }

    for album_medium in album_mediums {
        diesel::insert_into(album_mediums::table)
            .values(album_medium)
            .on_conflict_do_nothing()
            .execute(&mut *this_connection.lock().unwrap())?;
    }

    // Import audio files.

    let n_tracks = tracks.len();

    // TODO: Cross-platform paths?
    for (index, track) in tracks.into_iter().enumerate() {
        let library_track_file_path = library_folder.as_ref().join(Path::new(&track.path));

        // Skip tracks that are already present.
        if !fs::exists(&library_track_file_path)? {
            if let Some(parent) = library_track_file_path.parent() {
                fs::create_dir_all(parent)?;
            }

            let archive_track_file = archive.by_name(&track.path)?;
            let library_track_file = File::create(library_track_file_path)?;

            std::io::copy(
                &mut BufReader::new(archive_track_file),
                &mut BufWriter::new(library_track_file),
            )?;
        }

        // Ignore if the reveiver has been dropped.
        let _ = sender.send_blocking(LibraryProcessMsg::Progress(
            (index + 1) as f64 / n_tracks as f64,
        ));
    }

    Ok(())
}

#[derive(Debug)]
pub enum LibraryProcessMsg {
    Progress(f64),
    Result(Result<()>),
}
