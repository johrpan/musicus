use std::{cell::OnceCell, ops::Deref, path::Path, sync::LazyLock};

use adw::{glib, prelude::*, subclass::prelude::*};
use anyhow::{anyhow, Result};

pub use musicus_library::library::{GenerateRecordingParams, LibraryQuery, SearchItem, Tag};

use crate::config;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct Library {
        pub inner: OnceCell<musicus_library::Library>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Library {
        const NAME: &'static str = "MusicusLibrary";
        type Type = super::Library;
    }

    impl ObjectImpl for Library {
        fn signals() -> &'static [glib::subclass::Signal] {
            static SIGNALS: LazyLock<Vec<glib::subclass::Signal>> =
                LazyLock::new(|| vec![glib::subclass::Signal::builder("changed").build()]);

            SIGNALS.as_ref()
        }
    }
}

glib::wrapper! {
    pub struct Library(ObjectSubclass<imp::Library>);
}

impl Deref for Library {
    type Target = musicus_library::Library;

    fn deref(&self) -> &Self::Target {
        self.imp().inner.get().unwrap()
    }
}

impl Library {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let metadata_cache_dir = glib::user_cache_dir().join(config::APP_ID);
        let inner = musicus_library::Library::new(path, metadata_cache_dir)?;
        let changed_rx = inner.subscribe_changed();

        let obj: Self = glib::Object::new();
        obj.imp()
            .inner
            .set(inner)
            .map_err(|_| anyhow!("Library already initialized"))?;

        let obj_clone = obj.clone();
        glib::spawn_future_local(async move {
            while changed_rx.recv().await.is_ok() {
                obj_clone.emit_by_name::<()>("changed", &[]);
            }
        });

        Ok(obj)
    }

    pub fn connect_changed<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("changed", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            f(&obj);
            None
        })
    }
}
