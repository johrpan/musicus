use gtk::{glib, glib::Properties, prelude::*, subclass::prelude::*};
use rusqlite::{Connection, Row};
use std::{
    cell::OnceCell,
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
                person: None,
                ensemble: None,
                work: None,
                ..
            } => {
                let persons = self.con()
                    .prepare("SELECT first_name, last_name FROM persons WHERE first_name LIKE ?1 OR last_name LIKE ?1 LIMIT 9")
                    .unwrap()
                    .query_map([&search], Person::from_row)
                    .unwrap()
                    .collect::<rusqlite::Result<Vec<Person>>>()
                    .unwrap();

                LibraryResults {
                    persons,
                    ..Default::default()
                }
            }
            _ => LibraryResults::default(),
        }
    }

    fn con(&self) -> &Connection {
        self.imp().connection.get().unwrap()
    }
}

#[derive(Default)]
pub struct LibraryQuery {
    pub person: Option<Person>,
    pub ensemble: Option<Ensemble>,
    pub work: Option<Work>,
    pub search: String,
}

#[derive(Default)]
pub struct LibraryResults {
    pub persons: Vec<Person>,
    pub ensembles: Vec<Ensemble>,
    pub works: Vec<Work>,
    pub recordings: Vec<Recording>,
}

impl LibraryResults {
    pub fn is_empty(&self) -> bool {
        self.persons.is_empty() && self.ensembles.is_empty() && self.works.is_empty()
    }
}

pub struct Person {
    pub first_name: String,
    pub last_name: String,
}

impl Person {
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            first_name: row.get(0)?,
            last_name: row.get(1)?,
        })
    }

    pub fn name_fl(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
    }
}

pub struct Ensemble {
    pub name: String,
}

pub struct Work {
    pub title: String,
}

pub struct Recording {}
