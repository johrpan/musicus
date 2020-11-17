use super::schema::tracks;
use super::Database;
use anyhow::{Error, Result};
use diesel::prelude::*;
use std::convert::{TryFrom, TryInto};

/// Table row data for a track.
#[derive(Insertable, Queryable, Debug, Clone)]
#[table_name = "tracks"]
struct TrackRow {
    pub id: i64,
    pub file_name: String,
    pub recording: i64,
    pub track_index: i32,
    pub work_parts: String,
}

/// A structure representing one playable audio file.
#[derive(Debug, Clone)]
pub struct Track {
    pub work_parts: Vec<usize>,
    pub file_name: String,
}

impl TryFrom<TrackRow> for Track {
    type Error = Error;
    fn try_from(row: TrackRow) -> Result<Self> {
        let mut work_parts = Vec::<usize>::new();
        for part in row.work_parts.split(",") {
            if !part.is_empty() {
                work_parts.push(part.parse()?);
            }
        }

        let track = Track {
            work_parts,
            file_name: row.file_name,
        };

        Ok(track)
    }
}

impl Database {
    /// Insert or update tracks for the specified recording.
    pub fn update_tracks(&self, recording_id: u32, tracks: Vec<Track>) -> Result<()> {
        self.delete_tracks(recording_id)?;

        for (index, track) in tracks.iter().enumerate() {
            let row = TrackRow {
                id: rand::random(),
                file_name: track.file_name.clone(),
                recording: recording_id as i64,
                track_index: index.try_into()?,
                work_parts: track
                    .work_parts
                    .iter()
                    .map(|i| i.to_string())
                    .collect::<Vec<String>>()
                    .join(","),
            };

            diesel::insert_into(tracks::table)
                .values(row)
                .execute(&self.connection)?;
        }

        Ok(())
    }

    /// Delete all tracks for the specified recording.
    pub fn delete_tracks(&self, recording_id: u32) -> Result<()> {
        diesel::delete(tracks::table.filter(tracks::recording.eq(recording_id as i64)))
            .execute(&self.connection)?;

        Ok(())
    }

    /// Get all tracks of the specified recording.
    pub fn get_tracks(&self, recording_id: u32) -> Result<Vec<Track>> {
        let mut tracks = Vec::<Track>::new();

        let rows = tracks::table
            .filter(tracks::recording.eq(recording_id as i64))
            .order_by(tracks::track_index)
            .load::<TrackRow>(&self.connection)?;

        for row in rows {
            tracks.push(row.try_into()?);
        }

        Ok(tracks)
    }
}
