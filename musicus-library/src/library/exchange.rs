use std::{
    fs::{self, File},
    io::{BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
};

use anyhow::{anyhow, Error, Result};
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
    db::{
        self,
        schema::*,
        tables::{self, Source},
    },
    process::{Cancellation, ProcessHandle, ProcessMsg},
};

/// Run `operation` on a background thread, reporting its outcome as the final
/// message on the returned handle's channel.
fn spawn_process(
    operation: impl FnOnce(&async_channel::Sender<ProcessMsg>, &Cancellation) -> Result<()>
        + Send
        + 'static,
) -> ProcessHandle {
    let (sender, receiver) = async_channel::unbounded::<ProcessMsg>();
    let cancellation = Cancellation::new();

    let thread_cancellation = cancellation.clone();
    thread::spawn(move || {
        let result = operation(&sender, &thread_cancellation);

        // A cancelled operation fails with a sentinel error that must not be
        // reported to the user as a failure.
        let msg = if thread_cancellation.is_cancelled() {
            ProcessMsg::Cancelled
        } else {
            ProcessMsg::Result(result)
        };

        if let Err(err) = sender.send_blocking(msg) {
            log::error!("Failed to send library action result: {err:?}");
        }
    });

    ProcessHandle {
        receiver,
        cancellation,
    }
}

impl Library {
    /// Import from a music library ZIP archive at `path`.
    ///
    /// Cancelling leaves the library partially populated: the metadata is
    /// imported in one transaction before any audio file is copied, so a
    /// cancelled import can leave tracks whose files are missing. Running the
    /// same import again completes it, since existing files and rows are
    /// skipped.
    pub fn import_library_from_zip(
        &self,
        path: impl AsRef<Path>,
        source: Source,
    ) -> Result<ProcessHandle> {
        log::info!(
            "Importing library from ZIP at {}",
            path.as_ref().to_string_lossy()
        );
        let path = path.as_ref().to_owned();
        let library_folder = PathBuf::from(&self.folder());
        let this_connection = self.connection.clone();

        Ok(spawn_process(move |sender, cancellation| {
            import_library_from_zip_priv(
                path,
                library_folder,
                source,
                this_connection,
                sender,
                cancellation,
            )
        }))
    }

    /// Export the whole music library to a ZIP archive at `path`. If `path` already exists, it
    /// will be overwritten. The work will be done in a background thread.
    pub fn export_library_to_zip(&self, path: impl AsRef<Path>) -> Result<ProcessHandle> {
        log::info!(
            "Exporting library to ZIP at {}",
            path.as_ref().to_string_lossy()
        );
        let connection = &mut *self.conn();

        let path = path.as_ref().to_owned();
        let library_folder = PathBuf::from(&self.folder());
        let tracks = tracks::table.load::<tables::Track>(connection)?;

        Ok(spawn_process(move |sender, cancellation| {
            export_library_to_zip_priv(path, library_folder, tracks, sender, cancellation)
        }))
    }

    /// Import from a library archive at `url`.
    ///
    /// See [`Library::import_library_from_zip`] for what cancelling leaves
    /// behind.
    pub fn import_library_from_url(&self, url: &str, source: Source) -> Result<ProcessHandle> {
        log::info!("Importing library from URL {url}");
        let url = url.to_owned();
        let library_folder = PathBuf::from(&self.folder());
        let this_connection = self.connection.clone();

        Ok(spawn_process(move |sender, cancellation| {
            import_library_from_url_priv(
                url,
                library_folder,
                source,
                this_connection,
                sender,
                cancellation,
            )
        }))
    }

    /// Import from metadata from a database file at `url`.
    pub fn import_metadata_from_url(&self, url: &str) -> Result<ProcessHandle> {
        log::info!("Importing metadata from URL {url}");

        let url = url.to_owned();
        let this_connection = self.connection.clone();
        let cache_dir = self.metadata_cache_dir.clone();

        Ok(spawn_process(move |sender, cancellation| {
            import_metadata_from_url_priv(url, cache_dir, this_connection, sender, cancellation)
        }))
    }
}

// TODO: Add options whether to keep stats.
fn import_library_from_zip_priv(
    zip_path: impl AsRef<Path>,
    library_folder: impl AsRef<Path>,
    source: Source,
    this_connection: Arc<Mutex<SqliteConnection>>,
    sender: &async_channel::Sender<ProcessMsg>,
    cancellation: &Cancellation,
) -> Result<()> {
    let mut archive = zip::ZipArchive::new(BufReader::new(fs::File::open(zip_path)?))?;

    let archive_db_file = archive.by_name("musicus.db")?;
    let tmp_db_file = NamedTempFile::new()?;
    std::io::copy(
        &mut BufReader::new(archive_db_file),
        &mut BufWriter::new(tmp_db_file.as_file()),
    )?;

    cancellation.check()?;

    // Import metadata.
    let tracks = import_metadata_from_file(tmp_db_file.path(), source, this_connection, false)?;

    // Import audio files.
    
    // avoid div by 0
    let n_tracks = tracks.len().max(1);
    
    for (index, track) in tracks.into_iter().enumerate() {
        cancellation.check()?;

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
    cancellation: &Cancellation,
) -> Result<()> {
    let mut zip = zip::ZipWriter::new(BufWriter::new(fs::File::create(zip_path)?));

    // Start with the database:
    add_file_to_zip(&mut zip, &library_folder, "musicus.db")?;

    // avoid div by 0
    let n_tracks = tracks.len().max(1);

    // Include all tracks that are part of the library.
    for (index, track) in tracks.into_iter().enumerate() {
        cancellation.check()?;

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
    cache_dir: PathBuf,
    this_connection: Arc<Mutex<SqliteConnection>>,
    sender: &async_channel::Sender<ProcessMsg>,
    cancellation: &Cancellation,
) -> Result<()> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    let _ = sender.send_blocking(ProcessMsg::Message(
        formatx!(gettext("Downloading {}"), &url).unwrap(),
    ));

    let db_path = metadata_file_path(&cache_dir);

    runtime.block_on(download_file(&url, &db_path, sender, cancellation))?;
    cancellation.check()?;

    let _ = sender.send_blocking(ProcessMsg::Message(gettext("Importing downloaded library")));

    update_metadata_from_file(&db_path, this_connection)
}

fn import_library_from_url_priv(
    url: String,
    library_folder: impl AsRef<Path>,
    source: Source,
    this_connection: Arc<Mutex<SqliteConnection>>,
    sender: &async_channel::Sender<ProcessMsg>,
    cancellation: &Cancellation,
) -> Result<()> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    let _ = sender.send_blocking(ProcessMsg::Message(
        formatx!(gettext("Downloading {}"), &url).unwrap(),
    ));

    let archive_file = runtime.block_on(download_tmp_file(&url, sender, cancellation))?;
    cancellation.check()?;

    let _ = sender.send_blocking(ProcessMsg::Message(gettext("Importing downloaded library")));

    import_library_from_zip_priv(
        archive_file.path(),
        library_folder,
        source,
        this_connection,
        sender,
        cancellation,
    )
}

/// Update metadata from the database file at `path`.
fn update_metadata_from_file(
    path: impl AsRef<Path>,
    this_connection: Arc<Mutex<SqliteConnection>>,
) -> Result<()> {
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
    let albums = albums::table.load::<tables::Album>(&mut other_connection)?;
    let album_recordings =
        album_recordings::table.load::<tables::AlbumRecording>(&mut other_connection)?;

    let mut this_connection = db::lock_connection(&this_connection);

    this_connection.transaction::<(), Error, _>(|connection| {
        for person in persons {
            let enable_updates = persons::table
                .filter(persons::person_id.eq(&person.person_id))
                .select(persons::enable_updates)
                .first(connection)
                .optional()?;

            if enable_updates == Some(true) {
                diesel::update(persons::table.filter(persons::person_id.eq(&person.person_id)))
                    .set(persons::name.eq(person.name))
                    .execute(connection)?;
            }
        }

        for role in roles {
            let enable_updates = roles::table
                .filter(roles::role_id.eq(&role.role_id))
                .select(roles::enable_updates)
                .first(connection)
                .optional()?;

            if enable_updates == Some(true) {
                diesel::update(roles::table.filter(roles::role_id.eq(&role.role_id)))
                    .set(roles::name.eq(role.name))
                    .execute(connection)?;
            }
        }

        for instrument in instruments {
            let enable_updates = instruments::table
                .filter(instruments::instrument_id.eq(&instrument.instrument_id))
                .select(instruments::enable_updates)
                .first(connection)
                .optional()?;

            if enable_updates == Some(true) {
                diesel::update(
                    instruments::table
                        .filter(instruments::instrument_id.eq(&instrument.instrument_id)),
                )
                .set(instruments::name.eq(instrument.name))
                .execute(connection)?;
            }
        }

        for work in works {
            let enable_updates = works::table
                .filter(works::work_id.eq(&work.work_id))
                .select(works::enable_updates)
                .first(connection)
                .optional()?;

            if enable_updates == Some(true) {
                diesel::update(works::table.filter(works::work_id.eq(&work.work_id)))
                    .set(works::name.eq(work.name.clone()))
                    .execute(connection)?;

                diesel::delete(work_persons::table.filter(work_persons::work_id.eq(&work.work_id)))
                    .execute(connection)?;

                for work_person in work_persons
                    .iter()
                    .filter(|work_person| work_person.work_id == work.work_id)
                {
                    diesel::insert_into(work_persons::table)
                        .values(work_person)
                        .execute(connection)?;
                }

                diesel::delete(
                    work_instruments::table.filter(work_instruments::work_id.eq(&work.work_id)),
                )
                .execute(connection)?;

                for work_instrument in work_instruments
                    .iter()
                    .filter(|work_instrument| work_instrument.work_id == work.work_id)
                {
                    diesel::insert_into(work_instruments::table)
                        .values(work_instrument)
                        .execute(connection)?;
                }
            }
        }

        for ensemble in ensembles {
            let enable_updates = ensembles::table
                .filter(ensembles::ensemble_id.eq(&ensemble.ensemble_id))
                .select(ensembles::enable_updates)
                .first(connection)
                .optional()?;

            if enable_updates == Some(true) {
                diesel::update(
                    ensembles::table.filter(ensembles::ensemble_id.eq(&ensemble.ensemble_id)),
                )
                .set(ensembles::name.eq(ensemble.name.clone()))
                .execute(connection)?;

                diesel::delete(
                    ensemble_persons::table
                        .filter(ensemble_persons::ensemble_id.eq(&ensemble.ensemble_id)),
                )
                .execute(connection)?;

                for ensemble_person in ensemble_persons
                    .iter()
                    .filter(|ensemble_person| ensemble_person.ensemble_id == ensemble.ensemble_id)
                {
                    diesel::insert_into(ensemble_persons::table)
                        .values(ensemble_person)
                        .execute(connection)?;
                }
            }
        }

        for recording in recordings {
            let enable_updates = recordings::table
                .filter(recordings::recording_id.eq(&recording.recording_id))
                .select(recordings::enable_updates)
                .first(connection)
                .optional()?;

            if enable_updates == Some(true) {
                diesel::update(
                    recordings::table.filter(recordings::recording_id.eq(&recording.recording_id)),
                )
                .set(recordings::year.eq(recording.year))
                .execute(connection)?;

                diesel::delete(
                    recording_persons::table
                        .filter(recording_persons::recording_id.eq(&recording.recording_id)),
                )
                .execute(connection)?;

                for recording_person in recording_persons.iter().filter(|recording_person| {
                    recording_person.recording_id == recording.recording_id
                }) {
                    diesel::insert_into(recording_persons::table)
                        .values(recording_person)
                        .execute(connection)?;
                }

                diesel::delete(
                    recording_ensembles::table
                        .filter(recording_ensembles::recording_id.eq(&recording.recording_id)),
                )
                .execute(connection)?;

                for recording_ensemble in recording_ensembles.iter().filter(|recording_ensemble| {
                    recording_ensemble.recording_id == recording.recording_id
                }) {
                    diesel::insert_into(recording_ensembles::table)
                        .values(recording_ensemble)
                        .execute(connection)?;
                }
            }
        }

        for album in albums {
            let enable_updates = albums::table
                .filter(albums::album_id.eq(&album.album_id))
                .select(albums::enable_updates)
                .first(connection)
                .optional()?;

            if enable_updates == Some(true) {
                diesel::update(albums::table.filter(albums::album_id.eq(&album.album_id)))
                    .set(albums::name.eq(album.name.clone()))
                    .execute(connection)?;

                diesel::delete(
                    album_recordings::table.filter(album_recordings::album_id.eq(&album.album_id)),
                )
                .execute(connection)?;

                for album_recording in album_recordings
                    .iter()
                    .filter(|album_recording| album_recording.album_id == album.album_id)
                {
                    diesel::insert_into(album_recordings::table)
                        .values(album_recording)
                        .execute(connection)?;
                }
            }
        }

        Ok(())
    })?;

    Ok(())
}

/// Import metadata from the database file at `path`.
///
/// If `ignore_tracks` is `true`, tracks and associated items like mediums will not be imported
/// from the database. In that case, if the database contains tracks, a warning will be logged.
/// In any case, tracks are returned.
fn import_metadata_from_file(
    path: impl AsRef<Path>,
    source: Source,
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

    let mut this_connection = db::lock_connection(&this_connection);

    this_connection.transaction::<(), Error, _>(|connection| {
        for mut person in persons {
            person.source = source;
            person.created_at = now;
            person.edited_at = now;
            person.last_used_at = now;
            person.last_played_at = None;

            diesel::insert_into(persons::table)
                .values(person)
                .on_conflict_do_nothing()
                .execute(connection)?;
        }

        for mut role in roles {
            role.source = source;
            role.created_at = now;
            role.edited_at = now;
            role.last_used_at = now;

            diesel::insert_into(roles::table)
                .values(role)
                .on_conflict_do_nothing()
                .execute(connection)?;
        }

        for mut instrument in instruments {
            instrument.source = source;
            instrument.created_at = now;
            instrument.edited_at = now;
            instrument.last_used_at = now;
            instrument.last_played_at = None;

            diesel::insert_into(instruments::table)
                .values(instrument)
                .on_conflict_do_nothing()
                .execute(connection)?;
        }

        for mut work in works {
            work.source = source;
            work.created_at = now;
            work.edited_at = now;
            work.last_used_at = now;
            work.last_played_at = None;

            diesel::insert_into(works::table)
                .values(work)
                .on_conflict_do_nothing()
                .execute(connection)?;
        }

        for work_person in work_persons {
            diesel::insert_into(work_persons::table)
                .values(work_person)
                .on_conflict_do_nothing()
                .execute(connection)?;
        }

        for work_instrument in work_instruments {
            diesel::insert_into(work_instruments::table)
                .values(work_instrument)
                .on_conflict_do_nothing()
                .execute(connection)?;
        }

        for mut ensemble in ensembles {
            ensemble.source = source;
            ensemble.created_at = now;
            ensemble.edited_at = now;
            ensemble.last_used_at = now;
            ensemble.last_played_at = None;

            diesel::insert_into(ensembles::table)
                .values(ensemble)
                .on_conflict_do_nothing()
                .execute(connection)?;
        }

        for ensemble_person in ensemble_persons {
            diesel::insert_into(ensemble_persons::table)
                .values(ensemble_person)
                .on_conflict_do_nothing()
                .execute(connection)?;
        }

        for mut recording in recordings {
            recording.source = source;
            recording.created_at = now;
            recording.edited_at = now;
            recording.last_used_at = now;
            recording.last_played_at = None;

            diesel::insert_into(recordings::table)
                .values(recording)
                .on_conflict_do_nothing()
                .execute(connection)?;
        }

        for recording_person in recording_persons {
            diesel::insert_into(recording_persons::table)
                .values(recording_person)
                .on_conflict_do_nothing()
                .execute(connection)?;
        }

        for recording_ensemble in recording_ensembles {
            diesel::insert_into(recording_ensembles::table)
                .values(recording_ensemble)
                .on_conflict_do_nothing()
                .execute(connection)?;
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
                    .execute(connection)?;
            }

            for track_work in track_works {
                diesel::insert_into(track_works::table)
                    .values(track_work)
                    .on_conflict_do_nothing()
                    .execute(connection)?;
            }

            for mut medium in mediums {
                medium.created_at = now;
                medium.edited_at = now;
                medium.last_used_at = now;
                medium.last_played_at = None;

                diesel::insert_into(mediums::table)
                    .values(medium)
                    .on_conflict_do_nothing()
                    .execute(connection)?;
            }
        }

        for mut album in albums {
            album.source = source;
            album.created_at = now;
            album.edited_at = now;
            album.last_used_at = now;
            album.last_played_at = None;

            diesel::insert_into(albums::table)
                .values(album)
                .on_conflict_do_nothing()
                .execute(connection)?;
        }

        for album_recording in album_recordings {
            diesel::insert_into(album_recordings::table)
                .values(album_recording)
                .on_conflict_do_nothing()
                .execute(connection)?;
        }

        for album_medium in album_mediums {
            diesel::insert_into(album_mediums::table)
                .values(album_medium)
                .on_conflict_do_nothing()
                .execute(connection)?;
        }

        Ok(())
    })?;

    Ok(tracks)
}

/// How long a download may stall before it is considered failed.
///
/// There is deliberately no overall request timeout: a library archive can
/// legitimately take a long time to transfer. This bounds the time without any
/// progress instead, which is what actually distinguishes a stalled server from
/// a slow one.
const DOWNLOAD_READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

/// Download `url` into `path`, replacing whatever is already there.
///
/// The download goes to a temporary file next to `path` and is only moved into
/// place once it has completed, so that an interrupted download cannot leave a
/// truncated file behind for the next run to pick up.
async fn download_file(
    url: &str,
    path: impl AsRef<Path>,
    sender: &async_channel::Sender<ProcessMsg>,
    cancellation: &Cancellation,
) -> Result<()> {
    let path = path.as_ref();
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("Download target {} has no parent directory", path.display()))?;

    tokio::fs::create_dir_all(parent).await?;

    let file = download_to_tmp_file(url, Some(parent), sender, cancellation).await?;
    file.persist(path)?;

    Ok(())
}

/// Download `url` into a temporary file in the system temporary directory.
async fn download_tmp_file(
    url: &str,
    sender: &async_channel::Sender<ProcessMsg>,
    cancellation: &Cancellation,
) -> Result<NamedTempFile> {
    download_to_tmp_file(url, None, sender, cancellation).await
}

/// Download `url` into a new temporary file, in `dir` if given, reporting
/// progress on `sender` if the server announced a content length.
async fn download_to_tmp_file(
    url: &str,
    dir: Option<&Path>,
    sender: &async_channel::Sender<ProcessMsg>,
    cancellation: &Cancellation,
) -> Result<NamedTempFile> {
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .read_timeout(DOWNLOAD_READ_TIMEOUT)
        .build()?;

    let response = client.get(url).send().await?;
    response.error_for_status_ref()?;

    let total_size = response.content_length();
    let mut body_stream = response.bytes_stream();

    let file = match dir {
        Some(dir) => NamedTempFile::new_in(dir)?,
        None => NamedTempFile::new()?,
    };

    let mut writer =
        tokio::io::BufWriter::new(tokio::fs::File::from_std(file.as_file().try_clone()?));

    let mut downloaded = 0;
    while let Some(chunk) = body_stream.next().await {
        cancellation.check()?;

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

pub fn metadata_file_path(cache_dir: &Path) -> PathBuf {
    cache_dir.join("metadata.muslib")
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::db::TranslatedString;

    /// Drain a process channel until it reports a result, returning that result.
    fn wait_for_result(handle: ProcessHandle) -> Result<()> {
        loop {
            match handle.receiver.recv_blocking() {
                Ok(ProcessMsg::Result(result)) => return result,
                Ok(ProcessMsg::Cancelled) => return Err(anyhow!("process was cancelled")),
                Ok(_) => continue,
                Err(_) => return Err(anyhow!("process channel closed without a result")),
            }
        }
    }

    fn translated(name: &str) -> TranslatedString {
        let mut translations = std::collections::HashMap::new();
        translations.insert("generic".to_string(), name.to_string());
        TranslatedString(translations)
    }

    /// Populate `library` with one person, one work, one recording, and one track (backed
    /// by a small placeholder audio file), returning the created recording.
    fn populate(library: &Library, track_source: &Path) -> crate::db::models::Recording {
        let person = library
            .create_person(translated("Ludwig van Beethoven"), true)
            .unwrap();

        let work = library
            .create_work(
                translated("Symphony No. 5"),
                Vec::new(),
                vec![crate::db::models::Composer { person, role: None }],
                Vec::new(),
                true,
            )
            .unwrap();

        let recording = library
            .create_recording(work.clone(), None, Vec::new(), Vec::new(), true)
            .unwrap();

        library
            .import_track(track_source, &recording.recording_id, 0, vec![work])
            .unwrap();

        recording
    }

    #[test]
    fn library_round_trips_through_zip_export_and_import() {
        let source_dir = TempDir::new().unwrap();
        let source_cache_dir = TempDir::new().unwrap();
        let source = Library::new(source_dir.path(), source_cache_dir.path()).unwrap();

        let track_source_file = source_dir.path().join("source_track.mp3");
        fs::write(&track_source_file, b"not actually audio").unwrap();

        let recording = populate(&source, &track_source_file);

        let zip_path = source_dir.path().join("export.muslib");
        let handle = source.export_library_to_zip(&zip_path).unwrap();
        wait_for_result(handle).unwrap();

        let dest_dir = TempDir::new().unwrap();
        let dest_cache_dir = TempDir::new().unwrap();
        let dest = Library::new(dest_dir.path(), dest_cache_dir.path()).unwrap();

        let handle = dest
            .import_library_from_zip(&zip_path, Source::Import)
            .unwrap();
        wait_for_result(handle).unwrap();

        let persons = dest.search_persons("Beethoven").unwrap();
        assert_eq!(persons.len(), 1);
        assert_eq!(persons[0].item.name.get(), "Ludwig van Beethoven");

        let tracks = dest.tracks_for_recording(&recording.recording_id).unwrap();
        assert_eq!(tracks.len(), 1);
        assert_eq!(
            fs::read(dest_dir.path().join(&tracks[0].path)).unwrap(),
            b"not actually audio"
        );
    }

    /// Cancelling before the operation starts must report Cancelled rather than
    /// a failure, and must not produce a Result message.
    #[test]
    fn cancelled_export_reports_cancellation() {
        let dir = TempDir::new().unwrap();
        let cache_dir = TempDir::new().unwrap();
        let library = Library::new(dir.path(), cache_dir.path()).unwrap();

        let track_source_file = dir.path().join("source_track.mp3");
        fs::write(&track_source_file, b"not actually audio").unwrap();
        populate(&library, &track_source_file);

        let handle = library
            .export_library_to_zip(dir.path().join("export.muslib"))
            .unwrap();
        handle.cancellation.cancel();

        let mut terminal = None;
        while let Ok(msg) = handle.receiver.recv_blocking() {
            match msg {
                ProcessMsg::Result(_) | ProcessMsg::Cancelled => {
                    assert!(terminal.is_none(), "more than one terminal message");
                    terminal = Some(msg);
                }
                _ => {}
            }
        }

        assert!(
            matches!(terminal, Some(ProcessMsg::Cancelled)),
            "expected Cancelled, got {terminal:?}"
        );
    }

    #[test]
    fn metadata_import_respects_enable_updates() {
        let dest_dir = TempDir::new().unwrap();
        let dest_cache_dir = TempDir::new().unwrap();
        let dest = Library::new(dest_dir.path(), dest_cache_dir.path()).unwrap();

        // enable_updates = false: this person should not be overwritten by a metadata import.
        let person = dest
            .create_person(translated("Original Name"), false)
            .unwrap();

        // A separate "remote" metadata database with a same-ID person under a different name.
        let remote_dir = TempDir::new().unwrap();
        let remote_db_path = remote_dir.path().join("musicus.db");
        let mut remote_connection = db::connect(remote_db_path.to_str().unwrap()).unwrap();
        let now = Local::now().naive_local();
        diesel::insert_into(persons::table)
            .values(tables::Person {
                person_id: person.person_id.clone(),
                name: translated("Renamed"),
                source: Source::User,
                enable_updates: true,
                created_at: now,
                edited_at: now,
                last_used_at: now,
                last_played_at: None,
            })
            .execute(&mut remote_connection)
            .unwrap();

        update_metadata_from_file(&remote_db_path, dest.connection.clone()).unwrap();

        let after_disabled = dest.search_persons("Original Name").unwrap();
        assert_eq!(after_disabled.len(), 1, "name must stay unchanged");

        // Re-enable updates for this person, then re-run the same import.
        diesel::update(persons::table.filter(persons::person_id.eq(&person.person_id)))
            .set(persons::enable_updates.eq(true))
            .execute(&mut *dest.conn())
            .unwrap();

        update_metadata_from_file(&remote_db_path, dest.connection.clone()).unwrap();

        let after_enabled = dest.search_persons("Renamed").unwrap();
        assert_eq!(after_enabled.len(), 1, "name should now be updated");
    }
}
