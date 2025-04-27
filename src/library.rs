use std::{
    cell::OnceCell,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use adw::{
    glib::{self, subclass::Signal, Properties},
    prelude::*,
    subclass::prelude::*,
};
use anyhow::{anyhow, Context, Result};
use diesel::{prelude::*, SqliteConnection};
use once_cell::sync::Lazy;

use crate::db::{self, schema::*, tables};
pub use query::LibraryQuery;

pub mod edit;
pub mod exchange;
pub mod query;

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
    }
}

glib::wrapper! {
    pub struct Library(ObjectSubclass<imp::Library>);
}

impl Library {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let obj: Self = glib::Object::builder()
            .property("folder", path.as_ref().to_str().unwrap())
            .build();

        obj.init()?;
        Ok(obj)
    }

    /// Whether this library is empty. The library is considered empty, if
    /// there are no tracks.
    pub fn is_empty(&self) -> Result<bool> {
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();
        Ok(tracks::table
            .first::<tables::Track>(connection)
            .optional()?
            .is_none())
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

    fn init(&self) -> Result<()> {
        let db_path = PathBuf::from(&self.folder()).join("musicus.db");

        let connection = db::connect(
            db_path
                .to_str()
                .ok_or_else(|| anyhow!("Failed to convert libary path to string"))?,
        )
        .context("Failed to connect to music library database")?;

        self.imp()
            .connection
            .set(Arc::new(Mutex::new(connection)))
            .map_err(|_| anyhow!("Library already initialized"))?;

        Ok(())
    }
}
