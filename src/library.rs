use std::{
    cell::{OnceCell, RefCell},
    path::{Path, PathBuf},
};

use anyhow::Result;
use chrono::prelude::*;
use diesel::{
    dsl::{exists, sql},
    prelude::*,
    sql_query,
    sql_types::BigInt,
    QueryDsl, SqliteConnection,
};
use gtk::{glib, glib::Properties, prelude::*, subclass::prelude::*};

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
    #[properties(wrapper_type = super::MusicusLibrary)]
    pub struct MusicusLibrary {
        #[property(get, construct_only)]
        pub folder: OnceCell<String>,
        pub connection: RefCell<Option<SqliteConnection>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MusicusLibrary {
        const NAME: &'static str = "MusicusLibrary";
        type Type = super::MusicusLibrary;
    }

    #[glib::derived_properties]
    impl ObjectImpl for MusicusLibrary {
        fn constructed(&self) {
            self.parent_constructed();

            let db_path = PathBuf::from(&self.folder.get().unwrap()).join("musicus.db");
            let connection = db::connect(db_path.to_str().unwrap()).unwrap();
            self.connection.replace(Some(connection));
        }
    }
}

glib::wrapper! {
    pub struct MusicusLibrary(ObjectSubclass<imp::MusicusLibrary>);
}

impl MusicusLibrary {
    pub fn new(path: impl AsRef<Path>) -> Self {
        glib::Object::builder()
            .property("folder", path.as_ref().to_str().unwrap())
            .build()
    }

    pub fn query(&self, query: &LibraryQuery) -> Result<LibraryResults> {
        let search = format!("%{}%", query.search);
        let mut binding = self.imp().connection.borrow_mut();
        let connection = &mut *binding.as_mut().unwrap();

        Ok(match query {
            LibraryQuery {
                composer: None,
                performer: None,
                ensemble: None,
                work: None,
                ..
            } => {
                let composers: Vec<Person> = persons::table
                    .filter(
                        exists(
                            work_persons::table
                                .filter(work_persons::person_id.eq(persons::person_id)),
                        )
                        .and(persons::name.like(&search)),
                    )
                    .limit(9)
                    .load(connection)?;

                let performers: Vec<Person> = persons::table
                    .filter(
                        exists(
                            recording_persons::table
                                .filter(recording_persons::person_id.eq(persons::person_id)),
                        )
                        .and(persons::name.like(&search)),
                    )
                    .limit(9)
                    .load(connection)?;

                // TODO: Search ensemble persons as well.
                let ensembles: Vec<Ensemble> = ensembles::table
                    .filter(ensembles::name.like(&search))
                    .limit(9)
                    .load::<tables::Ensemble>(connection)?
                    .into_iter()
                    .map(|e| Ensemble::from_table(e, connection))
                    .collect::<Result<Vec<Ensemble>>>()?;

                let works: Vec<Work> = works::table
                    .inner_join(work_persons::table.inner_join(persons::table))
                    .filter(works::name.like(&search).or(persons::name.like(&search)))
                    .limit(9)
                    .select(works::all_columns)
                    .distinct()
                    .load::<tables::Work>(connection)?
                    .into_iter()
                    .map(|w| Work::from_table(w, connection))
                    .collect::<Result<Vec<Work>>>()?;

                let albums: Vec<Album> = albums::table
                    .filter(albums::name.like(&search))
                    .limit(9)
                    .load(connection)?;

                LibraryResults {
                    composers,
                    performers,
                    ensembles,
                    works,
                    albums,
                    ..Default::default()
                }
            }
            LibraryQuery {
                composer: Some(composer),
                performer: None,
                ensemble: None,
                work: None,
                ..
            } => {
                let performers: Vec<Person> = persons::table
                    .inner_join(recording_persons::table.inner_join(
                        recordings::table.inner_join(works::table.inner_join(work_persons::table)),
                    ))
                    .filter(
                        work_persons::person_id
                            .eq(&composer.person_id)
                            .and(persons::name.like(&search)),
                    )
                    .limit(9)
                    .select(persons::all_columns)
                    .distinct()
                    .load(connection)?;

                let ensembles: Vec<Ensemble> = ensembles::table
                    .inner_join(recording_ensembles::table.inner_join(
                        recordings::table.inner_join(works::table.inner_join(work_persons::table)),
                    ))
                    .filter(
                        work_persons::person_id
                            .eq(&composer.person_id)
                            .and(ensembles::name.like(&search)),
                    )
                    .limit(9)
                    .select(ensembles::all_columns)
                    .distinct()
                    .load::<tables::Ensemble>(connection)?
                    .into_iter()
                    .map(|e| Ensemble::from_table(e, connection))
                    .collect::<Result<Vec<Ensemble>>>()?;

                let works: Vec<Work> = works::table
                    .inner_join(work_persons::table)
                    .filter(
                        work_persons::person_id
                            .eq(&composer.person_id)
                            .and(works::name.like(&search)),
                    )
                    .limit(9)
                    .select(works::all_columns)
                    .distinct()
                    .load::<tables::Work>(connection)?
                    .into_iter()
                    .map(|w| Work::from_table(w, connection))
                    .collect::<Result<Vec<Work>>>()?;

                LibraryResults {
                    performers,
                    ensembles,
                    works,
                    ..Default::default()
                }
            }
            LibraryQuery {
                composer: None,
                performer: None,
                ensemble: Some(ensemble),
                work: None,
                ..
            } => {
                let composers: Vec<Person> =
                    persons::table
                        .inner_join(work_persons::table.inner_join(
                            works::table.inner_join(
                                recordings::table.inner_join(recording_ensembles::table),
                            ),
                        ))
                        .filter(
                            recording_ensembles::ensemble_id
                                .eq(&ensemble.ensemble_id)
                                .and(persons::name.like(&search)),
                        )
                        .limit(9)
                        .select(persons::all_columns)
                        .distinct()
                        .load(connection)?;

                let recordings = recordings::table
                    .inner_join(
                        works::table.inner_join(work_persons::table.inner_join(persons::table)),
                    )
                    // .inner_join(recording_persons::table.inner_join(persons::table))
                    .inner_join(recording_ensembles::table)
                    .filter(
                        recording_ensembles::ensemble_id
                            .eq(&ensemble.ensemble_id)
                            .and(works::name.like(&search).or(persons::name.like(&search))),
                    )
                    .select(recordings::all_columns)
                    .distinct()
                    .load::<tables::Recording>(connection)?
                    .into_iter()
                    .map(|r| Recording::from_table(r, connection))
                    .collect::<Result<Vec<Recording>>>()?;

                let albums = albums::table
                    .inner_join(
                        album_recordings::table
                            .inner_join(recordings::table.inner_join(recording_ensembles::table)),
                    )
                    .filter(
                        recording_ensembles::ensemble_id
                            .eq(&ensemble.ensemble_id)
                            .and(albums::name.like(&search)),
                    )
                    .select(albums::all_columns)
                    .distinct()
                    .load(connection)?;

                LibraryResults {
                    composers,
                    recordings,
                    albums,
                    ..Default::default()
                }
            }
            LibraryQuery {
                composer: None,
                performer: Some(performer),
                work: None,
                ..
            } => {
                let composers: Vec<Person> = persons::table
                    .inner_join(
                        work_persons::table
                            .inner_join(works::table.inner_join(
                                recordings::table.inner_join(recording_persons::table),
                            )),
                    )
                    .filter(
                        recording_persons::person_id
                            .eq(&performer.person_id)
                            .and(persons::name.like(&search)),
                    )
                    .limit(9)
                    .select(persons::all_columns)
                    .distinct()
                    .load(connection)?;

                let recordings = recordings::table
                    .inner_join(
                        works::table.inner_join(work_persons::table.inner_join(persons::table)),
                    )
                    .inner_join(recording_persons::table)
                    .filter(
                        recording_persons::person_id
                            .eq(&performer.person_id)
                            .and(works::name.like(&search).or(persons::name.like(&search))),
                    )
                    .select(recordings::all_columns)
                    .distinct()
                    .load::<tables::Recording>(connection)?
                    .into_iter()
                    .map(|r| Recording::from_table(r, connection))
                    .collect::<Result<Vec<Recording>>>()?;

                let albums = albums::table
                    .inner_join(
                        album_recordings::table
                            .inner_join(recordings::table.inner_join(recording_persons::table)),
                    )
                    .filter(
                        recording_persons::person_id
                            .eq(&performer.person_id)
                            .and(albums::name.like(&search)),
                    )
                    .select(albums::all_columns)
                    .distinct()
                    .load(connection)?;

                LibraryResults {
                    composers,
                    recordings,
                    albums,
                    ..Default::default()
                }
            }
            LibraryQuery {
                composer: Some(composer),
                ensemble: Some(ensemble),
                work: None,
                ..
            } => {
                let recordings = recordings::table
                    .inner_join(works::table.inner_join(work_persons::table))
                    .inner_join(recording_ensembles::table)
                    .filter(
                        work_persons::person_id
                            .eq(&composer.person_id)
                            .and(recording_ensembles::ensemble_id.eq(&ensemble.ensemble_id))
                            .and(works::name.like(search)),
                    )
                    .select(recordings::all_columns)
                    .distinct()
                    .load::<tables::Recording>(connection)?
                    .into_iter()
                    .map(|r| Recording::from_table(r, connection))
                    .collect::<Result<Vec<Recording>>>()?;

                LibraryResults {
                    recordings,
                    ..Default::default()
                }
            }
            LibraryQuery {
                composer: Some(composer),
                performer: Some(performer),
                work: None,
                ..
            } => {
                let recordings = recordings::table
                    .inner_join(works::table.inner_join(work_persons::table))
                    .inner_join(recording_persons::table)
                    .filter(
                        work_persons::person_id
                            .eq(&composer.person_id)
                            .and(recording_persons::person_id.eq(&performer.person_id))
                            .and(works::name.like(search)),
                    )
                    .select(recordings::all_columns)
                    .distinct()
                    .load::<tables::Recording>(connection)?
                    .into_iter()
                    .map(|r| Recording::from_table(r, connection))
                    .collect::<Result<Vec<Recording>>>()?;

                LibraryResults {
                    recordings,
                    ..Default::default()
                }
            }
            LibraryQuery {
                work: Some(work), ..
            } => {
                let recordings = recordings::table
                    .filter(recordings::work_id.eq(&work.work_id))
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
        let mut binding = self.imp().connection.borrow_mut();
        let connection = &mut *binding.as_mut().unwrap();

        let mut query = recordings::table
            .inner_join(works::table.inner_join(work_persons::table))
            .inner_join(recording_persons::table)
            .inner_join(recording_ensembles::table)
            .inner_join(album_recordings::table)
            .into_boxed();

        if let Some(composer_id) = program.composer_id() {
            query = query.filter(work_persons::person_id.eq(composer_id));
        }

        if let Some(performer_id) = program.performer_id() {
            query = query.filter(recording_persons::person_id.eq(performer_id));
        }

        if let Some(ensemble_id) = program.ensemble_id() {
            query = query.filter(recording_ensembles::ensemble_id.eq(ensemble_id));
        }

        if let Some(work_id) = program.work_id() {
            query = query.filter(recordings::work_id.eq(work_id));
        }

        if let Some(album_id) = program.album_id() {
            query = query.filter(album_recordings::album_id.eq(album_id));
        }

        if program.prefer_recently_added() > 0.0 {
            let oldest_timestamp = sql_query(
                "SELECT CAST(STRFTIME('%s', MIN(created_at)) AS INTEGER) AS value FROM recordings",
            )
            .get_result::<IntegerValue>(connection)?
            .value;

            let newest_timestamp = sql_query(
                "SELECT CAST(STRFTIME('%s', MAX(created_at)) AS INTEGER) AS value FROM recordings",
            )
            .get_result::<IntegerValue>(connection)?
            .value;

            let range = newest_timestamp - oldest_timestamp;

            if range >= 60 {
                let proportion = program.prefer_recently_added().max(1.0) * 0.9;
                let cutoff_timestamp =
                    oldest_timestamp + (proportion * range as f64).floor() as i64;

                query = query.filter(
                    sql::<BigInt>("CAST(STRFTIME('%s', recordings.created_at) AS INTEGER)")
                        .ge(cutoff_timestamp)
                        .or(recordings::last_played_at.is_null()),
                );
            }
        }

        if program.prefer_least_recently_played() > 0.0 {
            let oldest_timestamp =
                sql_query("SELECT CAST(STRFTIME('%s', MIN(last_played_at)) AS INTEGER) AS value FROM recordings")
                    .get_result::<IntegerValue>(connection)?
                    .value;

            let newest_timestamp =
                sql_query("SELECT CAST(STRFTIME('%s', MAX(last_played_at)) AS INTEGER) AS value FROM recordings")
                    .get_result::<IntegerValue>(connection)?
                    .value;

            let range = newest_timestamp - oldest_timestamp;

            if range >= 60 {
                let proportion = 1.0 - program.prefer_least_recently_played().max(1.0) * 0.9;
                let cutoff_timestamp =
                    oldest_timestamp + (proportion * range as f64).floor() as i64;

                query = query.filter(
                    sql::<BigInt>("CAST(STRFTIME('%s', recordings.last_played_at) AS INTEGER)")
                        .le(cutoff_timestamp)
                        .or(recordings::last_played_at.is_null()),
                );
            }
        }

        let row = query
            .order(random())
            .select(tables::Recording::as_select())
            .first::<tables::Recording>(connection)?;

        Recording::from_table(row, connection)
    }

    pub fn tracks_for_recording(&self, recording_id: &str) -> Result<Vec<Track>> {
        let mut binding = self.imp().connection.borrow_mut();
        let connection = &mut *binding.as_mut().unwrap();

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
        let mut binding = self.imp().connection.borrow_mut();
        let connection = &mut *binding.as_mut().unwrap();

        let now = Local::now().naive_local();

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

        diesel::update(tracks::table)
            .filter(tracks::track_id.eq(track_id))
            .set(tracks::last_played_at.eq(now))
            .execute(connection)?;

        Ok(())
    }

    pub fn search_persons(&self, search: &str) -> Result<Vec<Person>> {
        let search = format!("%{}%", search);
        let mut binding = self.imp().connection.borrow_mut();
        let connection = &mut *binding.as_mut().unwrap();

        let persons = persons::table
            .order(persons::last_used_at.desc())
            .filter(persons::name.like(&search))
            .limit(20)
            .load(connection)?;

        Ok(persons)
    }

    pub fn search_roles(&self, search: &str) -> Result<Vec<Role>> {
        let search = format!("%{}%", search);
        let mut binding = self.imp().connection.borrow_mut();
        let connection = &mut *binding.as_mut().unwrap();

        let roles = roles::table
            .order(roles::last_used_at.desc())
            .filter(roles::name.like(&search))
            .limit(20)
            .load(connection)?;

        Ok(roles)
    }

    pub fn search_instruments(&self, search: &str) -> Result<Vec<Instrument>> {
        let search = format!("%{}%", search);
        let mut binding = self.imp().connection.borrow_mut();
        let connection = &mut *binding.as_mut().unwrap();

        let instruments = instruments::table
            .order(instruments::last_used_at.desc())
            .filter(instruments::name.like(&search))
            .limit(20)
            .load(connection)?;

        Ok(instruments)
    }

    pub fn search_works(&self, composer: &Person, search: &str) -> Result<Vec<Work>> {
        let search = format!("%{}%", search);
        let mut binding = self.imp().connection.borrow_mut();
        let connection = &mut *binding.as_mut().unwrap();

        let works: Vec<Work> = works::table
            .inner_join(work_persons::table)
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

    pub fn search_ensembles(&self, search: &str) -> Result<Vec<Ensemble>> {
        let search = format!("%{}%", search);
        let mut binding = self.imp().connection.borrow_mut();
        let connection = &mut *binding.as_mut().unwrap();

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
        let mut binding = self.imp().connection.borrow_mut();
        let connection = &mut *binding.as_mut().unwrap();

        Ok(roles::table
            .filter(roles::role_id.eq("380d7e09eb2f49c1a90db2ba4acb6ffd"))
            .first::<Role>(connection)?)
    }

    pub fn performer_default_role(&self) -> Result<Role> {
        let mut binding = self.imp().connection.borrow_mut();
        let connection = &mut *binding.as_mut().unwrap();

        Ok(roles::table
            .filter(roles::role_id.eq("28ff0aeb11c041a6916d93e9b4884eef"))
            .first::<Role>(connection)?)
    }

    pub fn create_person(&self, name: TranslatedString) -> Result<Person> {
        let mut binding = self.imp().connection.borrow_mut();
        let connection = &mut *binding.as_mut().unwrap();

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

        Ok(person)
    }

    pub fn update_person(&self, id: &str, name: TranslatedString) -> Result<()> {
        let mut binding = self.imp().connection.borrow_mut();
        let connection = &mut *binding.as_mut().unwrap();

        let now = Local::now().naive_local();

        diesel::update(persons::table)
            .filter(persons::person_id.eq(id))
            .set((
                persons::name.eq(name),
                persons::edited_at.eq(now),
                persons::last_used_at.eq(now),
            ))
            .execute(connection)?;

        Ok(())
    }

    pub fn create_instrument(&self, name: TranslatedString) -> Result<Instrument> {
        let mut binding = self.imp().connection.borrow_mut();
        let connection = &mut *binding.as_mut().unwrap();

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

        Ok(instrument)
    }

    pub fn update_instrument(&self, id: &str, name: TranslatedString) -> Result<()> {
        let mut binding = self.imp().connection.borrow_mut();
        let connection = &mut *binding.as_mut().unwrap();

        let now = Local::now().naive_local();

        diesel::update(instruments::table)
            .filter(instruments::instrument_id.eq(id))
            .set((
                instruments::name.eq(name),
                instruments::edited_at.eq(now),
                instruments::last_used_at.eq(now),
            ))
            .execute(connection)?;

        Ok(())
    }

    pub fn create_role(&self, name: TranslatedString) -> Result<Role> {
        let mut binding = self.imp().connection.borrow_mut();
        let connection = &mut *binding.as_mut().unwrap();

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

        Ok(role)
    }

    pub fn update_role(&self, id: &str, name: TranslatedString) -> Result<()> {
        let mut binding = self.imp().connection.borrow_mut();
        let connection = &mut *binding.as_mut().unwrap();

        let now = Local::now().naive_local();

        diesel::update(roles::table)
            .filter(roles::role_id.eq(id))
            .set((
                roles::name.eq(name),
                roles::edited_at.eq(now),
                roles::last_used_at.eq(now),
            ))
            .execute(connection)?;

        Ok(())
    }

    pub fn create_work(
        &self,
        name: TranslatedString,
        parts: Vec<WorkPart>,
        persons: Vec<Composer>,
        instruments: Vec<Instrument>,
    ) -> Result<Work> {
        let mut binding = self.imp().connection.borrow_mut();
        let connection = &mut *binding.as_mut().unwrap();

        let work_id = db::generate_id();
        let now = Local::now().naive_local();

        let work_data = tables::Work {
            work_id: work_id.clone(),
            parent_work_id: None,
            sequence_number: None,
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
            let part_data = tables::Work {
                work_id: part.work_id,
                parent_work_id: Some(work_id.clone()),
                sequence_number: Some(index as i32),
                name: part.name,
                created_at: now,
                edited_at: now,
                last_used_at: now,
                last_played_at: None,
            };

            diesel::insert_into(works::table)
                .values(&part_data)
                .execute(connection)?;
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
        id: &str,
        name: TranslatedString,
        parts: Vec<WorkPart>,
        persons: Vec<Composer>,
        instruments: Vec<Instrument>,
    ) -> Result<()> {
        let mut binding = self.imp().connection.borrow_mut();
        let connection = &mut *binding.as_mut().unwrap();

        let now = Local::now().naive_local();

        // TODO: Update work, check which work parts etc exist, update them,
        // create new work parts, delete and readd composers and instruments.
        todo!()
    }

    pub fn create_ensemble(&self, name: TranslatedString) -> Result<Ensemble> {
        let mut binding = self.imp().connection.borrow_mut();
        let connection = &mut *binding.as_mut().unwrap();

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

        Ok(ensemble)
    }

    pub fn update_ensemble(&self, id: &str, name: TranslatedString) -> Result<()> {
        let mut binding = self.imp().connection.borrow_mut();
        let connection = &mut *binding.as_mut().unwrap();

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

        Ok(())
    }

    pub fn create_recording(
        &self,
        work: Work,
        year: Option<i32>,
        performers: Vec<Performer>,
        ensembles: Vec<EnsemblePerformer>,
    ) -> Result<Recording> {
        let mut binding = self.imp().connection.borrow_mut();
        let connection = &mut *binding.as_mut().unwrap();

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

        Ok(recording)
    }

    pub fn update_recording(
        &self,
        id: &str,
        work: Work,
        year: Option<i32>,
        performers: Vec<Performer>,
        ensembles: Vec<EnsemblePerformer>,
    ) -> Result<()> {
        let mut binding = self.imp().connection.borrow_mut();
        let connection = &mut *binding.as_mut().unwrap();

        let now = Local::now().naive_local();

        // TODO: Update recording.
        todo!()
    }
}

#[derive(Default, Debug)]
pub struct LibraryQuery {
    pub composer: Option<Person>,
    pub performer: Option<Person>,
    pub ensemble: Option<Ensemble>,
    pub work: Option<Work>,
    pub search: String,
}

impl LibraryQuery {
    pub fn is_empty(&self) -> bool {
        self.composer.is_none()
            && self.performer.is_none()
            && self.ensemble.is_none()
            && self.work.is_none()
            && self.search.is_empty()
    }
}

#[derive(Default, Debug)]
pub struct LibraryResults {
    pub composers: Vec<Person>,
    pub performers: Vec<Person>,
    pub ensembles: Vec<Ensemble>,
    pub works: Vec<Work>,
    pub recordings: Vec<Recording>,
    pub albums: Vec<Album>,
}

impl LibraryResults {
    pub fn is_empty(&self) -> bool {
        self.composers.is_empty()
            && self.performers.is_empty()
            && self.ensembles.is_empty()
            && self.works.is_empty()
            && self.recordings.is_empty()
            && self.albums.is_empty()
    }
}

#[derive(QueryableByName)]
pub struct IntegerValue {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    pub value: i64,
}
