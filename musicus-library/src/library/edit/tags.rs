//! Editing tags: creating, updating, deleting and (bulk) assigning tags to
//! works and recordings.

use anyhow::{Error, Result};
use diesel::{dsl::exists, prelude::*, QueryDsl, SqliteConnection};

use crate::db::{
    self,
    models::*,
    schema::*,
    tables::{self, Source},
    TranslatedString,
};
use crate::error::{EntityKind, LibraryError};
use crate::library::Library;

impl Library {
    pub fn create_tag(
        &self,
        name: TranslatedString,
        takes_value: bool,
        private: bool,
        enable_updates: bool,
    ) -> Result<Tag> {
        let connection = &mut *self.conn();

        let now = db::now();

        let tag = Tag {
            tag_id: db::generate_id(),
            name,
            takes_value,
            source: Source::User,
            created_at: now,
            edited_at: now,
            last_used_at: now,
            enable_updates,
            private,
        };

        diesel::insert_into(tags::table)
            .values(&tag)
            .execute(connection)?;

        self.changed();

        Ok(tag)
    }

    /// Whether any work or recording is tagged with this tag.
    pub fn tag_is_in_use(&self, tag_id: &str) -> Result<bool> {
        let connection = &mut *self.conn();
        Self::tag_is_in_use_priv(connection, tag_id)
    }

    /// The body of [`Library::tag_is_in_use`], for callers that already hold the
    /// connection. Taking it twice would deadlock: the guard lives until the end
    /// of the enclosing function, and dropping the `&mut` to it does not release
    /// it.
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
    /// with it. It decides what the existing assignments mean and how they are
    /// found: dropping the value would discard every value already recorded,
    /// and adding one would leave every existing assignment without the value
    /// that a valued tag is searched by. A tag nothing uses yet can still be
    /// corrected.
    pub fn update_tag(
        &self,
        id: &str,
        name: TranslatedString,
        takes_value: bool,
        private: bool,
        enable_updates: bool,
    ) -> Result<(), LibraryError> {
        let connection = &mut *self.conn();

        let previous = tags::table
            .filter(tags::tag_id.eq(id))
            .select(tags::takes_value)
            .first::<bool>(connection)
            .map_err(anyhow::Error::from)?;

        if previous != takes_value && Self::tag_is_in_use_priv(connection, id)? {
            return Err(LibraryError::StillReferenced(EntityKind::Tag));
        }

        let now = db::now();

        diesel::update(tags::table)
            .filter(tags::tag_id.eq(id))
            .set((
                tags::name.eq(name),
                tags::takes_value.eq(takes_value),
                tags::private.eq(private),
                tags::edited_at.eq(now),
                tags::last_used_at.eq(now),
                tags::enable_updates.eq(enable_updates),
            ))
            .execute(connection)
            .map_err(anyhow::Error::from)?;

        self.changed();

        Ok(())
    }

    pub fn delete_tag(&self, tag_id: &str) -> Result<(), LibraryError> {
        let connection = &mut *self.conn();

        diesel::delete(tags::table)
            .filter(tags::tag_id.eq(tag_id))
            .execute(connection)
            .map_err(|err| LibraryError::from_delete(EntityKind::Tag, err))?;

        self.changed();

        Ok(())
    }

    /// Replace a work's tag assignments.
    ///
    /// Assignments are keyed by `(work_id, sequence_number)`, so they are
    /// rewritten wholesale rather than diffed, matching how the other ordered
    /// relations of a work are updated.
    pub(super) fn set_work_tags(
        connection: &mut SqliteConnection,
        work_id: &str,
        tags: Vec<TagValue>,
    ) -> Result<()> {
        diesel::delete(work_tags::table)
            .filter(work_tags::work_id.eq(work_id))
            .execute(connection)?;

        for (index, tag_value) in tags.into_iter().enumerate() {
            let work_tag_data = tables::WorkTag {
                work_id: work_id.to_string(),
                tag_id: tag_value.tag.tag_id,
                value: tag_value.value.filter(|_| tag_value.tag.takes_value),
                sequence_number: index as i32,
            };

            diesel::insert_into(work_tags::table)
                .values(&work_tag_data)
                .execute(connection)?;
        }

        Ok(())
    }

    /// Replace a recording's tag assignments. See [`Library::set_work_tags`].
    pub(super) fn set_recording_tags(
        connection: &mut SqliteConnection,
        recording_id: &str,
        tags: Vec<TagValue>,
    ) -> Result<()> {
        diesel::delete(recording_tags::table)
            .filter(recording_tags::recording_id.eq(recording_id))
            .execute(connection)?;

        for (index, tag_value) in tags.into_iter().enumerate() {
            let recording_tag_data = tables::RecordingTag {
                recording_id: recording_id.to_string(),
                tag_id: tag_value.tag.tag_id,
                value: tag_value.value.filter(|_| tag_value.tag.takes_value),
                sequence_number: index as i32,
            };

            diesel::insert_into(recording_tags::table)
                .values(&recording_tag_data)
                .execute(connection)?;
        }

        Ok(())
    }

    /// Assign a tag to several works at once.
    ///
    /// Returns how many works have been updated. Existing associations are
    /// skipped.
    pub fn add_tag_to_works(
        &self,
        work_ids: &[&str],
        tag: &Tag,
        value: Option<&str>,
    ) -> Result<usize> {
        let connection = &mut *self.conn();
        let value = value.filter(|_| tag.takes_value).map(str::to_owned);
        let now = db::now();

        let changed = connection.transaction::<usize, Error, _>(|connection| {
            let mut changed = 0usize;

            for &work_id in work_ids {
                let existing = work_tags::table
                    .filter(work_tags::work_id.eq(work_id))
                    .select((
                        work_tags::tag_id,
                        work_tags::value,
                        work_tags::sequence_number,
                    ))
                    .load::<(String, Option<String>, i32)>(connection)?;

                let already_tagged = existing.iter().any(|(tag_id, existing_value, _)| {
                    tag_id == &tag.tag_id && existing_value == &value
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
                        tag_id: tag.tag_id.clone(),
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

        self.changed();

        Ok(changed)
    }

    /// Assign a tag to several recordings at once.
    ///
    /// Returns how many recordings have been updated. Existing associations
    /// are skipped.
    pub fn add_tag_to_recordings(
        &self,
        recording_ids: &[&str],
        tag: &Tag,
        value: Option<&str>,
    ) -> Result<usize> {
        let connection = &mut *self.conn();
        let value = value.filter(|_| tag.takes_value).map(str::to_owned);
        let now = db::now();

        let changed = connection.transaction::<usize, Error, _>(|connection| {
            let mut changed = 0usize;

            for &recording_id in recording_ids {
                let existing = recording_tags::table
                    .filter(recording_tags::recording_id.eq(recording_id))
                    .select((
                        recording_tags::tag_id,
                        recording_tags::value,
                        recording_tags::sequence_number,
                    ))
                    .load::<(String, Option<String>, i32)>(connection)?;

                let already_tagged = existing.iter().any(|(tag_id, existing_value, _)| {
                    tag_id == &tag.tag_id && existing_value == &value
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
                        tag_id: tag.tag_id.clone(),
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

        self.changed();

        Ok(changed)
    }

    /// Remove a tag from several works at once.
    ///
    /// Returns how many works have been updated.
    pub fn remove_tag_from_works(&self, work_ids: &[&str], tag_id: &str) -> Result<usize> {
        let connection = &mut *self.conn();
        let now = db::now();

        let changed = connection.transaction::<usize, Error, _>(|connection| {
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

        self.changed();

        Ok(changed)
    }

    /// Remove a tag from several recordings at once.
    ///
    /// Returns how many recordings have been updated.
    pub fn remove_tag_from_recordings(
        &self,
        recording_ids: &[&str],
        tag_id: &str,
    ) -> Result<usize> {
        let connection = &mut *self.conn();
        let now = db::now();

        let changed = connection.transaction::<usize, Error, _>(|connection| {
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

        self.changed();

        Ok(changed)
    }
}
