use gtk::{glib, glib::Properties, prelude::*, subclass::prelude::*};
use rusqlite::{Connection, Row};
use std::{
    cell::OnceCell,
    num::ParseIntError,
    path::{Path, PathBuf},
};

mod imp {
    use super::*;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::MusicusLibrary)]
    pub struct MusicusLibrary {
        #[property(get, construct_only)]
        pub folder: OnceCell<String>,
        pub connection: OnceCell<Connection>,
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
            let db_path = PathBuf::from(self.folder.get().unwrap()).join("musicus.db");
            self.connection
                .set(Connection::open(db_path).unwrap())
                .unwrap();
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

    pub fn query(&self, query: &LibraryQuery) -> LibraryResults {
        let search = format!("%{}%", query.search);

        match query {
            LibraryQuery {
                composer: None,
                performer: None,
                ensemble: None,
                work: None,
                ..
            } => {
                let composers = self
                    .con()
                    .prepare(
                        "SELECT DISTINCT persons.id, persons.first_name, persons.last_name \
                        FROM persons \
                            JOIN works ON works.composer = persons.id \
                        WHERE persons.first_name LIKE ?1 OR persons.last_name LIKE ?1 \
                        LIMIT 9",
                    )
                    .unwrap()
                    .query_map([&search], Person::from_row)
                    .unwrap()
                    .collect::<rusqlite::Result<Vec<Person>>>()
                    .unwrap();

                let performers = self
                    .con()
                    .prepare(
                        "SELECT DISTINCT persons.id, persons.first_name, persons.last_name \
                        FROM persons \
                            JOIN performances ON performances.person = persons.id \
                        WHERE persons.first_name LIKE ?1 OR persons.last_name LIKE ?1 \
                        LIMIT 9",
                    )
                    .unwrap()
                    .query_map([&search], Person::from_row)
                    .unwrap()
                    .collect::<rusqlite::Result<Vec<Person>>>()
                    .unwrap();

                let ensembles = self
                    .con()
                    .prepare("SELECT id, name FROM ensembles WHERE name LIKE ?1 LIMIT 9")
                    .unwrap()
                    .query_map([&search], Ensemble::from_row)
                    .unwrap()
                    .collect::<rusqlite::Result<Vec<Ensemble>>>()
                    .unwrap();

                let works = self
                    .con()
                    .prepare(
                        "SELECT works.id, works.title, persons.id, persons.first_name, persons.last_name \
                        FROM works \
                            JOIN persons ON works.composer = persons.id \
                        WHERE title LIKE ?1 \
                        LIMIT 9"
                    )
                    .unwrap()
                    .query_map([&search], Work::from_row)
                    .unwrap()
                    .collect::<rusqlite::Result<Vec<Work>>>()
                    .unwrap();

                LibraryResults {
                    composers,
                    performers,
                    ensembles,
                    works,
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
                let performers = self
                    .con()
                    .prepare(
                        "SELECT DISTINCT persons.id, persons.first_name, persons.last_name \
                        FROM persons \
                            JOIN performances ON performances.person = persons.id \
                            JOIN recordings ON recordings.id = performances.recording \
                            JOIN works ON works.id = recordings.work \
                        WHERE works.composer IS ?1 \
                            AND (persons.first_name LIKE ?2 OR persons.last_name LIKE ?2) \
                        LIMIT 9",
                    )
                    .unwrap()
                    .query_map([&composer.id, &search], Person::from_row)
                    .unwrap()
                    .collect::<rusqlite::Result<Vec<Person>>>()
                    .unwrap();

                let ensembles = self
                    .con()
                    .prepare(
                        "SELECT DISTINCT ensembles.id, ensembles.name \
                        FROM ensembles \
                            JOIN performances ON performances.ensemble = ensembles.id \
                            JOIN recordings ON recordings.id = performances.recording \
                            JOIN works ON works.id = recordings.work \
                        WHERE works.composer IS ?1 AND ensembles.name LIKE ?2 \
                        LIMIT 9",
                    )
                    .unwrap()
                    .query_map([&composer.id, &search], Ensemble::from_row)
                    .unwrap()
                    .collect::<rusqlite::Result<Vec<Ensemble>>>()
                    .unwrap();

                let works = self
                    .con()
                    .prepare(
                        "SELECT DISTINCT works.id, works.title, persons.id, persons.first_name, persons.last_name \
                        FROM works \
                            JOIN persons ON works.composer = persons.id \
                        WHERE works.composer = ?1 AND title LIKE ?2 \
                        LIMIT 9")
                    .unwrap()
                    .query_map([&composer.id, &search], Work::from_row)
                    .unwrap()
                    .collect::<rusqlite::Result<Vec<Work>>>()
                    .unwrap();

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
                let composers = self
                    .con()
                    .prepare(
                        "SELECT DISTINCT persons.id, persons.first_name, persons.last_name \
                        FROM persons \
                            JOIN works ON works.composer = persons.id \
                            JOIN recordings ON recordings.work = works.id \
                            JOIN performances ON performances.recording = recordings.id \
                        WHERE performances.ensemble IS ?1 \
                            AND (persons.first_name LIKE ?2 OR persons.last_name LIKE ?2) \
                        LIMIT 9",
                    )
                    .unwrap()
                    .query_map([&ensemble.id, &search], Person::from_row)
                    .unwrap()
                    .collect::<rusqlite::Result<Vec<Person>>>()
                    .unwrap();

                let recordings = self
                    .con()
                    .prepare(
                        "SELECT DISTINCT recordings.id, works.id, works.title, persons.id, persons.first_name, persons.last_name \
                        FROM recordings \
                            JOIN works ON recordings.work = works.id \
                            JOIN persons ON works.composer = persons.id \
                            JOIN performances ON recordings.id = performances.recording \
                        WHERE performances.ensemble IS ?1 \
                            AND (works.title LIKE ?2 OR persons.first_name LIKE ?2 OR persons.last_name LIKE ?2) \
                        LIMIT 9")
                    .unwrap()
                    .query_map([&ensemble.id, &search], Recording::from_row)
                    .unwrap()
                    .collect::<rusqlite::Result<Vec<Recording>>>()
                    .unwrap();

                LibraryResults {
                    composers,
                    recordings,
                    ..Default::default()
                }
            }
            LibraryQuery {
                composer: None,
                performer: Some(performer),
                work: None,
                ..
            } => {
                let composers = self
                    .con()
                    .prepare(
                        "SELECT DISTINCT persons.id, persons.first_name, persons.last_name \
                        FROM persons \
                            JOIN works ON works.composer = persons.id \
                            JOIN recordings ON recordings.work = works.id \
                            JOIN performances ON performances.recording = recordings.id \
                        WHERE performances.person IS ?1 \
                            AND (persons.first_name LIKE ?2 OR persons.last_name LIKE ?2) \
                        LIMIT 9",
                    )
                    .unwrap()
                    .query_map([&performer.id, &search], Person::from_row)
                    .unwrap()
                    .collect::<rusqlite::Result<Vec<Person>>>()
                    .unwrap();

                let recordings = self
                    .con()
                    .prepare(
                        "SELECT DISTINCT recordings.id, works.id, works.title, persons.id, persons.first_name, persons.last_name \
                        FROM recordings \
                            JOIN works ON recordings.work = works.id \
                            JOIN persons ON works.composer = persons.id \
                            JOIN performances ON recordings.id = performances.recording \
                        WHERE performances.person IS ?1 \
                            AND (works.title LIKE ?2 OR persons.first_name LIKE ?2 OR persons.last_name LIKE ?2) \
                        LIMIT 9")
                    .unwrap()
                    .query_map([&performer.id, &search], Recording::from_row)
                    .unwrap()
                    .collect::<rusqlite::Result<Vec<Recording>>>()
                    .unwrap();

                LibraryResults {
                    composers,
                    recordings,
                    ..Default::default()
                }
            }
            LibraryQuery {
                composer: Some(composer),
                ensemble: Some(ensemble),
                work: None,
                ..
            } => {
                let recordings = self
                    .con()
                    .prepare(
                        "SELECT DISTINCT recordings.id, works.id, works.title, persons.id, persons.first_name, persons.last_name \
                        FROM recordings \
                            JOIN works ON recordings.work = works.id \
                            JOIN persons ON works.composer = persons.id \
                            JOIN performances ON recordings.id = performances.recording \
                        WHERE works.composer IS ?1 \
                            AND performances.ensemble IS ?2 \
                            AND works.title LIKE ?3 \
                        LIMIT 9")
                    .unwrap()
                    .query_map([&composer.id, &ensemble.id, &search], Recording::from_row)
                    .unwrap()
                    .collect::<rusqlite::Result<Vec<Recording>>>()
                    .unwrap();

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
                let recordings = self
                    .con()
                    .prepare(
                        "SELECT DISTINCT recordings.id, works.id, works.title, persons.id, persons.first_name, persons.last_name \
                        FROM recordings \
                            JOIN works ON recordings.work = works.id \
                            JOIN persons ON works.composer = persons.id \
                            JOIN performances ON recordings.id = performances.recording \
                        WHERE works.composer IS ?1 \
                            AND performances.person IS ?2 \
                            AND works.title LIKE ?3 \
                        LIMIT 9")
                    .unwrap()
                    .query_map([&composer.id, &performer.id, &search], Recording::from_row)
                    .unwrap()
                    .collect::<rusqlite::Result<Vec<Recording>>>()
                    .unwrap();

                LibraryResults {
                    recordings,
                    ..Default::default()
                }
            }
            LibraryQuery {
                work: Some(work), ..
            } => {
                let recordings = self
                    .con()
                    .prepare(
                        "SELECT DISTINCT recordings.id, works.id, works.title, persons.id, persons.first_name, persons.last_name \
                        FROM recordings \
                            JOIN works ON recordings.work = works.id \
                            JOIN persons ON works.composer IS persons.id \
                        WHERE works.id IS ?1")
                    .unwrap()
                    .query_map([&work.id], Recording::from_row)
                    .unwrap()
                    .collect::<rusqlite::Result<Vec<Recording>>>()
                    .unwrap();

                LibraryResults {
                    recordings,
                    ..Default::default()
                }
            }
        }
    }

    pub fn work_parts(&self, work: &Work) -> Vec<String> {
        self.con()
            .prepare("SELECT * FROM work_parts WHERE work IS ?1 ORDER BY part_index")
            .unwrap()
            .query_map([&work.id], |row| row.get::<_, String>(3))
            .unwrap()
            .collect::<rusqlite::Result<Vec<String>>>()
            .unwrap()
    }

    pub fn tracks(&self, recording: &Recording) -> Vec<Track> {
        self.con()
            .prepare("SELECT * FROM tracks WHERE recording IS ?1 ORDER BY \"index\"")
            .unwrap()
            .query_map([&recording.id], |row| {
                Ok(Track {
                    work_parts: row
                        .get::<_, String>(4)?
                        .split(',')
                        .filter(|s| !s.is_empty())
                        .map(str::parse::<usize>)
                        .collect::<Result<Vec<usize>, ParseIntError>>()
                        .expect("work part IDs should be valid integers"),
                    path: PathBuf::from(self.folder()).join(row.get::<_, String>(6)?),
                })
            })
            .unwrap()
            .collect::<rusqlite::Result<Vec<Track>>>()
            .unwrap()
    }

    pub fn random_recording(&self, query: &LibraryQuery) -> Option<Recording> {
        match query {
            LibraryQuery { .. } => self
                .con()
                .prepare("SELECT * FROM recordings ORDER BY RANDOM() LIMIT 1")
                .unwrap()
                .query_map([], Recording::from_row)
                .unwrap()
                .next()
                .map(|r| r.unwrap()),
        }
    }

    pub fn performances(&self, recording: &Recording) -> Vec<String> {
        let mut performances = self
            .con()
            .prepare(
                "SELECT persons.id, persons.first_name, persons.last_name, instruments.id, instruments.name \
                FROM performances \
                    INNER JOIN persons ON persons.id = performances.person \
                    LEFT JOIN instruments ON instruments.id = performances.role \
                    INNER JOIN recordings ON performances.recording = recordings.id \
                WHERE recordings.id IS ?1")
            .unwrap()
            .query_map([&recording.id], Performance::from_person_row)
            .unwrap()
            .collect::<rusqlite::Result<Vec<Performance>>>()
            .unwrap();

        performances.append(
            &mut self
                .con()
                .prepare(
                    "SELECT ensembles.id, ensembles.name, instruments.id, instruments.name \
                FROM performances \
                    INNER JOIN ensembles ON ensembles.id = performances.ensemble \
                    LEFT JOIN instruments ON instruments.id = performances.role \
                    INNER JOIN recordings ON performances.recording = recordings.id \
                WHERE recordings.id IS ?1",
                )
                .unwrap()
                .query_map([&recording.id], Performance::from_ensemble_row)
                .unwrap()
                .collect::<rusqlite::Result<Vec<Performance>>>()
                .unwrap(),
        );

        performances
            .into_iter()
            .map(|performance| match performance {
                Performance::Person(person, role) => {
                    let mut result = person.name_fl();
                    if let Some(role) = role {
                        result.push_str(&format!(" ({})", role.name));
                    }
                    result
                }
                Performance::Ensemble(ensemble, role) => {
                    let mut result = ensemble.name;
                    if let Some(role) = role {
                        result.push_str(&format!(" ({})", role.name));
                    }
                    result
                }
            })
            .collect::<Vec<String>>()
    }

    fn con(&self) -> &Connection {
        self.imp().connection.get().unwrap()
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
}

impl LibraryResults {
    pub fn is_empty(&self) -> bool {
        self.composers.is_empty()
            && self.performers.is_empty()
            && self.ensembles.is_empty()
            && self.works.is_empty()
            && self.recordings.is_empty()
    }
}

#[derive(Debug, Clone, Eq)]
pub struct Person {
    pub id: String,
    pub first_name: String,
    pub last_name: String,
}

impl PartialEq for Person {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Person {
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            first_name: row.get(1)?,
            last_name: row.get(2)?,
        })
    }

    pub fn name_fl(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
    }
}

#[derive(Debug, Clone, Eq)]
pub struct Ensemble {
    pub id: String,
    pub name: String,
}

impl PartialEq for Ensemble {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Ensemble {
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            name: row.get(1)?,
        })
    }
}

#[derive(Debug, Clone, Eq)]
pub struct Work {
    pub id: String,
    pub title: String,
    pub composer: Person,
}

impl PartialEq for Work {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Work {
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            title: row.get(1)?,
            composer: Person {
                id: row.get(2)?,
                first_name: row.get(3)?,
                last_name: row.get(4)?,
            },
        })
    }
}

#[derive(Debug, Clone, Eq)]
pub struct Recording {
    pub id: String,
    pub work: Work,
}

impl PartialEq for Recording {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Recording {
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            work: Work {
                id: row.get(1)?,
                title: row.get(2)?,
                composer: Person {
                    id: row.get(3)?,
                    first_name: row.get(4)?,
                    last_name: row.get(5)?,
                },
            },
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Performance {
    Person(Person, Option<Role>),
    Ensemble(Ensemble, Option<Role>),
}

impl Performance {
    pub fn from_person_row(row: &Row) -> rusqlite::Result<Self> {
        let person = Person {
            id: row.get(0)?,
            first_name: row.get(1)?,
            last_name: row.get(2)?,
        };

        Ok(match row.get::<_, Option<String>>(3)? {
            None => Self::Person(person, None),
            Some(role_id) => Self::Person(
                person,
                Some(Role {
                    id: role_id,
                    name: row.get(4)?,
                }),
            ),
        })
    }

    pub fn from_ensemble_row(row: &Row) -> rusqlite::Result<Self> {
        let ensemble = Ensemble {
            id: row.get(0)?,
            name: row.get(1)?,
        };

        Ok(match row.get::<_, Option<String>>(2)? {
            None => Self::Ensemble(ensemble, None),
            Some(role_id) => Self::Ensemble(
                ensemble,
                Some(Role {
                    id: role_id,
                    name: row.get(3)?,
                }),
            ),
        })
    }
}

#[derive(Debug, Clone, Eq)]
pub struct Role {
    pub id: String,
    pub name: String,
}

impl PartialEq for Role {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Debug, Clone)]
pub struct Track {
    pub work_parts: Vec<usize>,
    pub path: PathBuf,
}
