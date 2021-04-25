use super::generate_id;
use super::schema::{ensembles, mediums, performances, persons, recordings, tracks};
use super::{Database, Error, Recording, Result};
use diesel::prelude::*;
use log::info;
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

    /// The tracks of the medium.
    pub tracks: Vec<Track>,
}

/// A track on a medium.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    /// The recording on this track.
    pub recording: Recording,

    /// The work parts that are played on this track. They are indices to the
    /// work parts of the work that is associated with the recording.
    pub work_parts: Vec<usize>,

    /// The index of the track within its source. This is used to associate
    /// the metadata with the audio data from the source when importing.
    pub source_index: usize,

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

/// Table data for a [`Track`].
#[derive(Insertable, Queryable, Debug, Clone)]
#[table_name = "tracks"]
struct TrackRow {
    pub id: String,
    pub medium: String,
    pub index: i32,
    pub recording: String,
    pub work_parts: String,
    pub source_index: i32,
    pub path: String,
}

impl Database {
    /// Update an existing medium or insert a new one.
    pub fn update_medium(&self, medium: Medium) -> Result<()> {
        info!("Updating medium {:?}", medium);
        self.defer_foreign_keys()?;

        self.connection.transaction::<(), Error, _>(|| {
            let medium_id = &medium.id;

            // This will also delete the tracks.
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

            for (index, track) in medium.tracks.iter().enumerate() {
                // Add associated items from the server, if they don't already exist.

                if self.get_recording(&track.recording.id)?.is_none() {
                    self.update_recording(track.recording.clone())?;
                }

                // Add the actual track data.

                let work_parts = track
                    .work_parts
                    .iter()
                    .map(|part_index| part_index.to_string())
                    .collect::<Vec<String>>()
                    .join(",");

                let track_row = TrackRow {
                    id: generate_id(),
                    medium: medium_id.to_owned(),
                    index: index as i32,
                    recording: track.recording.id.clone(),
                    work_parts,
                    source_index: track.source_index as i32,
                    path: track.path.clone(),
                };

                diesel::insert_into(tracks::table)
                    .values(track_row)
                    .execute(&self.connection)?;
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

    /// Get mediums that have a specific source ID.
    pub fn get_mediums_by_source_id(&self, source_id: &str) -> Result<Vec<Medium>> {
        let mut mediums: Vec<Medium> = Vec::new();

        let rows = mediums::table
            .filter(mediums::discid.nullable().eq(source_id))
            .load::<MediumRow>(&self.connection)?;

        for row in rows {
            let medium = self.get_medium_data(row)?;
            mediums.push(medium);
        }

        Ok(mediums)
    }

    /// Get mediums on which this person is performing.
    pub fn get_mediums_for_person(&self, person_id: &str) -> Result<Vec<Medium>> {
        let mut mediums: Vec<Medium> = Vec::new();

        let rows = mediums::table
            .inner_join(tracks::table.on(tracks::medium.eq(mediums::id)))
            .inner_join(recordings::table.on(recordings::id.eq(tracks::recording)))
            .inner_join(performances::table.on(performances::recording.eq(recordings::id)))
            .inner_join(persons::table.on(persons::id.nullable().eq(performances::person)))
            .filter(persons::id.eq(person_id))
            .select(mediums::table::all_columns())
            .distinct()
            .load::<MediumRow>(&self.connection)?;

        for row in rows {
            let medium = self.get_medium_data(row)?;
            mediums.push(medium);
        }

        Ok(mediums)
    }

    /// Get mediums on which this ensemble is performing.
    pub fn get_mediums_for_ensemble(&self, ensemble_id: &str) -> Result<Vec<Medium>> {
        let mut mediums: Vec<Medium> = Vec::new();

        let rows = mediums::table
            .inner_join(tracks::table.on(tracks::medium.eq(tracks::id)))
            .inner_join(recordings::table.on(recordings::id.eq(tracks::recording)))
            .inner_join(performances::table.on(performances::recording.eq(recordings::id)))
            .inner_join(ensembles::table.on(ensembles::id.nullable().eq(performances::ensemble)))
            .filter(ensembles::id.eq(ensemble_id))
            .select(mediums::table::all_columns())
            .distinct()
            .load::<MediumRow>(&self.connection)?;

        for row in rows {
            let medium = self.get_medium_data(row)?;
            mediums.push(medium);
        }

        Ok(mediums)
    }

    /// Delete a medium and all of its tracks. This will fail, if the music
    /// library contains audio files referencing any of those tracks.
    pub fn delete_medium(&self, id: &str) -> Result<()> {
        info!("Deleting medium {}", id);
        diesel::delete(mediums::table.filter(mediums::id.eq(id))).execute(&self.connection)?;
        Ok(())
    }

    /// Get all available tracks for a recording.
    pub fn get_tracks(&self, recording_id: &str) -> Result<Vec<Track>> {
        let mut tracks: Vec<Track> = Vec::new();

        let rows = tracks::table
            .inner_join(recordings::table.on(recordings::id.eq(tracks::recording)))
            .filter(recordings::id.eq(recording_id))
            .select(tracks::table::all_columns())
            .load::<TrackRow>(&self.connection)?;

        for row in rows {
            let track = self.get_track_from_row(row)?;
            tracks.push(track);
        }

        Ok(tracks)
    }

    /// Retrieve all available information on a medium from related tables.
    fn get_medium_data(&self, row: MediumRow) -> Result<Medium> {
        let track_rows = tracks::table
            .filter(tracks::medium.eq(&row.id))
            .order_by(tracks::index)
            .load::<TrackRow>(&self.connection)?;

        let mut tracks = Vec::new();

        for track_row in track_rows {
            let track = self.get_track_from_row(track_row)?;
            tracks.push(track);
        }

        let medium = Medium {
            id: row.id,
            name: row.name,
            discid: row.discid,
            tracks,
        };

        Ok(medium)
    }

    /// Convert a track row from the database to an actual track.
    fn get_track_from_row(&self, row: TrackRow) -> Result<Track> {
        let recording_id = row.recording;

        let recording = self
            .get_recording(&recording_id)?
            .ok_or(Error::MissingItem("recording", recording_id))?;

        let mut part_indices = Vec::new();

        let work_parts = row.work_parts.split(',');

        for part_index in work_parts {
            if !part_index.is_empty() {
                let index = str::parse(part_index)
                    .map_err(|_| Error::ParsingError("part index", String::from(part_index)))?;

                part_indices.push(index);
            }
        }

        let track = Track {
            recording,
            work_parts: part_indices,
            source_index: row.source_index as usize,
            path: row.path,
        };

        Ok(track)
    }
}
