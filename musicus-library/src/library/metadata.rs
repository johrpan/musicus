//! Importing entities from the metadata catalogue.
//!
//! Every import selects the complete graph the selected entity needs from a
//! validated snapshot of the catalogue, then applies it to the library with
//! the same local-wins rules as archive imports: an entity that already exists
//! locally is kept entirely unchanged, and only entities that are new are
//! written.

use std::{
    fs,
    sync::{Arc, Mutex},
};

use anyhow::anyhow;
use diesel::{prelude::*, SqliteConnection};

use super::{exchange, Library};
use crate::{
    db::{self, tables::Source},
    error::{EntityKind, LibraryError, Result},
    library::{
        import_graph::{apply_import, DestinationState, ImportGraph},
        load::Loader,
    },
    models::*,
};

impl Library {
    /// Get the cached metadata database connection or reopen it if a new
    /// database was downloaded.
    pub fn metadata_connection(&self) -> Option<Arc<Mutex<SqliteConnection>>> {
        let mut metadata_connection = self.metadata_connection.borrow_mut();
        let path = exchange::metadata_file_path(&self.cache_dir);

        if !path.exists() {
            *metadata_connection = None;
            return None;
        }

        // Downloading an updated metadata database replaces the file behind an
        // already open connection, which would otherwise keep serving the
        // contents of the previous download until the app is restarted.
        let modified = fs::metadata(&path).and_then(|m| m.modified()).ok();

        if metadata_connection
            .as_ref()
            .is_none_or(|(cached, _)| *cached != modified)
        {
            match db::connect(&path) {
                Ok(connection) => {
                    *metadata_connection = Some((modified, Arc::new(Mutex::new(connection))));
                }
                Err(err) => {
                    log::error!("Failed to open metadata database: {err:?}");
                    *metadata_connection = None;
                }
            }
        }

        metadata_connection
            .as_ref()
            .map(|(_, connection)| connection.clone())
    }

    pub fn import_metadata_person(&self, person_id: &str) -> Result<Person> {
        self.import_metadata_selection(EntityKind::Person, person_id, |loader, id| {
            loader.person(id)
        })
    }

    pub fn import_metadata_role(&self, role_id: &str) -> Result<Role> {
        self.import_metadata_selection(EntityKind::Role, role_id, |loader, id| loader.role(id))
    }

    pub fn import_metadata_tag(&self, tag_id: &str) -> Result<Tag> {
        self.import_metadata_selection(EntityKind::Tag, tag_id, |loader, id| loader.tag(id))
    }

    pub fn import_metadata_instrument(&self, instrument_id: &str) -> Result<Instrument> {
        self.import_metadata_selection(EntityKind::Instrument, instrument_id, |loader, id| {
            loader.instrument(id)
        })
    }

    pub fn import_metadata_ensemble(&self, ensemble_id: &str) -> Result<Ensemble> {
        self.import_metadata_selection(EntityKind::Ensemble, ensemble_id, |loader, id| {
            loader.ensemble(id)
        })
    }

    pub fn import_metadata_work(&self, work_id: &str) -> Result<Work> {
        self.import_metadata_selection(EntityKind::Work, work_id, |loader, id| loader.work(id))
    }

    pub fn import_metadata_recording(&self, recording_id: &str) -> Result<Recording> {
        self.import_metadata_selection(EntityKind::Recording, recording_id, |loader, id| {
            loader.recording(id)
        })
    }

    /// Import the closure of entities `kind` `id` needs from the metadata
    /// catalogue, then load and return the selected entity.
    ///
    /// The selection is validated against the catalogue before anything is
    /// written, and applied to the library with local-wins semantics in one
    /// transaction.
    fn import_metadata_selection<T>(
        &self,
        kind: EntityKind,
        id: &str,
        load: impl FnOnce(&Loader, &str) -> anyhow::Result<T>,
    ) -> Result<T> {
        let metadata_connection = self
            .metadata_connection()
            .ok_or_else(|| anyhow!("No metadata database available"))?;
        let metadata_connection = &mut *db::lock_connection(&metadata_connection);

        let graph = ImportGraph::load(metadata_connection)?;
        let closure = graph.closure_for(kind, id)?;

        self.commit(|connection| {
            let destination = DestinationState::load(connection)?;
            let plan = graph.plan_merge(&destination, Some(&closure))?;

            connection.transaction::<(), LibraryError, _>(|connection| {
                apply_import(
                    connection,
                    &graph,
                    &plan,
                    &destination,
                    Some(&closure),
                    Source::Metadata,
                    false,
                )?;

                Ok(())
            })?;

            let loader = Loader::load(connection)?;
            Ok(load(&loader, id)?)
        })
    }
}
