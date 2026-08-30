//! Editing tags: creating, updating, deleting and (bulk) assigning tags to
//! works and recordings.

use diesel::{dsl::exists, prelude::*, QueryDsl, SqliteConnection};

use crate::{
    db::{self, schema::*, tables, tables::Source, TranslatedString},
    error::{EntityKind, LibraryError, Result},
    library::Library,
    models::*,
};

use super::common::{delete_entity, fresh_timestamps, update_entity};

impl Library {
    pub fn create_tag(
        &self,
        name: TranslatedString,
        takes_value: bool,
        private: bool,
        enable_updates: bool,
    ) -> Result<Tag> {
        let (created_at, edited_at, last_used_at) = fresh_timestamps();

        let tag = tables::Tag {
            tag_id: db::generate_id(),
            name,
            takes_value,
            source: Source::User,
            created_at,
            edited_at,
            last_used_at,
            enable_updates,
            private,
        };

        self.commit(|connection| {
            diesel::insert_into(tags::table)
                .values(&tag)
                .execute(connection)?;

            Ok(Tag::from(tag))
        })
    }

    /// Whether any work or recording is tagged with this tag.
    pub fn tag_is_in_use(&self, tag_id: &str) -> Result<bool> {
        let connection = &mut *self.conn();
        Self::tag_is_in_use_priv(connection, tag_id)
    }

    fn tag_is_in_use_priv(connection: &mut SqliteConnection, tag_id: &str) -> Result<bool> {
        Ok(diesel::select(exists(
            work_tags::table.filter(work_tags::tag_id.eq(tag_id)),
        ))
        .get_result::<bool>(connection)?
            || diesel::select(exists(
                recording_tags::table.filter(recording_tags::tag_id.eq(tag_id)),
            ))
            .get_result::<bool>(connection)?)
    }

    /// Update a tag.
    ///
    /// Whether a tag takes a value cannot be changed once anything is tagged
    /// with it.
    pub fn update_tag(
        &self,
        id: &str,
        name: TranslatedString,
        takes_value: bool,
        private: bool,
        enable_updates: bool,
    ) -> Result<()> {
        self.commit(|connection| {
            let previous = tags::table
                .filter(tags::tag_id.eq(id))
                .select(tags::takes_value)
                .first::<bool>(connection)
                .map_err(anyhow::Error::from)?;

            if previous != takes_value && Self::tag_is_in_use_priv(connection, id)? {
                return Err(LibraryError::StillReferenced(EntityKind::Tag).into());
            }

            update_entity(connection, EntityKind::Tag, id, |connection, edited_at| {
                diesel::update(tags::table)
                    .filter(tags::tag_id.eq(id))
                    .set((
                        tags::name.eq(name),
                        tags::takes_value.eq(takes_value),
                        tags::private.eq(private),
                        tags::edited_at.eq(edited_at),
                        tags::last_used_at.eq(edited_at),
                        tags::enable_updates.eq(enable_updates),
                    ))
                    .execute(connection)
            })
        })
    }

    pub fn delete_tag(&self, tag_id: &str) -> Result<()> {
        self.commit(|connection| {
            delete_entity(connection, EntityKind::Tag, tag_id, |connection| {
                diesel::delete(tags::table)
                    .filter(tags::tag_id.eq(tag_id))
                    .execute(connection)
            })
        })
    }

    /// Assign a tag to several works at once.
    ///
    /// Returns how many works have been updated. Existing associations are
    /// skipped; a call that changes nothing updates no timestamps and does not
    /// notify observers.
    pub fn add_tag_to_works(
        &self,
        work_ids: &[&str],
        tag: &Tag,
        value: Option<&TranslatedString>,
    ) -> Result<usize> {
        let connection = &mut *self.conn();
        let value = value.filter(|_| tag.takes_value).cloned();
        let now = db::now();

        let changed = connection.transaction::<usize, LibraryError, _>(|connection| {
            let mut changed = 0usize;

            for &work_id in work_ids {
                let existing = work_tags::table
                    .filter(work_tags::work_id.eq(work_id))
                    .select((
                        work_tags::tag_id,
                        work_tags::value,
                        work_tags::sequence_number,
                    ))
                    .load::<(String, Option<TranslatedString>, i32)>(connection)?;

                let already_tagged = existing.iter().any(|(tag_id, existing_value, _)| {
                    tag_id == &tag.id && existing_value == &value
                });

                if already_tagged {
                    continue;
                }

                let next_sequence = existing
                    .iter()
                    .map(|(_, _, sequence_number)| *sequence_number)
                    .max()
                    .map_or(0, |n| n + 1);

                diesel::insert_into(work_tags::table)
                    .values(tables::WorkTag {
                        work_id: work_id.to_string(),
                        tag_id: tag.id.clone(),
                        value: value.clone(),
                        sequence_number: next_sequence,
                    })
                    .execute(connection)?;

                diesel::update(works::table)
                    .filter(works::work_id.eq(work_id))
                    .set((works::edited_at.eq(now), works::last_used_at.eq(now)))
                    .execute(connection)?;

                changed += 1;
            }

            Ok(changed)
        })?;

        if changed > 0 {
            self.changed();
        }

        Ok(changed)
    }

    /// Assign a tag to several recordings at once.
    ///
    /// Returns how many recordings have been updated. Existing associations
    /// are skipped; a call that changes nothing updates no timestamps and does
    /// not notify observers.
    pub fn add_tag_to_recordings(
        &self,
        recording_ids: &[&str],
        tag: &Tag,
        value: Option<&TranslatedString>,
    ) -> Result<usize> {
        let connection = &mut *self.conn();
        let value = value.filter(|_| tag.takes_value).cloned();
        let now = db::now();

        let changed = connection.transaction::<usize, LibraryError, _>(|connection| {
            let mut changed = 0usize;

            for &recording_id in recording_ids {
                let existing = recording_tags::table
                    .filter(recording_tags::recording_id.eq(recording_id))
                    .select((
                        recording_tags::tag_id,
                        recording_tags::value,
                        recording_tags::sequence_number,
                    ))
                    .load::<(String, Option<TranslatedString>, i32)>(connection)?;

                let already_tagged = existing.iter().any(|(tag_id, existing_value, _)| {
                    tag_id == &tag.id && existing_value == &value
                });

                if already_tagged {
                    continue;
                }

                let next_sequence = existing
                    .iter()
                    .map(|(_, _, sequence_number)| *sequence_number)
                    .max()
                    .map_or(0, |n| n + 1);

                diesel::insert_into(recording_tags::table)
                    .values(tables::RecordingTag {
                        recording_id: recording_id.to_string(),
                        tag_id: tag.id.clone(),
                        value: value.clone(),
                        sequence_number: next_sequence,
                    })
                    .execute(connection)?;

                diesel::update(recordings::table)
                    .filter(recordings::recording_id.eq(recording_id))
                    .set((
                        recordings::edited_at.eq(now),
                        recordings::last_used_at.eq(now),
                    ))
                    .execute(connection)?;

                changed += 1;
            }

            Ok(changed)
        })?;

        if changed > 0 {
            self.changed();
        }

        Ok(changed)
    }

    /// Remove a tag from several works at once.
    ///
    /// Returns how many works have been updated. A call that changes nothing
    /// updates no timestamps and does not notify observers.
    pub fn remove_tag_from_works(&self, work_ids: &[&str], tag_id: &str) -> Result<usize> {
        let connection = &mut *self.conn();
        let now = db::now();

        let changed = connection.transaction::<usize, LibraryError, _>(|connection| {
            let mut changed = 0usize;

            for &work_id in work_ids {
                let deleted = diesel::delete(
                    work_tags::table
                        .filter(work_tags::work_id.eq(work_id))
                        .filter(work_tags::tag_id.eq(tag_id)),
                )
                .execute(connection)?;

                if deleted == 0 {
                    continue;
                }

                diesel::update(works::table)
                    .filter(works::work_id.eq(work_id))
                    .set((works::edited_at.eq(now), works::last_used_at.eq(now)))
                    .execute(connection)?;

                changed += 1;
            }

            Ok(changed)
        })?;

        if changed > 0 {
            self.changed();
        }

        Ok(changed)
    }

    /// Remove a tag from several recordings at once.
    ///
    /// Returns how many recordings have been updated. A call that changes
    /// nothing updates no timestamps and does not notify observers.
    pub fn remove_tag_from_recordings(
        &self,
        recording_ids: &[&str],
        tag_id: &str,
    ) -> Result<usize> {
        let connection = &mut *self.conn();
        let now = db::now();

        let changed = connection.transaction::<usize, LibraryError, _>(|connection| {
            let mut changed = 0usize;

            for &recording_id in recording_ids {
                let deleted = diesel::delete(
                    recording_tags::table
                        .filter(recording_tags::recording_id.eq(recording_id))
                        .filter(recording_tags::tag_id.eq(tag_id)),
                )
                .execute(connection)?;

                if deleted == 0 {
                    continue;
                }

                diesel::update(recordings::table)
                    .filter(recordings::recording_id.eq(recording_id))
                    .set((
                        recordings::edited_at.eq(now),
                        recordings::last_used_at.eq(now),
                    ))
                    .execute(connection)?;

                changed += 1;
            }

            Ok(changed)
        })?;

        if changed > 0 {
            self.changed();
        }

        Ok(changed)
    }
}
