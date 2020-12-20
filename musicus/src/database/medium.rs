use super::generate_id;
use super::schema::{mediums, track_sets, tracks};
use super::{Database, Recording};
use anyhow::{anyhow, Error, Result};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

/// Representation of someting like a physical audio disc or a folder with
/// audio files (i.e. a collection of tracks for one or more recordings).
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Medium {
    pub id: String,
    pub name: String,
    pub discid: Option<String>,
    pub tracks: Vec<TrackSet>,
}

/// A set of tracks of one recording within a medium.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TrackSet {
    pub recording: Recording,
    pub tracks: Vec<Track>,
}

/// A track within a recording on a medium.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    work_parts: Vec<usize>,
}

/// Table data for a [`Medium`].
#[derive(Insertable, Queryable, Debug, Clone)]
#[table_name = "mediums"]
struct MediumRow {
    pub id: String,
    pub name: String,
    pub discid: Option<String>,
}

/// Table data for a [`TrackSet`].
#[derive(Insertable, Queryable, Debug, Clone)]
#[table_name = "track_sets"]
struct TrackSetRow {
    pub id: String,
    pub medium: String,
    pub index: i32,
    pub recording: String,
}

/// Table data for a [`Track`].
#[derive(Insertable, Queryable, Debug, Clone)]
#[table_name = "tracks"]
struct TrackRow {
    pub id: String,
    pub track_set: String,
    pub index: i32,
    pub work_parts: String,
}

impl Database {
    /// Update an existing medium or insert a new one.
    pub fn update_medium(&self, medium: Medium) -> Result<()> {
        self.defer_foreign_keys()?;

        self.connection.transaction::<(), Error, _>(|| {
            let medium_id = &medium.id;

            // This will also delete the track sets and tracks.
            self.delete_medium(medium_id)?;

            for (index, track_set) in medium.tracks.iter().enumerate() {
                let track_set_id = generate_id();

                let track_set_row = TrackSetRow {
                    id: track_set_id.clone(),
                    medium: medium_id.to_owned(),
                    index: index as i32,
                    recording: track_set.recording.id.clone(),
                };

                diesel::insert_into(track_sets::table)
                    .values(track_set_row)
                    .execute(&self.connection)?;

                for (index, track) in track_set.tracks.iter().enumerate() {
                    let work_parts = track
                        .work_parts
                        .iter()
                        .map(|part_index| part_index.to_string())
                        .collect::<Vec<String>>()
                        .join(",");

                    let track_row = TrackRow {
                        id: generate_id(),
                        track_set: track_set_id.clone(),
                        index: index as i32,
                        work_parts,
                    };

                    diesel::insert_into(tracks::table)
                        .values(track_row)
                        .execute(&self.connection)?;
                }
            }

            Ok(())
        })?;

        Ok(())
    }

    /// Get an existing medium.
    pub fn get_medium(&self, id: &str) -> Result<Option<Medium>> {
        let row = mediums::table
            .filter(mediums::id.eq(id))
            .load::<MediumRow>(&self.connection)?
            .into_iter()
            .next();

        let medium = match row {
            Some(row) => Some(self.get_medium_data(row)?),
            None => None,
        };

        Ok(medium)
    }

    /// Delete a medium and all of its tracks. This will fail, if the music
    /// library contains audio files referencing any of those tracks.
    pub fn delete_medium(&self, id: &str) -> Result<()> {
        diesel::delete(mediums::table.filter(mediums::id.eq(id))).execute(&self.connection)?;
        Ok(())
    }

    /// Retrieve all available information on a medium from related tables.
    fn get_medium_data(&self, row: MediumRow) -> Result<Medium> {
        let track_set_rows = track_sets::table
            .filter(track_sets::medium.eq(&row.id))
            .order_by(track_sets::index)
            .load::<TrackSetRow>(&self.connection)?;

        let mut track_sets = Vec::new();

        for track_set_row in track_set_rows {
            let recording_id = &track_set_row.recording;

            let recording = self
                .get_recording(recording_id)?
                .ok_or_else(|| anyhow!("No recording with ID: {}", recording_id))?;

            let track_rows = tracks::table
                .filter(tracks::id.eq(&track_set_row.id))
                .order_by(tracks::index)
                .load::<TrackRow>(&self.connection)?;

            let mut tracks = Vec::new();

            for track_row in track_rows {
                let work_parts = track_row
                    .work_parts
                    .split(',')
                    .map(|part_index| Ok(str::parse(part_index)?))
                    .collect::<Result<Vec<usize>>>()?;

                let track = Track { work_parts };

                tracks.push(track);
            }

            let track_set = TrackSet { recording, tracks };

            track_sets.push(track_set);
        }

        let medium = Medium {
            id: row.id,
            name: row.name,
            discid: row.discid,
            tracks: track_sets,
        };

        Ok(medium)
    }
}
