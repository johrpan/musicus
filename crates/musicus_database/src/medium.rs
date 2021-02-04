use super::generate_id;
use super::schema::{mediums, recordings, track_sets, tracks};
use super::{Database, Error, Recording, Result};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

/// Representation of someting like a physical audio disc or a folder with
/// audio files (i.e. a collection of tracks for one or more recordings).
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Medium {
    /// An unique ID for the medium.
    pub id: String,

    /// The human identifier for the medium.
    pub name: String,

    /// If applicable, the MusicBrainz DiscID.
    pub discid: Option<String>,

    /// The tracks of the medium, grouped by recording.
    pub tracks: Vec<TrackSet>,
}

/// A set of tracks of one recording within a medium.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TrackSet {
    /// The recording to which the tracks belong.
    pub recording: Recording,

    /// The actual tracks.
    pub tracks: Vec<Track>,
}

/// A track within a recording on a medium.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    /// The work parts that are played on this track. They are indices to the
    /// work parts of the work that is associated with the recording.
    pub work_parts: Vec<usize>,

    /// The path to the audio file containing this track. This will not be
    /// included when communicating with the server.
    #[serde(skip)]
    pub path: String,
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
    pub path: String,
}

impl Database {
    /// Update an existing medium or insert a new one.
    pub fn update_medium(&self, medium: Medium) -> Result<()> {
        self.defer_foreign_keys()?;

        self.connection.transaction::<(), Error, _>(|| {
            let medium_id = &medium.id;

            // This will also delete the track sets and tracks.
            self.delete_medium(medium_id)?;

            // Add the new medium.

            let medium_row = MediumRow {
                id: medium_id.to_owned(),
                name: medium.name.clone(),
                discid: medium.discid.clone(),
            };

            diesel::insert_into(mediums::table)
                .values(medium_row)
                .execute(&self.connection)?;

            for (index, track_set) in medium.tracks.iter().enumerate() {
                // Add associated items from the server, if they don't already
                // exist.

                if self.get_recording(&track_set.recording.id)?.is_none() {
                    self.update_recording(track_set.recording.clone())?;
                }

                // Add the actual track set data.

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
                        path: track.path.clone(),
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

    /// Get all available track sets for a recording.
    pub fn get_track_sets(&self, recording_id: &str) -> Result<Vec<TrackSet>> {
        let mut track_sets: Vec<TrackSet> = Vec::new();

        let rows = track_sets::table
            .inner_join(recordings::table.on(recordings::id.eq(track_sets::recording)))
            .filter(recordings::id.eq(recording_id))
            .select(track_sets::table::all_columns())
            .load::<TrackSetRow>(&self.connection)?;

        for row in rows {
            let track_set = self.get_track_set_from_row(row)?;
            track_sets.push(track_set);
        }

        Ok(track_sets)
    }

    /// Retrieve all available information on a medium from related tables.
    fn get_medium_data(&self, row: MediumRow) -> Result<Medium> {
        let track_set_rows = track_sets::table
            .filter(track_sets::medium.eq(&row.id))
            .order_by(track_sets::index)
            .load::<TrackSetRow>(&self.connection)?;

        let mut track_sets = Vec::new();

        for track_set_row in track_set_rows {
            let track_set = self.get_track_set_from_row(track_set_row)?;
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

    /// Convert a track set row from the database to an actual track set.
    fn get_track_set_from_row(&self, row: TrackSetRow) -> Result<TrackSet> {
        let recording_id = row.recording;

        let recording = self
            .get_recording(&recording_id)?
            .ok_or(Error::Other(format!(
                "Failed to get recording ({}) for track set ({}).",
                recording_id,
                row.id,
            )))?;

        let track_rows = tracks::table
            .filter(tracks::track_set.eq(row.id))
            .order_by(tracks::index)
            .load::<TrackRow>(&self.connection)?;

        let mut tracks = Vec::new();

        for track_row in track_rows {
            let work_parts = track_row
                .work_parts
                .split(',')
                .map(|part_index| {
                    str::parse(part_index)
                        .or(Err(Error::Other(
                            format!("Failed to parse part index from '{}'.", track_row.work_parts,
                        ))))
                })
                .collect::<Result<Vec<usize>>>()?;

            let track = Track {
                work_parts,
                path: track_row.path,
            };

            tracks.push(track);
        }

        let track_set = TrackSet { recording, tracks };

        Ok(track_set)
    }
}
