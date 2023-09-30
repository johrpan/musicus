use crate::db::{self, SqliteConnection};
use gtk::{glib, glib::Properties, prelude::*, subclass::prelude::*};
use std::{
    cell::{OnceCell, RefCell},
    path::Path,
};

mod imp {
    use super::*;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::MusicusLibrary)]
    pub struct MusicusLibrary {
        #[property(get, set)]
        pub folder: RefCell<String>,
        pub connection: OnceCell<SqliteConnection>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MusicusLibrary {
        const NAME: &'static str = "MusicusLibrary";
        type Type = super::MusicusLibrary;
    }

    #[glib::derived_properties]
    impl ObjectImpl for MusicusLibrary {}
}

glib::wrapper! {
    pub struct MusicusLibrary(ObjectSubclass<imp::MusicusLibrary>);
}

impl MusicusLibrary {
    pub fn new(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref();
        let obj: MusicusLibrary = glib::Object::builder()
            .property("folder", path.to_str().unwrap())
            .build();

        let connection = db::connect(path.join("musicus.db").to_str().unwrap()).unwrap();

        obj.imp()
            .connection
            .set(connection)
            .unwrap_or_else(|_| panic!("Database connection already set"));

        obj
    }

    pub fn db(&self) -> &SqliteConnection {
        self.imp().connection.get().unwrap()
    }
}
