use std::{
    cell::{OnceCell, RefCell},
    path::{Path, PathBuf},
};

use anyhow::Result;
use chrono::prelude::*;
use diesel::{dsl::exists, prelude::*, QueryDsl, SqliteConnection};
use gtk::{glib, glib::Properties, prelude::*, subclass::prelude::*};

use crate::db::{self, models::*, schema::*, tables, TranslatedString};

diesel::sql_function! {
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
            self.connection.set(Some(connection));
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
                    .map(|r| Recording::from_table(r, &&self.folder(), connection))
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
                    .map(|r| Recording::from_table(r, &self.folder(), connection))
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
                    .map(|r| Recording::from_table(r, &self.folder(), connection))
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
                    .map(|r| Recording::from_table(r, &self.folder(), connection))
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
                    .map(|r| Recording::from_table(r, &self.folder(), connection))
                    .collect::<Result<Vec<Recording>>>()?;

                LibraryResults {
                    recordings,
                    ..Default::default()
                }
            }
        })
    }

    pub fn random_recording(&self, query: &LibraryQuery) -> Result<Recording> {
        let mut binding = self.imp().connection.borrow_mut();
        let connection = &mut *binding.as_mut().unwrap();

        match query {
            LibraryQuery { .. } => Recording::from_table(
                recordings::table
                    .order(random())
                    .first::<tables::Recording>(connection)?,
                &self.folder(),
                connection,
            ),
        }
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

    pub fn composer_default_role(&self) -> Result<Role> {
        let mut binding = self.imp().connection.borrow_mut();
        let connection = &mut *binding.as_mut().unwrap();

        Ok(roles::table
            .filter(roles::role_id.eq("380d7e09eb2f49c1a90db2ba4acb6ffd"))
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
}

#[derive(Default, Debug)]
pub struct LibraryQuery {
    pub composer: Option<Person>,
    pub performer: Option<Person>,
    pub ensemble: Option<Ensemble>,
    pub work: Option<Work>,
    pub search: String,
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
