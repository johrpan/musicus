use std::{
    fs::{self, File},
    io::{BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
};

use adw::subclass::prelude::*;
use anyhow::{anyhow, Result};
use chrono::prelude::*;
use diesel::{prelude::*, SqliteConnection};
use formatx::formatx;
use futures_util::StreamExt;
use gettextrs::gettext;
use tempfile::NamedTempFile;
use tokio::io::AsyncWriteExt;
use zip::{write::SimpleFileOptions, ZipWriter};

use super::Library;
use crate::{
    db::{self, schema::*, tables},
    process::ProcessMsg,
};

impl Library {
    /// Import from a music library ZIP archive at `path`.
    pub fn import_library_from_zip(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<async_channel::Receiver<ProcessMsg>> {
        log::info!(
            "Importing library from ZIP at {}",
            path.as_ref().to_string_lossy()
        );
        let path = path.as_ref().to_owned();
        let library_folder = PathBuf::from(&self.folder());
        let this_connection = self.imp().connection.get().unwrap().clone();

        let (sender, receiver) = async_channel::unbounded::<ProcessMsg>();
        thread::spawn(move || {
            if let Err(err) = sender.send_blocking(ProcessMsg::Result(
                import_library_from_zip_priv(path, library_folder, this_connection, &sender),
            )) {
                log::error!("Failed to send library action result: {err:?}");
            }
        });

        Ok(receiver)
    }

    /// Export the whole music library to a ZIP archive at `path`. If `path` already exists, it
    /// will be overwritten. The work will be done in a background thread.
    pub fn export_library_to_zip(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<async_channel::Receiver<ProcessMsg>> {
        log::info!(
            "Exporting library to ZIP at {}",
            path.as_ref().to_string_lossy()
        );
        let connection = &mut *self.imp().connection.get().unwrap().lock().unwrap();

        let path = path.as_ref().to_owned();
        let library_folder = PathBuf::from(&self.folder());
        let tracks = tracks::table.load::<tables::Track>(connection)?;

        let (sender, receiver) = async_channel::unbounded::<ProcessMsg>();
        thread::spawn(move || {
            if let Err(err) = sender.send_blocking(ProcessMsg::Result(export_library_to_zip_priv(
                path,
                library_folder,
                tracks,
                &sender,
            ))) {
                log::error!("Failed to send library action result: {err:?}");
            }
        });

        Ok(receiver)
    }

    /// Import from a library archive at `url`.
    pub fn import_library_from_url(
        &self,
        url: &str,
    ) -> Result<async_channel::Receiver<ProcessMsg>> {
        log::info!("Importing library from URL {url}");
        let url = url.to_owned();
        let library_folder = PathBuf::from(&self.folder());
        let this_connection = self.imp().connection.get().unwrap().clone();

        let (sender, receiver) = async_channel::unbounded::<ProcessMsg>();

        thread::spawn(move || {
            if let Err(err) = sender.send_blocking(ProcessMsg::Result(
                import_library_from_url_priv(url, library_folder, this_connection, &sender),
            )) {
                log::error!("Failed to send library action result: {err:?}");
            }
        });

        Ok(receiver)
    }

    /// Import from metadata from a database file at `url`.
    pub fn import_metadata_from_url(
        &self,
        url: &str,
    ) -> Result<async_channel::Receiver<ProcessMsg>> {
        log::info!("Importing metadata from URL {url}");

        let url = url.to_owned();
        let this_connection = self.imp().connection.get().unwrap().clone();

        let (sender, receiver) = async_channel::unbounded::<ProcessMsg>();

        thread::spawn(move || {
            if let Err(err) = sender.send_blocking(ProcessMsg::Result(
                import_metadata_from_url_priv(url, this_connection, &sender),
            )) {
                log::error!("Failed to send library action result: {err:?}");
            }
        });

        Ok(receiver)
    }
}

// TODO: Add options whether to keep stats.
fn import_library_from_zip_priv(
    zip_path: impl AsRef<Path>,
    library_folder: impl AsRef<Path>,
    this_connection: Arc<Mutex<SqliteConnection>>,
    sender: &async_channel::Sender<ProcessMsg>,
) -> Result<()> {
    let mut archive = zip::ZipArchive::new(BufReader::new(fs::File::open(zip_path)?))?;

    let archive_db_file = archive.by_name("musicus.db")?;
    let tmp_db_file = NamedTempFile::new()?;
    std::io::copy(
        &mut BufReader::new(archive_db_file),
        &mut BufWriter::new(tmp_db_file.as_file()),
    )?;

    // Import metadata.
    let tracks = import_metadata_from_file(tmp_db_file.path(), this_connection, false)?;

    // Import audio files.
    let n_tracks = tracks.len();
    for (index, track) in tracks.into_iter().enumerate() {
        let library_track_file_path = library_folder.as_ref().join(&track.path);

        // Skip tracks that are already present.
        if !fs::exists(&library_track_file_path)? {
            if let Some(parent) = library_track_file_path.parent() {
                fs::create_dir_all(parent)?;
            }

            let archive_track_file = archive.by_name(&path_to_zip(&track.path)?)?;
            let library_track_file = File::create(library_track_file_path)?;

            std::io::copy(
                &mut BufReader::new(archive_track_file),
                &mut BufWriter::new(library_track_file),
            )?;
        }

        // Ignore if the reveiver has been dropped.
        let _ = sender.send_blocking(ProcessMsg::Progress((index + 1) as f64 / n_tracks as f64));
    }

    Ok(())
}

fn export_library_to_zip_priv(
    zip_path: impl AsRef<Path>,
    library_folder: impl AsRef<Path>,
    tracks: Vec<tables::Track>,
    sender: &async_channel::Sender<ProcessMsg>,
) -> Result<()> {
    let mut zip = zip::ZipWriter::new(BufWriter::new(fs::File::create(zip_path)?));

    // Start with the database:
    add_file_to_zip(&mut zip, &library_folder, "musicus.db")?;

    let n_tracks = tracks.len();

    // Include all tracks that are part of the library.
    for (index, track) in tracks.into_iter().enumerate() {
        add_file_to_zip(&mut zip, &library_folder, &path_to_zip(&track.path)?)?;

        // Ignore if the reveiver has been dropped.
        let _ = sender.send_blocking(ProcessMsg::Progress((index + 1) as f64 / n_tracks as f64));
    }

    zip.finish()?;

    Ok(())
}

fn add_file_to_zip(
    zip: &mut ZipWriter<BufWriter<File>>,
    library_folder: impl AsRef<Path>,
    library_path: &str,
) -> Result<()> {
    let file_path = library_folder.as_ref().join(PathBuf::from(library_path));

    let mut file = File::open(file_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    zip.start_file(library_path, SimpleFileOptions::default())?;
    zip.write_all(&buffer)?;

    Ok(())
}

fn import_metadata_from_url_priv(
    url: String,
    this_connection: Arc<Mutex<SqliteConnection>>,
    sender: &async_channel::Sender<ProcessMsg>,
) -> Result<()> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    let _ = sender.send_blocking(ProcessMsg::Message(
        formatx!(gettext("Downloading {}"), &url).unwrap(),
    ));

    match runtime.block_on(download_tmp_file(&url, sender)) {
        Ok(db_file) => {
            let _ = sender.send_blocking(ProcessMsg::Message(
                formatx!(gettext("Importing downloaded library"), &url).unwrap(),
            ));

            let _ = sender.send_blocking(ProcessMsg::Result(
                import_metadata_from_file(db_file.path(), this_connection, true).map(|tracks| {
                    if !tracks.is_empty() {
                        log::warn!("The metadata file at {url} contains tracks.");
                    }
                }),
            ));
        }
        Err(err) => {
            let _ = sender.send_blocking(ProcessMsg::Result(Err(err)));
        }
    }

    Ok(())
}

fn import_library_from_url_priv(
    url: String,
    library_folder: impl AsRef<Path>,
    this_connection: Arc<Mutex<SqliteConnection>>,
    sender: &async_channel::Sender<ProcessMsg>,
) -> Result<()> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    let _ = sender.send_blocking(ProcessMsg::Message(
        formatx!(gettext("Downloading {}"), &url).unwrap(),
    ));

    let archive_file = runtime.block_on(download_tmp_file(&url, sender));

    match archive_file {
        Ok(archive_file) => {
            let _ = sender.send_blocking(ProcessMsg::Message(
                formatx!(gettext("Importing downloaded library"), &url).unwrap(),
            ));

            let _ = sender.send_blocking(ProcessMsg::Result(import_library_from_zip_priv(
                archive_file.path(),
                library_folder,
                this_connection,
                sender,
            )));
        }
        Err(err) => {
            let _ = sender.send_blocking(ProcessMsg::Result(Err(err)));
        }
    }

    Ok(())
}

/// Import metadata from the database file at `path`.
///
/// If `ignore_tracks` is `true`, tracks and associated items like mediums will not be imported
/// from the database. In that case, if the database contains tracks, a warning will be logged.
/// In any case, tracks are returned.
fn import_metadata_from_file(
    path: impl AsRef<Path>,
    this_connection: Arc<Mutex<SqliteConnection>>,
    ignore_tracks: bool,
) -> Result<Vec<tables::Track>> {
    let now = Local::now().naive_local();

    let mut other_connection = db::connect(path.as_ref().to_str().unwrap())?;

    // Load all metadata from the archive.
    let persons = persons::table.load::<tables::Person>(&mut other_connection)?;
    let roles = roles::table.load::<tables::Role>(&mut other_connection)?;
    let instruments = instruments::table.load::<tables::Instrument>(&mut other_connection)?;
    let works = works::table.load::<tables::Work>(&mut other_connection)?;
    let work_persons = work_persons::table.load::<tables::WorkPerson>(&mut other_connection)?;
    let work_instruments =
        work_instruments::table.load::<tables::WorkInstrument>(&mut other_connection)?;
    let ensembles = ensembles::table.load::<tables::Ensemble>(&mut other_connection)?;
    let ensemble_persons =
        ensemble_persons::table.load::<tables::EnsemblePerson>(&mut other_connection)?;
    let recordings = recordings::table.load::<tables::Recording>(&mut other_connection)?;
    let recording_persons =
        recording_persons::table.load::<tables::RecordingPerson>(&mut other_connection)?;
    let recording_ensembles =
        recording_ensembles::table.load::<tables::RecordingEnsemble>(&mut other_connection)?;
    let tracks = tracks::table.load::<tables::Track>(&mut other_connection)?;
    let track_works = track_works::table.load::<tables::TrackWork>(&mut other_connection)?;
    let mediums = mediums::table.load::<tables::Medium>(&mut other_connection)?;
    let albums = albums::table.load::<tables::Album>(&mut other_connection)?;
    let album_recordings =
        album_recordings::table.load::<tables::AlbumRecording>(&mut other_connection)?;
    let album_mediums = album_mediums::table.load::<tables::AlbumMedium>(&mut other_connection)?;

    // Import metadata that is not already present.

    for mut person in persons {
        person.created_at = now;
        person.edited_at = now;
        person.last_used_at = now;
        person.last_played_at = None;

        diesel::insert_into(persons::table)
            .values(person)
            .on_conflict_do_nothing()
            .execute(&mut *this_connection.lock().unwrap())?;
    }

    for mut role in roles {
        role.created_at = now;
        role.edited_at = now;
        role.last_used_at = now;

        diesel::insert_into(roles::table)
            .values(role)
            .on_conflict_do_nothing()
            .execute(&mut *this_connection.lock().unwrap())?;
    }

    for mut instrument in instruments {
        instrument.created_at = now;
        instrument.edited_at = now;
        instrument.last_used_at = now;
        instrument.last_played_at = None;

        diesel::insert_into(instruments::table)
            .values(instrument)
            .on_conflict_do_nothing()
            .execute(&mut *this_connection.lock().unwrap())?;
    }

    for mut work in works {
        work.created_at = now;
        work.edited_at = now;
        work.last_used_at = now;
        work.last_played_at = None;

        diesel::insert_into(works::table)
            .values(work)
            .on_conflict_do_nothing()
            .execute(&mut *this_connection.lock().unwrap())?;
    }

    for work_person in work_persons {
        diesel::insert_into(work_persons::table)
            .values(work_person)
            .on_conflict_do_nothing()
            .execute(&mut *this_connection.lock().unwrap())?;
    }

    for work_instrument in work_instruments {
        diesel::insert_into(work_instruments::table)
            .values(work_instrument)
            .on_conflict_do_nothing()
            .execute(&mut *this_connection.lock().unwrap())?;
    }

    for mut ensemble in ensembles {
        ensemble.created_at = now;
        ensemble.edited_at = now;
        ensemble.last_used_at = now;
        ensemble.last_played_at = None;

        diesel::insert_into(ensembles::table)
            .values(ensemble)
            .on_conflict_do_nothing()
            .execute(&mut *this_connection.lock().unwrap())?;
    }

    for ensemble_person in ensemble_persons {
        diesel::insert_into(ensemble_persons::table)
            .values(ensemble_person)
            .on_conflict_do_nothing()
            .execute(&mut *this_connection.lock().unwrap())?;
    }

    for mut recording in recordings {
        recording.created_at = now;
        recording.edited_at = now;
        recording.last_used_at = now;
        recording.last_played_at = None;

        diesel::insert_into(recordings::table)
            .values(recording)
            .on_conflict_do_nothing()
            .execute(&mut *this_connection.lock().unwrap())?;
    }

    for recording_person in recording_persons {
        diesel::insert_into(recording_persons::table)
            .values(recording_person)
            .on_conflict_do_nothing()
            .execute(&mut *this_connection.lock().unwrap())?;
    }

    for recording_ensemble in recording_ensembles {
        diesel::insert_into(recording_ensembles::table)
            .values(recording_ensemble)
            .on_conflict_do_nothing()
            .execute(&mut *this_connection.lock().unwrap())?;
    }

    if !ignore_tracks {
        for mut track in tracks.clone() {
            track.created_at = now;
            track.edited_at = now;
            track.last_used_at = now;
            track.last_played_at = None;

            diesel::insert_into(tracks::table)
                .values(track)
                .on_conflict_do_nothing()
                .execute(&mut *this_connection.lock().unwrap())?;
        }

        for track_work in track_works {
            diesel::insert_into(track_works::table)
                .values(track_work)
                .on_conflict_do_nothing()
                .execute(&mut *this_connection.lock().unwrap())?;
        }

        for mut medium in mediums {
            medium.created_at = now;
            medium.edited_at = now;
            medium.last_used_at = now;
            medium.last_played_at = None;

            diesel::insert_into(mediums::table)
                .values(medium)
                .on_conflict_do_nothing()
                .execute(&mut *this_connection.lock().unwrap())?;
        }
    }

    for mut album in albums {
        album.created_at = now;
        album.edited_at = now;
        album.last_used_at = now;
        album.last_played_at = None;

        diesel::insert_into(albums::table)
            .values(album)
            .on_conflict_do_nothing()
            .execute(&mut *this_connection.lock().unwrap())?;
    }

    for album_recording in album_recordings {
        diesel::insert_into(album_recordings::table)
            .values(album_recording)
            .on_conflict_do_nothing()
            .execute(&mut *this_connection.lock().unwrap())?;
    }

    for album_medium in album_mediums {
        diesel::insert_into(album_mediums::table)
            .values(album_medium)
            .on_conflict_do_nothing()
            .execute(&mut *this_connection.lock().unwrap())?;
    }

    Ok(tracks)
}

async fn download_tmp_file(
    url: &str,
    sender: &async_channel::Sender<ProcessMsg>,
) -> Result<NamedTempFile> {
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()?;

    let response = client.get(url).send().await?;
    response.error_for_status_ref()?;

    let total_size = response.content_length();
    let mut body_stream = response.bytes_stream();

    let file = NamedTempFile::new()?;
    let mut writer =
        tokio::io::BufWriter::new(tokio::fs::File::from_std(file.as_file().try_clone()?));

    let mut downloaded = 0;
    while let Some(chunk) = body_stream.next().await {
        let chunk: Vec<u8> = chunk?.into();
        let chunk_size = chunk.len();

        writer.write_all(&chunk).await?;

        if let Some(total_size) = total_size {
            downloaded += chunk_size as u64;
            let _ = sender
                .send(ProcessMsg::Progress(downloaded as f64 / total_size as f64))
                .await;
        }
    }

    writer.flush().await?;

    Ok(file)
}

/// Convert a path to a ZIP path. ZIP files use "/" as the path separator
/// regardless of the current platform.
fn path_to_zip(path: impl AsRef<Path>) -> Result<String> {
    Ok(path
        .as_ref()
        .iter()
        .map(|p| {
            p.to_str()
                .ok_or_else(|| {
                    anyhow!(
                        "Path \"{}\"contains invalid UTF-8",
                        path.as_ref().to_string_lossy()
                    )
                })
                .map(|s| s.to_owned())
        })
        .collect::<Result<Vec<String>>>()?
        .join("/"))
}
