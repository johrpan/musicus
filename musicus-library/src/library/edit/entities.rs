//! CRUD for library entities.

use std::collections::HashSet;

use anyhow::{Error, Result};
use diesel::{prelude::*, SqliteConnection};

use crate::{
    db::{self, schema::*, tables, tables::Source, TranslatedString},
    error::{EntityKind, LibraryError},
    input::*,
    library::{load::Loader, Library},
    models::*,
};

use super::{
    assoc,
    common::{delete_entity, fresh_timestamps, update_entity},
};

impl Library {
    pub fn create_person(&self, name: TranslatedString, enable_updates: bool) -> Result<Person> {
        let (created_at, edited_at, last_used_at) = fresh_timestamps();

        let person = tables::Person {
            person_id: db::generate_id(),
            name,
            source: Source::User,
            created_at,
            edited_at,
            last_used_at,
            enable_updates,
        };

        self.commit(|connection| {
            diesel::insert_into(persons::table)
                .values(&person)
                .execute(connection)?;

            Ok(Person::from(person))
        })
    }

    pub fn update_person(
        &self,
        id: &str,
        name: TranslatedString,
        enable_updates: bool,
    ) -> Result<()> {
        self.commit(|connection| {
            update_entity(
                connection,
                EntityKind::Person,
                id,
                |connection, edited_at| {
                    diesel::update(persons::table)
                        .filter(persons::person_id.eq(id))
                        .set((
                            persons::name.eq(name),
                            persons::edited_at.eq(edited_at),
                            persons::last_used_at.eq(edited_at),
                            persons::enable_updates.eq(enable_updates),
                        ))
                        .execute(connection)
                },
            )
        })
    }

    pub fn delete_person(&self, person_id: &str) -> Result<()> {
        self.commit(|connection| {
            delete_entity(connection, EntityKind::Person, person_id, |connection| {
                diesel::delete(persons::table)
                    .filter(persons::person_id.eq(person_id))
                    .execute(connection)
            })
        })
    }

    pub fn create_instrument(
        &self,
        name: TranslatedString,
        enable_updates: bool,
    ) -> Result<Instrument> {
        let (created_at, edited_at, last_used_at) = fresh_timestamps();

        let instrument = tables::Instrument {
            instrument_id: db::generate_id(),
            name,
            source: Source::User,
            created_at,
            edited_at,
            last_used_at,
            enable_updates,
        };

        self.commit(|connection| {
            diesel::insert_into(instruments::table)
                .values(&instrument)
                .execute(connection)?;

            Ok(Instrument::from(instrument))
        })
    }

    pub fn update_instrument(
        &self,
        id: &str,
        name: TranslatedString,
        enable_updates: bool,
    ) -> Result<()> {
        self.commit(|connection| {
            update_entity(
                connection,
                EntityKind::Instrument,
                id,
                |connection, edited_at| {
                    diesel::update(instruments::table)
                        .filter(instruments::instrument_id.eq(id))
                        .set((
                            instruments::name.eq(name),
                            instruments::edited_at.eq(edited_at),
                            instruments::last_used_at.eq(edited_at),
                            instruments::enable_updates.eq(enable_updates),
                        ))
                        .execute(connection)
                },
            )
        })
    }

    pub fn delete_instrument(&self, instrument_id: &str) -> Result<()> {
        self.commit(|connection| {
            delete_entity(
                connection,
                EntityKind::Instrument,
                instrument_id,
                |connection| {
                    diesel::delete(instruments::table)
                        .filter(instruments::instrument_id.eq(instrument_id))
                        .execute(connection)
                },
            )
        })
    }

    pub fn create_role(&self, name: TranslatedString, enable_updates: bool) -> Result<Role> {
        let (created_at, edited_at, last_used_at) = fresh_timestamps();

        let role = tables::Role {
            role_id: db::generate_id(),
            name,
            source: Source::User,
            created_at,
            edited_at,
            last_used_at,
            enable_updates,
        };

        self.commit(|connection| {
            diesel::insert_into(roles::table)
                .values(&role)
                .execute(connection)?;

            Ok(Role::from(role))
        })
    }

    pub fn update_role(
        &self,
        id: &str,
        name: TranslatedString,
        enable_updates: bool,
    ) -> Result<()> {
        self.commit(|connection| {
            update_entity(connection, EntityKind::Role, id, |connection, edited_at| {
                diesel::update(roles::table)
                    .filter(roles::role_id.eq(id))
                    .set((
                        roles::name.eq(name),
                        roles::edited_at.eq(edited_at),
                        roles::last_used_at.eq(edited_at),
                        roles::enable_updates.eq(enable_updates),
                    ))
                    .execute(connection)
            })
        })
    }

    pub fn delete_role(&self, role_id: &str) -> Result<()> {
        self.commit(|connection| {
            delete_entity(connection, EntityKind::Role, role_id, |connection| {
                diesel::delete(roles::table)
                    .filter(roles::role_id.eq(role_id))
                    .execute(connection)
            })
        })
    }

    /// Create a new work from `input`.
    ///
    /// A child node with an ID that already exists belongs to another work and
    /// is rejected; children without an existing ID are created fresh. The
    /// root must not carry an ID — use [`Library::update_work`] for that.
    pub fn create_work(&self, input: WorkInput) -> Result<Work> {
        if let Some(id) = &input.id {
            return Err(LibraryError::InvalidInput(format!(
                "A new work cannot carry the existing ID {id}"
            ))
            .into());
        }

        self.commit(|connection| {
            let mut validators = Validator::new(connection)?;
            validators.validate_work_input(&input, None)?;

            let work_id = connection.transaction::<String, Error, _>(|connection| {
                let work_id = db::generate_id();

                Self::create_work_priv(connection, work_id.clone(), &input, None, None)?;

                Ok(work_id)
            })?;

            let loader = Loader::load(connection)?;

            loader.work(&work_id)
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn create_work_priv(
        connection: &mut SqliteConnection,
        work_id: String,
        input: &WorkInput,
        parent_work_id: Option<&str>,
        sequence_number: Option<i32>,
    ) -> Result<()> {
        let (created_at, edited_at, last_used_at) = fresh_timestamps();

        let work_data = tables::Work {
            work_id: work_id.clone(),
            parent_work_id: parent_work_id.map(str::to_owned),
            sequence_number,
            name: input.name.clone(),
            source: Source::User,
            created_at,
            edited_at,
            last_used_at,
            enable_updates: input.enable_updates,
        };

        diesel::insert_into(works::table)
            .values(&work_data)
            .execute(connection)?;

        for (index, part) in input.parts.iter().enumerate() {
            Self::create_work_priv(
                connection,
                db::generate_id(),
                part,
                Some(&work_id),
                Some(db::sequence_number(index)?),
            )?;
        }

        assoc::replace_work_relations(connection, &work_id, &input.related_work_ids)?;
        assoc::replace_work_tags(connection, &work_id, &input.tags)?;
        assoc::replace_work_persons(connection, &work_id, &input.work_persons)?;
        assoc::replace_work_instruments(connection, &work_id, &input.instrument_ids)?;

        Ok(())
    }

    /// Update the work identified by `id` from `input`.
    ///
    /// A child node with an ID that exists and already belongs to `id` is
    /// updated in place; a child node with an ID that exists elsewhere (or is
    /// a root work) is rejected rather than silently moved; any other child
    /// node is created fresh.
    pub fn update_work(&self, id: &str, input: WorkInput) -> Result<()> {
        if let Some(input_id) = &input.id {
            if input_id != id {
                return Err(LibraryError::InvalidInput(format!(
                    "The input for work {id} carries a different ID {input_id}"
                ))
                .into());
            }
        }

        self.commit(|connection| {
            let mut validators = Validator::new(connection)?;
            validators.validate_work_input(&input, Some(id))?;

            connection.transaction::<(), Error, _>(|connection| {
                Self::update_work_priv(connection, id, &input, None, None)
            })
        })
    }

    fn update_work_priv(
        connection: &mut SqliteConnection,
        work_id: &str,
        input: &WorkInput,
        parent_work_id: Option<&str>,
        sequence_number: Option<i32>,
    ) -> Result<()> {
        update_entity(
            connection,
            EntityKind::Work,
            work_id,
            |connection, edited_at| {
                diesel::update(works::table)
                    .filter(works::work_id.eq(work_id))
                    .set((
                        works::parent_work_id.eq(parent_work_id),
                        works::sequence_number.eq(sequence_number),
                        works::name.eq(input.name.clone()),
                        works::edited_at.eq(edited_at),
                        works::last_used_at.eq(edited_at),
                        works::enable_updates.eq(input.enable_updates),
                    ))
                    .execute(connection)
            },
        )?;

        // Remove children the input no longer lists.
        let requested_ids = input
            .parts
            .iter()
            .filter_map(|part| part.id.clone())
            .collect::<HashSet<String>>();

        diesel::delete(
            works::table
                .filter(works::parent_work_id.eq(work_id))
                .filter(works::work_id.ne_all(requested_ids.iter().map(String::as_str))),
        )
        .execute(connection)?;

        for (index, part) in input.parts.iter().enumerate() {
            let sequence = Some(db::sequence_number(index)?);

            match &part.id {
                Some(part_id) if work_exists(connection, part_id)? => {
                    Self::update_work_priv(connection, part_id, part, Some(work_id), sequence)?;
                }
                // A new child (an ID that exists nowhere), or a create that
                // carries a fabricated ID.
                _ => {
                    Self::create_work_priv(
                        connection,
                        db::generate_id(),
                        part,
                        Some(work_id),
                        sequence,
                    )?;
                }
            }
        }

        assoc::replace_work_relations(connection, work_id, &input.related_work_ids)?;
        assoc::replace_work_persons(connection, work_id, &input.work_persons)?;
        assoc::replace_work_instruments(connection, work_id, &input.instrument_ids)?;
        assoc::replace_work_tags(connection, work_id, &input.tags)?;

        Ok(())
    }

    pub fn delete_work(&self, work_id: &str) -> Result<()> {
        self.commit(|connection| {
            delete_entity(connection, EntityKind::Work, work_id, |connection| {
                diesel::delete(works::table)
                    .filter(works::work_id.eq(work_id))
                    .execute(connection)
            })
        })
    }

    pub fn create_ensemble(&self, input: EnsembleInput) -> Result<Ensemble> {
        self.commit(|connection| {
            let mut validators = Validator::new(connection)?;
            validators.validate_member_inputs(&input.members)?;

            let ensemble_id = connection.transaction::<String, Error, _>(|connection| {
                let ensemble_id = db::generate_id();
                let (created_at, edited_at, last_used_at) = fresh_timestamps();

                let ensemble_data = tables::Ensemble {
                    ensemble_id: ensemble_id.clone(),
                    name: input.name.clone(),
                    source: Source::User,
                    created_at,
                    edited_at,
                    last_used_at,
                    enable_updates: input.enable_updates,
                };

                diesel::insert_into(ensembles::table)
                    .values(&ensemble_data)
                    .execute(connection)?;

                assoc::replace_ensemble_persons(connection, &ensemble_id, &input.members)?;

                Ok(ensemble_id)
            })?;

            let loader = Loader::load(connection)?;

            loader.ensemble(&ensemble_id)
        })
    }

    pub fn update_ensemble(&self, id: &str, input: EnsembleInput) -> Result<()> {
        self.commit(|connection| {
            let mut validators = Validator::new(connection)?;
            validators.validate_member_inputs(&input.members)?;

            connection.transaction::<(), Error, _>(|connection| {
                update_entity(
                    connection,
                    EntityKind::Ensemble,
                    id,
                    |connection, edited_at| {
                        diesel::update(ensembles::table)
                            .filter(ensembles::ensemble_id.eq(id))
                            .set((
                                ensembles::name.eq(input.name.clone()),
                                ensembles::edited_at.eq(edited_at),
                                ensembles::last_used_at.eq(edited_at),
                                ensembles::enable_updates.eq(input.enable_updates),
                            ))
                            .execute(connection)
                    },
                )?;

                assoc::replace_ensemble_persons(connection, id, &input.members)
            })
        })
    }

    pub fn delete_ensemble(&self, ensemble_id: &str) -> Result<()> {
        self.commit(|connection| {
            delete_entity(
                connection,
                EntityKind::Ensemble,
                ensemble_id,
                |connection| {
                    diesel::delete(ensembles::table)
                        .filter(ensembles::ensemble_id.eq(ensemble_id))
                        .execute(connection)
                },
            )
        })
    }

    pub fn create_recording(&self, input: RecordingInput) -> Result<Recording> {
        self.commit(|connection| {
            let mut validators = Validator::new(connection)?;
            validators.validate_recording_input(&input)?;

            let recording_id = connection.transaction::<String, Error, _>(|connection| {
                let recording_id = db::generate_id();
                let (created_at, edited_at, last_used_at) = fresh_timestamps();

                let recording_data = tables::Recording {
                    recording_id: recording_id.clone(),
                    work_id: input.work_id.clone(),
                    source: Source::User,
                    created_at,
                    edited_at,
                    last_used_at,
                    comment: input.comment.clone(),
                    enable_updates: input.enable_updates,
                };

                diesel::insert_into(recordings::table)
                    .values(&recording_data)
                    .execute(connection)?;

                assoc::replace_recording_persons(
                    connection,
                    &recording_id,
                    &input.recording_persons,
                )?;
                assoc::replace_recording_ensembles(
                    connection,
                    &recording_id,
                    &input.recording_ensembles,
                )?;
                assoc::replace_recording_tags(connection, &recording_id, &input.tags)?;

                Ok(recording_id)
            })?;

            let loader = Loader::load(connection)?;

            loader.recording(&recording_id)
        })
    }

    pub fn update_recording(&self, recording_id: &str, input: RecordingInput) -> Result<()> {
        self.commit(|connection| {
            let mut validators = Validator::new(connection)?;
            validators.validate_recording_input(&input)?;

            connection.transaction::<(), Error, _>(|connection| {
                update_entity(
                    connection,
                    EntityKind::Recording,
                    recording_id,
                    |connection, edited_at| {
                        diesel::update(recordings::table)
                            .filter(recordings::recording_id.eq(recording_id))
                            .set((
                                recordings::work_id.eq(input.work_id.clone()),
                                recordings::edited_at.eq(edited_at),
                                recordings::last_used_at.eq(edited_at),
                                recordings::comment.eq(input.comment.clone()),
                                recordings::enable_updates.eq(input.enable_updates),
                            ))
                            .execute(connection)
                    },
                )?;

                assoc::replace_recording_persons(
                    connection,
                    recording_id,
                    &input.recording_persons,
                )?;
                assoc::replace_recording_ensembles(
                    connection,
                    recording_id,
                    &input.recording_ensembles,
                )?;
                assoc::replace_recording_tags(connection, recording_id, &input.tags)
            })
        })
    }

    pub fn delete_recording(&self, recording_id: &str) -> Result<()> {
        self.commit(|connection| {
            delete_entity(
                connection,
                EntityKind::Recording,
                recording_id,
                |connection| {
                    diesel::delete(recordings::table)
                        .filter(recordings::recording_id.eq(recording_id))
                        .execute(connection)
                },
            )
        })
    }

    pub fn create_album(&self, input: AlbumInput) -> Result<Album> {
        self.commit(|connection| {
            let mut validators = Validator::new(connection)?;
            validators.validate_recording_ids(&input.recording_ids)?;

            let album_id = connection.transaction::<String, Error, _>(|connection| {
                let album_id = db::generate_id();
                let (created_at, edited_at, last_used_at) = fresh_timestamps();

                let album_data = tables::Album {
                    album_id: album_id.clone(),
                    name: input.name.clone(),
                    source: Source::User,
                    enable_updates: input.enable_updates,
                    created_at,
                    edited_at,
                    last_used_at,
                };

                diesel::insert_into(albums::table)
                    .values(&album_data)
                    .execute(connection)?;

                assoc::replace_album_recordings(connection, &album_id, &input.recording_ids)?;

                Ok(album_id)
            })?;

            let loader = Loader::load(connection)?;

            loader.album(&album_id)
        })
    }

    pub fn update_album(&self, album_id: &str, input: AlbumInput) -> Result<()> {
        self.commit(|connection| {
            let mut validators = Validator::new(connection)?;
            validators.validate_recording_ids(&input.recording_ids)?;

            connection.transaction::<(), Error, _>(|connection| {
                update_entity(
                    connection,
                    EntityKind::Album,
                    album_id,
                    |connection, edited_at| {
                        diesel::update(albums::table)
                            .filter(albums::album_id.eq(album_id))
                            .set((
                                albums::name.eq(input.name.clone()),
                                albums::enable_updates.eq(input.enable_updates),
                                albums::edited_at.eq(edited_at),
                                albums::last_used_at.eq(edited_at),
                            ))
                            .execute(connection)
                    },
                )?;

                assoc::replace_album_recordings(connection, album_id, &input.recording_ids)
            })
        })
    }

    pub fn delete_album(&self, album_id: &str) -> Result<()> {
        self.commit(|connection| {
            delete_entity(connection, EntityKind::Album, album_id, |connection| {
                diesel::delete(albums::table)
                    .filter(albums::album_id.eq(album_id))
                    .execute(connection)
            })
        })
    }

    /// Mark an entity as explicitly used, updating its `last_used_at` for
    /// recency ordering.
    ///
    /// Merely loading an entity for display does not touch it; the application
    /// calls this when the user selects an item and wants the library to
    /// remember the selection. Tracks have no recency ordering of their own —
    /// listening history is represented by `plays`.
    pub fn touch(&self, kind: EntityKind, id: &str) -> Result<()> {
        self.commit(|connection| touch_entity(connection, kind, id))
    }
}

/// Update only `last_used_at` of an existing entity.
fn touch_entity(connection: &mut SqliteConnection, kind: EntityKind, id: &str) -> Result<()> {
    let now = db::now();

    let updated = match kind {
        EntityKind::Person => diesel::update(persons::table.filter(persons::person_id.eq(id)))
            .set(persons::last_used_at.eq(now))
            .execute(connection)?,
        EntityKind::Role => diesel::update(roles::table.filter(roles::role_id.eq(id)))
            .set(roles::last_used_at.eq(now))
            .execute(connection)?,
        EntityKind::Instrument => {
            diesel::update(instruments::table.filter(instruments::instrument_id.eq(id)))
                .set(instruments::last_used_at.eq(now))
                .execute(connection)?
        }
        EntityKind::Tag => diesel::update(tags::table.filter(tags::tag_id.eq(id)))
            .set(tags::last_used_at.eq(now))
            .execute(connection)?,
        EntityKind::Work => diesel::update(works::table.filter(works::work_id.eq(id)))
            .set(works::last_used_at.eq(now))
            .execute(connection)?,
        EntityKind::Ensemble => {
            diesel::update(ensembles::table.filter(ensembles::ensemble_id.eq(id)))
                .set(ensembles::last_used_at.eq(now))
                .execute(connection)?
        }
        EntityKind::Recording => {
            diesel::update(recordings::table.filter(recordings::recording_id.eq(id)))
                .set(recordings::last_used_at.eq(now))
                .execute(connection)?
        }
        EntityKind::Album => diesel::update(albums::table.filter(albums::album_id.eq(id)))
            .set(albums::last_used_at.eq(now))
            .execute(connection)?,
        EntityKind::Track => {
            return Err(LibraryError::InvalidInput(
                "Tracks have no recency ordering; listening history is `plays`".to_owned(),
            )
            .into())
        }
    };

    if updated == 0 {
        return Err(LibraryError::NotFound(kind, id.to_owned()).into());
    }

    Ok(())
}

/// One-pass validation of an input against the current database state,
/// before any mutation starts.
struct Validator<'a> {
    connection: &'a mut SqliteConnection,
}

impl<'a> Validator<'a> {
    fn new(connection: &'a mut SqliteConnection) -> Result<Self> {
        Ok(Self { connection })
    }

    /// Validate a whole work input, recursively.
    ///
    /// `parent_id` is the database ID of the work this input is (or would be)
    /// a child of: `None` for the root of a create.
    fn validate_work_input(&mut self, input: &WorkInput, parent_id: Option<&str>) -> Result<()> {
        if let Some(id) = &input.id {
            // An update may only change a child that already belongs to this
            // parent; a work that exists elsewhere (or is a root work that
            // this operation has no right to adopt) must not be silently
            // reparented.
            let stored_parent = works::table
                .filter(works::work_id.eq(id))
                .select(works::parent_work_id)
                .first::<Option<String>>(self.connection)
                .optional()?;

            match stored_parent {
                // The ID is not in the database: a new child whose caller-side
                // ID is only a placeholder.
                None => {}
                Some(parent) if parent.as_deref() != parent_id => {
                    return Err(LibraryError::InvalidInput(format!(
                        "Work {id} belongs to another parent and cannot be moved here"
                    ))
                    .into())
                }
                Some(_) => {}
            }
        }

        let mut seen_ids = HashSet::new();
        for part in &input.parts {
            if let Some(id) = &part.id {
                if !seen_ids.insert(id.clone()) {
                    return Err(LibraryError::InvalidInput(format!(
                        "The input contains the child {id} twice"
                    ))
                    .into());
                }
            }
        }

        // An existing child's own children must belong to that child; a new
        // child may not adopt existing works at all.
        let children_parent = input.id.as_deref().or(parent_id);

        for part in &input.parts {
            self.validate_work_input(part, children_parent)?;
        }

        for work_person in &input.work_persons {
            self.validate_person(&work_person.person_id)?;
            self.validate_role(work_person.role_id.as_deref())?;
        }

        for instrument_id in &input.instrument_ids {
            self.validate_instrument(instrument_id)?;
        }

        self.validate_tags(&input.tags)?;

        Ok(())
    }

    fn validate_person(&mut self, id: &str) -> Result<()> {
        if !person_exists(self.connection, id)? {
            return Err(LibraryError::NotFound(EntityKind::Person, id.to_owned()).into());
        }
        Ok(())
    }

    fn validate_role(&mut self, id: Option<&str>) -> Result<()> {
        if let Some(id) = id {
            if !role_exists(self.connection, id)? {
                return Err(LibraryError::NotFound(EntityKind::Role, id.to_owned()).into());
            }
        }
        Ok(())
    }

    fn validate_instrument(&mut self, id: &str) -> Result<()> {
        if !instrument_exists(self.connection, id)? {
            return Err(LibraryError::NotFound(EntityKind::Instrument, id.to_owned()).into());
        }
        Ok(())
    }

    fn validate_member_inputs(&mut self, members: &[RecordingPersonInput]) -> Result<()> {
        for member in members {
            self.validate_person(&member.person_id)?;
            self.validate_role(member.role_id.as_deref())?;

            if let Some(instrument_id) = &member.instrument_id {
                self.validate_instrument(instrument_id)?;
            }
        }
        Ok(())
    }

    fn validate_recording_input(&mut self, input: &RecordingInput) -> Result<()> {
        if !work_exists(self.connection, &input.work_id)? {
            return Err(LibraryError::NotFound(EntityKind::Work, input.work_id.clone()).into());
        }

        for person in &input.recording_persons {
            self.validate_person(&person.person_id)?;
            self.validate_role(person.role_id.as_deref())?;

            if let Some(instrument_id) = &person.instrument_id {
                self.validate_instrument(instrument_id)?;
            }
        }

        for ensemble in &input.recording_ensembles {
            if !ensemble_exists(self.connection, &ensemble.ensemble_id)? {
                return Err(LibraryError::NotFound(
                    EntityKind::Ensemble,
                    ensemble.ensemble_id.clone(),
                )
                .into());
            }
            self.validate_role(ensemble.role_id.as_deref())?;
        }

        self.validate_tags(&input.tags)?;

        Ok(())
    }

    fn validate_recording_ids(&mut self, recording_ids: &[String]) -> Result<()> {
        let mut seen = HashSet::new();
        for recording_id in recording_ids {
            if !seen.insert(recording_id.clone()) {
                return Err(LibraryError::InvalidInput(format!(
                    "Recording {recording_id} appears twice in the album"
                ))
                .into());
            }

            if !recording_exists(self.connection, recording_id)? {
                return Err(
                    LibraryError::NotFound(EntityKind::Recording, recording_id.clone()).into(),
                );
            }
        }
        Ok(())
    }

    fn validate_tags(&mut self, tags: &[TagInput]) -> Result<()> {
        let tag_ids = tags
            .iter()
            .map(|tag| tag.tag_id.clone())
            .collect::<Vec<_>>();

        let takes_value = tags::table
            .filter(tags::tag_id.eq_any(&tag_ids))
            .select((tags::tag_id, tags::takes_value))
            .load::<(String, bool)>(self.connection)?;

        for tag in tags {
            let stored = match takes_value.iter().find(|(id, _)| id == &tag.tag_id) {
                Some(stored) => stored,
                None => {
                    return Err(LibraryError::NotFound(EntityKind::Tag, tag.tag_id.clone()).into())
                }
            };

            match (stored.1, tag.value.is_some()) {
                (false, true) => {
                    return Err(LibraryError::InvalidInput(format!(
                        "Tag {} does not take a value",
                        tag.tag_id
                    ))
                    .into())
                }
                (true, false) => {
                    return Err(LibraryError::InvalidInput(format!(
                        "Tag {} requires a value",
                        tag.tag_id
                    ))
                    .into())
                }
                _ => {}
            }
        }

        Ok(())
    }
}

fn work_exists(connection: &mut SqliteConnection, id: &str) -> Result<bool> {
    Ok(works::table
        .filter(works::work_id.eq(id))
        .select(works::work_id)
        .first::<String>(connection)
        .optional()?
        .is_some())
}

fn person_exists(connection: &mut SqliteConnection, id: &str) -> Result<bool> {
    Ok(persons::table
        .filter(persons::person_id.eq(id))
        .select(persons::person_id)
        .first::<String>(connection)
        .optional()?
        .is_some())
}

fn role_exists(connection: &mut SqliteConnection, id: &str) -> Result<bool> {
    Ok(roles::table
        .filter(roles::role_id.eq(id))
        .select(roles::role_id)
        .first::<String>(connection)
        .optional()?
        .is_some())
}

fn instrument_exists(connection: &mut SqliteConnection, id: &str) -> Result<bool> {
    Ok(instruments::table
        .filter(instruments::instrument_id.eq(id))
        .select(instruments::instrument_id)
        .first::<String>(connection)
        .optional()?
        .is_some())
}

fn ensemble_exists(connection: &mut SqliteConnection, id: &str) -> Result<bool> {
    Ok(ensembles::table
        .filter(ensembles::ensemble_id.eq(id))
        .select(ensembles::ensemble_id)
        .first::<String>(connection)
        .optional()?
        .is_some())
}

fn recording_exists(connection: &mut SqliteConnection, id: &str) -> Result<bool> {
    Ok(recordings::table
        .filter(recordings::recording_id.eq(id))
        .select(recordings::recording_id)
        .first::<String>(connection)
        .optional()?
        .is_some())
}
