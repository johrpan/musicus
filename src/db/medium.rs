use chrono::{DateTime, TimeZone, Utc};
use diesel::prelude::*;
use log::info;

use crate::db::{
    defer_foreign_keys, generate_id, get_recording,
    schema::{ensembles, mediums, performances, persons, recordings, tracks},
    update_recording, Error, Recording, Result,
};

/// Representation of someting like a physical audio disc or a folder with
/// audio files (i.e. a collection of tracks for one or more recordings).
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct Medium {
    /// An unique ID for the medium.
    pub id: String,

    /// The human identifier for the medium.
    pub name: String,

    /// If applicable, the MusicBrainz DiscID.
    pub discid: Option<String>,

    /// The tracks of the medium.
    pub tracks: Vec<Track>,

    pub last_used: Option<DateTime<Utc>>,
    pub last_played: Option<DateTime<Utc>>,
}

impl Medium {
    pub fn new(id: String, name: String, discid: Option<String>, tracks: Vec<Track>) -> Self {
        Self {
            id,
            name,
            discid,
            tracks,
            last_used: Some(Utc::now()),
            last_played: None,
        }
    }
}

/// A track on a medium.
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct Track {
    /// The recording on this track.
    pub recording: Recording,

    /// The work parts that are played on this track. They are indices to the
    /// work parts of the work that is associated with the recording.
    pub work_parts: Vec<usize>,

    /// The index of the track within its source. This is used to associate
    /// the metadata with the audio data from the source when importing.
    pub source_index: usize,

    /// The path to the audio file containing this track.
    pub path: String,

    pub last_used: Option<DateTime<Utc>>,
    pub last_played: Option<DateTime<Utc>>,
}

impl Track {
    pub fn new(
        recording: Recording,
        work_parts: Vec<usize>,
        source_index: usize,
        path: String,
    ) -> Self {
        Self {
            recording,
            work_parts,
            source_index,
            path,
            last_used: Some(Utc::now()),
            last_played: None,
        }
    }
}

/// Table data for a [`Medium`].
#[derive(Insertable, Queryable, Debug, Clone)]
#[diesel(table_name = mediums)]
struct MediumRow {
    pub id: String,
    pub name: String,
    pub discid: Option<String>,
    pub last_used: Option<i64>,
    pub last_played: Option<i64>,
}

/// Table data for a [`Track`].
#[derive(Insertable, Queryable, QueryableByName, Debug, Clone)]
#[diesel(table_name = tracks)]
struct TrackRow {
    pub id: String,
    pub medium: Option<String>,
    pub index: i32,
    pub recording: String,
    pub work_parts: String,
    pub source_index: i32,
    pub path: String,
    pub last_used: Option<i64>,
    pub last_played: Option<i64>,
}

/// Update an existing medium or insert a new one.
pub fn update_medium(connection: &mut SqliteConnection, medium: Medium) -> Result<()> {
    info!("Updating medium {:?}", medium);
    defer_foreign_keys(connection)?;

    connection.transaction::<(), Error, _>(|connection| {
        let medium_id = &medium.id;

        // This will also delete the tracks.
        delete_medium(connection, medium_id)?;

        // Add the new medium.

        let medium_row = MediumRow {
            id: medium_id.to_owned(),
            name: medium.name.clone(),
            discid: medium.discid.clone(),
            last_used: Some(Utc::now().timestamp()),
            last_played: medium.last_played.map(|t| t.timestamp()),
        };

        diesel::insert_into(mediums::table)
            .values(medium_row)
            .execute(connection)?;

        for (index, track) in medium.tracks.iter().enumerate() {
            // Add associated items from the server, if they don't already exist.

            if get_recording(connection, &track.recording.id)?.is_none() {
                update_recording(connection, track.recording.clone())?;
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
                medium: Some(medium_id.to_owned()),
                index: index as i32,
                recording: track.recording.id.clone(),
                work_parts,
                source_index: track.source_index as i32,
                path: track.path.clone(),
                last_used: Some(Utc::now().timestamp()),
                last_played: track.last_played.map(|t| t.timestamp()),
            };

            diesel::insert_into(tracks::table)
                .values(track_row)
                .execute(connection)?;
        }

        Ok(())
    })?;

    Ok(())
}

/// Get an existing medium.
pub fn get_medium(connection: &mut SqliteConnection, id: &str) -> Result<Option<Medium>> {
    let row = mediums::table
        .filter(mediums::id.eq(id))
        .load::<MediumRow>(connection)?
        .into_iter()
        .next();

    let medium = match row {
        Some(row) => Some(get_medium_data(connection, row)?),
        None => None,
    };

    Ok(medium)
}

/// Get mediums that have a specific source ID.
pub fn get_mediums_by_source_id(
    connection: &mut SqliteConnection,
    source_id: &str,
) -> Result<Vec<Medium>> {
    let mut mediums: Vec<Medium> = Vec::new();

    let rows = mediums::table
        .filter(mediums::discid.nullable().eq(source_id))
        .load::<MediumRow>(connection)?;

    for row in rows {
        let medium = get_medium_data(connection, row)?;
        mediums.push(medium);
    }

    Ok(mediums)
}

/// Get mediums on which this person is performing.
pub fn get_mediums_for_person(
    connection: &mut SqliteConnection,
    person_id: &str,
) -> Result<Vec<Medium>> {
    let mut mediums: Vec<Medium> = Vec::new();

    let rows = mediums::table
        .inner_join(tracks::table.on(tracks::medium.eq(mediums::id.nullable())))
        .inner_join(recordings::table.on(recordings::id.eq(tracks::recording)))
        .inner_join(performances::table.on(performances::recording.eq(recordings::id)))
        .inner_join(persons::table.on(persons::id.nullable().eq(performances::person)))
        .filter(persons::id.eq(person_id))
        .select(mediums::table::all_columns())
        .distinct()
        .load::<MediumRow>(connection)?;

    for row in rows {
        let medium = get_medium_data(connection, row)?;
        mediums.push(medium);
    }

    Ok(mediums)
}

/// Get mediums on which this ensemble is performing.
pub fn get_mediums_for_ensemble(
    connection: &mut SqliteConnection,
    ensemble_id: &str,
) -> Result<Vec<Medium>> {
    let mut mediums: Vec<Medium> = Vec::new();

    let rows = mediums::table
        .inner_join(tracks::table.on(tracks::medium.eq(tracks::id.nullable())))
        .inner_join(recordings::table.on(recordings::id.eq(tracks::recording)))
        .inner_join(performances::table.on(performances::recording.eq(recordings::id)))
        .inner_join(ensembles::table.on(ensembles::id.nullable().eq(performances::ensemble)))
        .filter(ensembles::id.eq(ensemble_id))
        .select(mediums::table::all_columns())
        .distinct()
        .load::<MediumRow>(connection)?;

    for row in rows {
        let medium = get_medium_data(connection, row)?;
        mediums.push(medium);
    }

    Ok(mediums)
}

/// Delete a medium and all of its tracks. This will fail, if the music
/// library contains audio files referencing any of those tracks.
pub fn delete_medium(connection: &mut SqliteConnection, id: &str) -> Result<()> {
    info!("Deleting medium {}", id);
    diesel::delete(mediums::table.filter(mediums::id.eq(id))).execute(connection)?;
    Ok(())
}

/// Get all available tracks for a recording.
pub fn get_tracks(connection: &mut SqliteConnection, recording_id: &str) -> Result<Vec<Track>> {
    let mut tracks: Vec<Track> = Vec::new();

    let rows = tracks::table
        .inner_join(recordings::table.on(recordings::id.eq(tracks::recording)))
        .filter(recordings::id.eq(recording_id))
        .select(tracks::table::all_columns())
        .load::<TrackRow>(connection)?;

    for row in rows {
        let track = get_track_from_row(connection, row)?;
        tracks.push(track);
    }

    Ok(tracks)
}

/// Get a random track from the database.
pub fn random_track(connection: &mut SqliteConnection) -> Result<Track> {
    let row = diesel::sql_query("SELECT * FROM tracks ORDER BY RANDOM() LIMIT 1")
        .load::<TrackRow>(connection)?
        .into_iter()
        .next()
        .ok_or(Error::Other("Failed to generate random track"))?;

    get_track_from_row(connection, row)
}

/// Retrieve all available information on a medium from related tables.
fn get_medium_data(connection: &mut SqliteConnection, row: MediumRow) -> Result<Medium> {
    let track_rows = tracks::table
        .filter(tracks::medium.eq(&row.id))
        .order_by(tracks::index)
        .load::<TrackRow>(connection)?;

    let mut tracks = Vec::new();

    for track_row in track_rows {
        let track = get_track_from_row(connection, track_row)?;
        tracks.push(track);
    }

    let medium = Medium {
        id: row.id,
        name: row.name,
        discid: row.discid,
        tracks,
        last_used: row.last_used.map(|t| Utc.timestamp_opt(t, 0).unwrap()),
        last_played: row.last_played.map(|t| Utc.timestamp_opt(t, 0).unwrap()),
    };

    Ok(medium)
}

/// Convert a track row from the database to an actual track.
fn get_track_from_row(connection: &mut SqliteConnection, row: TrackRow) -> Result<Track> {
    let recording_id = row.recording;

    let recording = get_recording(connection, &recording_id)?
        .ok_or(Error::MissingItem("recording", recording_id))?;

    let mut part_indices = Vec::new();

    let work_parts = row.work_parts.split(',');

    for part_index in work_parts {
        if !part_index.is_empty() {
            let index = str::parse(part_index)
                .map_err(|_| Error::Parsing("part index", String::from(part_index)))?;

            part_indices.push(index);
        }
    }

    let track = Track {
        recording,
        work_parts: part_indices,
        source_index: row.source_index as usize,
        path: row.path,
        last_used: row.last_used.map(|t| Utc.timestamp_opt(t, 0).unwrap()),
        last_played: row.last_played.map(|t| Utc.timestamp_opt(t, 0).unwrap()),
    };

    Ok(track)
}
