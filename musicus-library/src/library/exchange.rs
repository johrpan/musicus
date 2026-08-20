use std::{
    fs::{self, File},
    io::{BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
};

use anyhow::{anyhow, bail, Error, Result};
use diesel::{prelude::*, SqliteConnection};
use futures_util::StreamExt;
use gettextrs::gettext;
use tempfile::{NamedTempFile, TempDir};
use tokio::io::AsyncWriteExt;
use zip::{write::SimpleFileOptions, ZipWriter};

use super::Library;
use crate::{
    db::{
        self,
        schema::*,
        tables::{self, Source},
    },
    format_translated,
    process::{Cancellation, ProcessHandle, ProcessMsg},
};

/// The name of the manifest entry inside a `.muslib` archive.
const MANIFEST_NAME: &str = "manifest.json";

/// The version of the `.muslib` archive layout this build writes.
///
/// Bump when the archive gains, loses or renames entries. Archives with a
/// higher version are refused rather than partially understood.
const ARCHIVE_FORMAT_VERSION: u32 = 1;

/// Describes a `.muslib` archive.
///
/// Archives written before this existed have no manifest at all; they are
/// treated as format version 0 and imported as before.
#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct ArchiveManifest {
    format_version: u32,
    /// The schema version of the `musicus.db` inside the archive, so that an
    /// unreadable archive can be rejected before anything is extracted.
    schema_version: i32,
    created_at: String,
    n_tracks: usize,
}

/// Read and check the manifest of an opened archive.
fn read_manifest(
    archive: &mut zip::ZipArchive<BufReader<fs::File>>,
) -> Result<Option<ArchiveManifest>> {
    let manifest: ArchiveManifest = match archive.by_name(MANIFEST_NAME) {
        Ok(file) => serde_json::from_reader(BufReader::new(file))?,
        Err(zip::result::ZipError::FileNotFound) => {
            log::info!("Archive has no manifest; assuming the pre-versioning layout");
            return Ok(None);
        }
        Err(err) => return Err(err.into()),
    };

    if manifest.format_version > ARCHIVE_FORMAT_VERSION {
        bail!(
            "This archive was created by a newer version of Musicus \
             (archive format version {}, this version supports {ARCHIVE_FORMAT_VERSION}).",
            manifest.format_version
        );
    }

    if manifest.schema_version > db::SCHEMA_VERSION {
        bail!(
            "This archive contains a library from a newer version of Musicus \
             (library schema version {}, this version supports {}).",
            manifest.schema_version,
            db::SCHEMA_VERSION
        );
    }

    Ok(Some(manifest))
}

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
    /// cancelled import can leave tracks whose files are missing. That is
    /// deliberate, because it keeps cancelling a large import cheap.
    ///
    /// Running the same import again copies exactly what is still missing.
    /// Every audio file is written next to its destination and only moved into
    /// place once it is whole, so a file that is present at its final path is
    /// always complete — a crash or a full disk in the middle of a copy cannot
    /// leave a truncated file that later runs would skip.
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
    ///
    /// Private tags are personal to this library and are left out, together
    /// with every assignment referring to one.
    pub fn export_library_to_zip(&self, path: impl AsRef<Path>) -> Result<ProcessHandle> {
        log::info!(
            "Exporting library to ZIP at {}",
            path.as_ref().to_string_lossy()
        );
        let connection = &mut *self.conn();

        let path = path.as_ref().to_owned();
        let library_folder = PathBuf::from(&self.folder());
        let tracks = tracks::table.load::<tables::Track>(connection)?;
        let this_connection = self.connection.clone();

        Ok(spawn_process(move |sender, cancellation| {
            export_library_to_zip_priv(
                path,
                library_folder,
                this_connection,
                tracks,
                sender,
                cancellation,
            )
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

fn import_library_from_zip_priv(
    zip_path: impl AsRef<Path>,
    library_folder: impl AsRef<Path>,
    source: Source,
    this_connection: Arc<Mutex<SqliteConnection>>,
    sender: &async_channel::Sender<ProcessMsg>,
    cancellation: &Cancellation,
) -> Result<()> {
    let mut archive = zip::ZipArchive::new(BufReader::new(fs::File::open(zip_path)?))?;

    // Refuse an archive this build cannot read before extracting anything.
    read_manifest(&mut archive)?;

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
        let mut part_path = library_track_file_path.clone();
        part_path.as_mut_os_string().push(".part");

        // Skip tracks that are already present.
        if fs::exists(&library_track_file_path)? {
            // A file at its final path is always complete, so anything left
            // next to it comes from an earlier interrupted run of this import
            // and is garbage.
            if part_path.exists() {
                if let Err(err) = fs::remove_file(&part_path) {
                    log::warn!(
                        "Failed to remove leftover temporary file {}: {err}",
                        part_path.display()
                    );
                }
            }
        } else {
            if let Some(parent) = library_track_file_path.parent() {
                fs::create_dir_all(parent)?;
            }

            let archive_track_file = archive.by_name(&path_to_zip(&track.path)?)?;

            // Copy through a temporary file next to the destination and only
            // move it into place once it is whole. A crash, a kill or a full
            // disk in the middle of the copy would otherwise leave a truncated
            // file that every later run of this import mistakes for a finished
            // track and skips forever.
            let result = copy_to_file(archive_track_file, &part_path)
                .and_then(|()| Ok(fs::rename(&part_path, &library_track_file_path)?));

            if let Err(err) = result {
                if let Err(err) = fs::remove_file(&part_path) {
                    log::warn!(
                        "Failed to remove partially copied track {}: {err}",
                        part_path.display()
                    );
                }

                return Err(err);
            }
        }

        // Ignore if the reveiver has been dropped.
        let _ = sender.send_blocking(ProcessMsg::Progress((index + 1) as f64 / n_tracks as f64));
    }

    Ok(())
}

/// Write all of `source` to a new file at `path` and make sure it reached the
/// disk before returning.
///
/// The caller renames the result into place, which is only safe once the bytes
/// are durable.
fn copy_to_file(source: impl Read, path: impl AsRef<Path>) -> Result<()> {
    let file = File::create(path)?;

    let mut writer = BufWriter::new(&file);
    std::io::copy(&mut BufReader::new(source), &mut writer)?;
    writer.flush()?;
    drop(writer);

    file.sync_all()?;

    Ok(())
}

/// Copy the library database into a temporary directory, without the listening
/// history and the privatetags, along with the assignments referring to those
/// tags.
///
/// An export is meant to be handed to someone else, so neither must be in the
/// archive at all — not even as a row nothing points at. The copy is made with
/// `VACUUM INTO`, which writes a consistent snapshot of the database, and
/// vacuumed once more after the deletes so that no freed page still carries the
/// removed rows.
///
/// The tag assignments that stay keep their sequence numbers, which may now
/// have gaps. Only their order matters, and the editors rewrite a work's or
/// recording's tags as a whole anyway.
///
/// The returned directory owns the copy and deletes it when dropped, so it has
/// to outlive the export.
fn database_for_export(connection: &mut SqliteConnection) -> Result<TempDir> {
    let dir = TempDir::new()?;
    let path = dir.path().join("musicus.db");
    let path = path
        .to_str()
        .ok_or_else(|| anyhow!("The temporary directory path is not valid Unicode"))?;

    diesel::sql_query("VACUUM INTO ?")
        .bind::<diesel::sql_types::Text, _>(path)
        .execute(connection)?;

    let copy = &mut SqliteConnection::establish(path)?;

    copy.transaction::<_, Error, _>(|copy| {
        diesel::delete(plays::table).execute(copy)?;

        let private_tag_ids = tags::table
            .filter(tags::private.eq(true))
            .select(tags::tag_id)
            .load::<String>(copy)?;

        diesel::delete(work_tags::table.filter(work_tags::tag_id.eq_any(&private_tag_ids)))
            .execute(copy)?;

        diesel::delete(
            recording_tags::table.filter(recording_tags::tag_id.eq_any(&private_tag_ids)),
        )
        .execute(copy)?;

        diesel::delete(tags::table.filter(tags::tag_id.eq_any(&private_tag_ids))).execute(copy)?;

        Ok(())
    })?;

    diesel::sql_query("VACUUM").execute(copy)?;

    Ok(dir)
}

fn export_library_to_zip_priv(
    zip_path: impl AsRef<Path>,
    library_folder: impl AsRef<Path>,
    this_connection: Arc<Mutex<SqliteConnection>>,
    tracks: Vec<tables::Track>,
    sender: &async_channel::Sender<ProcessMsg>,
    cancellation: &Cancellation,
) -> Result<()> {
    // Every export ships the sanitized copy rather than the file on disk. Every
    // library has a listening history to strip, so there is no case left where
    // shipping the original would be correct, and skipping the copy on a library
    // that happens to have no private tags would silently reintroduce the leak.
    // The copy happens here rather than in the caller, so that the work does not
    // block the thread that started the export.
    let database = {
        let connection = &mut *db::lock_connection(&this_connection);
        database_for_export(connection)?
    };

    cancellation.check()?;

    let mut zip = zip::ZipWriter::new(BufWriter::new(fs::File::create(zip_path)?));

    // Describe the archive first, so that an importer can decide whether it can
    // read the rest before extracting anything.
    let manifest = ArchiveManifest {
        format_version: ARCHIVE_FORMAT_VERSION,
        schema_version: db::SCHEMA_VERSION,
        created_at: db::now().and_utc().to_rfc3339(),
        n_tracks: tracks.len(),
    };

    zip.start_file(MANIFEST_NAME, SimpleFileOptions::default())?;
    zip.write_all(&serde_json::to_vec_pretty(&manifest)?)?;

    // Without the database the archive would be worthless, so this file is the
    // one that is not allowed to be missing.
    let database_path = database.path().join("musicus.db");

    if !add_file_to_zip(&mut zip, &database_path, "musicus.db")? {
        bail!("The library database is missing");
    }

    // avoid div by 0
    let n_tracks = tracks.len().max(1);
    let mut n_missing = 0;

    // Include all tracks that are part of the library.
    for (index, track) in tracks.into_iter().enumerate() {
        cancellation.check()?;

        if !add_file_to_zip(
            &mut zip,
            library_folder.as_ref().join(&track.path),
            &path_to_zip(&track.path)?,
        )? {
            n_missing += 1;
        }

        // Ignore if the reveiver has been dropped.
        let _ = sender.send_blocking(ProcessMsg::Progress((index + 1) as f64 / n_tracks as f64));
    }

    zip.finish()?;

    if n_missing > 0 {
        let _ = sender.send_blocking(ProcessMsg::Warning(format_translated!(
            gettext("{} track files were missing and could not be exported."),
            n_missing
        )));
    }

    Ok(())
}

/// Add the file at `file_path` to `zip` under the archive path `zip_path`.
///
/// The two are given separately because the database may be read from a
/// sanitized copy outside the library folder while still going into the archive
/// where an importer expects it.
///
/// Returns whether the file was there. A library can legitimately be missing
/// audio files — an import that was cancelled or interrupted leaves exactly
/// that — and not being able to back up the rest of it would be worse than an
/// incomplete archive.
fn add_file_to_zip(
    zip: &mut ZipWriter<BufWriter<File>>,
    file_path: impl AsRef<Path>,
    zip_path: &str,
) -> Result<bool> {
    let file_path = file_path.as_ref();

    let mut file = match File::open(file_path) {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            log::warn!("Skipping missing library file {}", file_path.display());
            return Ok(false);
        }
        Err(err) => return Err(err.into()),
    };

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    zip.start_file(zip_path, SimpleFileOptions::default())?;
    zip.write_all(&buffer)?;

    Ok(true)
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

    let _ = sender.send_blocking(ProcessMsg::Message(format_translated!(
        gettext("Downloading {}"),
        &url
    )));

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

    let _ = sender.send_blocking(ProcessMsg::Message(format_translated!(
        gettext("Downloading {}"),
        &url
    )));

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
    let tags = tags::table.load::<tables::Tag>(&mut other_connection)?;
    let work_tags = work_tags::table.load::<tables::WorkTag>(&mut other_connection)?;
    let recording_tags =
        recording_tags::table.load::<tables::RecordingTag>(&mut other_connection)?;

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

        for tag in tags {
            let enable_updates = tags::table
                .filter(tags::tag_id.eq(&tag.tag_id))
                .select(tags::enable_updates)
                .first(connection)
                .optional()?;

            if enable_updates == Some(true) {
                // Only the name is merged. Whether a tag takes a value decides
                // what the assignments already in this library mean, so an
                // update from the catalogue must not change it underneath them.
                diesel::update(tags::table.filter(tags::tag_id.eq(&tag.tag_id)))
                    .set(tags::name.eq(tag.name))
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

                diesel::delete(work_tags::table.filter(work_tags::work_id.eq(&work.work_id)))
                    .execute(connection)?;

                for work_tag in work_tags
                    .iter()
                    .filter(|work_tag| work_tag.work_id == work.work_id)
                {
                    diesel::insert_into(work_tags::table)
                        .values(work_tag)
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
                diesel::delete(
                    recording_tags::table
                        .filter(recording_tags::recording_id.eq(&recording.recording_id)),
                )
                .execute(connection)?;

                for recording_tag in recording_tags
                    .iter()
                    .filter(|recording_tag| recording_tag.recording_id == recording.recording_id)
                {
                    diesel::insert_into(recording_tags::table)
                        .values(recording_tag)
                        .execute(connection)?;
                }

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
/// If `ignore_tracks` is `true`, tracks will not be imported from the database.
/// In that case, if the database contains tracks, a warning will be logged. In
/// any case, tracks are returned.
fn import_metadata_from_file(
    path: impl AsRef<Path>,
    source: Source,
    this_connection: Arc<Mutex<SqliteConnection>>,
    ignore_tracks: bool,
) -> Result<Vec<tables::Track>> {
    let now = db::now();

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
    let albums = albums::table.load::<tables::Album>(&mut other_connection)?;
    let album_recordings =
        album_recordings::table.load::<tables::AlbumRecording>(&mut other_connection)?;
    let tags = tags::table.load::<tables::Tag>(&mut other_connection)?;
    let work_tags = work_tags::table.load::<tables::WorkTag>(&mut other_connection)?;
    let recording_tags =
        recording_tags::table.load::<tables::RecordingTag>(&mut other_connection)?;

    // Import metadata that is not already present.

    let mut this_connection = db::lock_connection(&this_connection);

    this_connection.transaction::<(), Error, _>(|connection| {
        for mut person in persons {
            person.source = source;
            person.created_at = now;
            person.edited_at = now;
            person.last_used_at = now;

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

        for mut tag in tags {
            tag.source = source;
            tag.created_at = now;
            tag.edited_at = now;
            tag.last_used_at = now;

            diesel::insert_into(tags::table)
                .values(tag)
                .on_conflict_do_nothing()
                .execute(connection)?;
        }

        for mut instrument in instruments {
            instrument.source = source;
            instrument.created_at = now;
            instrument.edited_at = now;
            instrument.last_used_at = now;

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

        for work_tag in work_tags {
            diesel::insert_into(work_tags::table)
                .values(work_tag)
                .on_conflict_do_nothing()
                .execute(connection)?;
        }

        for mut ensemble in ensembles {
            ensemble.source = source;
            ensemble.created_at = now;
            ensemble.edited_at = now;
            ensemble.last_used_at = now;

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

        for recording_tag in recording_tags {
            diesel::insert_into(recording_tags::table)
                .values(recording_tag)
                .on_conflict_do_nothing()
                .execute(connection)?;
        }

        if !ignore_tracks {
            for mut track in tracks.clone() {
                track.created_at = now;
                track.edited_at = now;
                track.last_used_at = now;

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
        }

        for mut album in albums {
            album.source = source;
            album.created_at = now;
            album.edited_at = now;
            album.last_used_at = now;

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

    /// Drain a process channel, returning its result together with every
    /// warning it reported on the way.
    fn wait_for_result_and_warnings(handle: ProcessHandle) -> (Result<()>, Vec<String>) {
        let mut warnings = Vec::new();

        loop {
            match handle.receiver.recv_blocking() {
                Ok(ProcessMsg::Result(result)) => return (result, warnings),
                Ok(ProcessMsg::Cancelled) => {
                    return (Err(anyhow!("process was cancelled")), warnings)
                }
                Ok(ProcessMsg::Warning(warning)) => warnings.push(warning),
                Ok(_) => continue,
                Err(_) => {
                    return (
                        Err(anyhow!("process channel closed without a result")),
                        warnings,
                    )
                }
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
                Vec::new(),
                None,
                true,
            )
            .unwrap();

        let recording = library
            .create_recording(work.clone(), Vec::new(), Vec::new(), Vec::new(), None, true)
            .unwrap();

        library
            .import_track(track_source, &recording.recording_id, 0, vec![work])
            .unwrap();

        recording
    }

    /// Extract the database out of an archive into `dir` and open it, so that a
    /// test can look at what an export actually ships rather than at what an
    /// import chose to keep.
    fn database_in_archive(zip_path: &Path, dir: &TempDir) -> SqliteConnection {
        let mut archive = zip::ZipArchive::new(fs::File::open(zip_path).unwrap()).unwrap();
        let path = dir.path().join("musicus.db");
        copy_to_file(archive.by_name("musicus.db").unwrap(), &path).unwrap();
        SqliteConnection::establish(path.to_str().unwrap()).unwrap()
    }

    /// The listening history is nobody else's business, so an archive must not
    /// contain it — however the import happens to treat it.
    #[test]
    fn export_leaves_out_the_listening_history() {
        let source_dir = TempDir::new().unwrap();
        let source_cache_dir = TempDir::new().unwrap();
        let source = Library::new(source_dir.path(), source_cache_dir.path()).unwrap();

        let track_source_file = source_dir.path().join("source_track.mp3");
        fs::write(&track_source_file, b"not actually audio").unwrap();
        let recording = populate(&source, &track_source_file);

        let tracks = source
            .tracks_for_recording(&recording.recording_id)
            .unwrap();
        source.track_played(&tracks[0].track_id).unwrap();

        let zip_path = source_dir.path().join("export.muslib");
        wait_for_result(source.export_library_to_zip(&zip_path).unwrap()).unwrap();

        let archive_dir = TempDir::new().unwrap();
        let archive = &mut database_in_archive(&zip_path, &archive_dir);
        let plays = plays::table.count().get_result::<i64>(archive).unwrap();
        assert_eq!(plays, 0, "the archive must not carry any plays");

        // Exporting is not supposed to cost the library its own statistics.
        let plays = plays::table
            .count()
            .get_result::<i64>(&mut *source.conn())
            .unwrap();
        assert_eq!(plays, 1);
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

    /// A private tag is personal to its library: neither the tag nor the
    /// assignment referring to it may end up in an archive.
    #[test]
    fn export_leaves_out_private_tags() {
        let source_dir = TempDir::new().unwrap();
        let source_cache_dir = TempDir::new().unwrap();
        let source = Library::new(source_dir.path(), source_cache_dir.path()).unwrap();

        let track_source_file = source_dir.path().join("source_track.mp3");
        fs::write(&track_source_file, b"not actually audio").unwrap();
        let recording = populate(&source, &track_source_file);

        let public = source
            .create_tag(translated("Baroque"), false, false, true)
            .unwrap();
        let private = source
            .create_tag(translated("Favourite"), false, true, true)
            .unwrap();

        source
            .update_recording(
                &recording.recording_id,
                recording.work.clone(),
                Vec::new(),
                Vec::new(),
                vec![
                    crate::db::models::TagValue {
                        tag: public.clone(),
                        value: None,
                    },
                    crate::db::models::TagValue {
                        tag: private.clone(),
                        value: None,
                    },
                ],
                None,
                true,
            )
            .unwrap();

        let zip_path = source_dir.path().join("export.muslib");
        wait_for_result(source.export_library_to_zip(&zip_path).unwrap()).unwrap();

        let dest_dir = TempDir::new().unwrap();
        let dest_cache_dir = TempDir::new().unwrap();
        let dest = Library::new(dest_dir.path(), dest_cache_dir.path()).unwrap();
        wait_for_result(
            dest.import_library_from_zip(&zip_path, Source::Import)
                .unwrap(),
        )
        .unwrap();

        let names = dest
            .search_tags("")
            .unwrap()
            .into_iter()
            .map(|item| item.item.name.get().to_string())
            .collect::<Vec<_>>();
        assert!(names.contains(&"Baroque".to_owned()));
        assert!(
            !names.contains(&"Favourite".to_owned()),
            "the private tag must not be exported: {names:?}"
        );

        // The assignment must be gone too, not just the tag it points at.
        let connection = &mut *dest.conn();
        let tag_ids = recording_tags::table
            .select(recording_tags::tag_id)
            .load::<String>(connection)
            .unwrap();
        assert_eq!(tag_ids, vec![public.tag_id]);

        // The source library keeps its private tag.
        assert!(source
            .search_tags("Favourite")
            .unwrap()
            .iter()
            .any(|item| item.item.tag_id == private.tag_id));
    }

    #[test]
    fn export_writes_a_manifest_describing_the_archive() {
        let dir = TempDir::new().unwrap();
        let cache_dir = TempDir::new().unwrap();
        let library = Library::new(dir.path(), cache_dir.path()).unwrap();

        let track_source_file = dir.path().join("source_track.mp3");
        fs::write(&track_source_file, b"not actually audio").unwrap();
        populate(&library, &track_source_file);

        let zip_path = dir.path().join("export.muslib");
        wait_for_result(library.export_library_to_zip(&zip_path).unwrap()).unwrap();

        let mut archive =
            zip::ZipArchive::new(BufReader::new(fs::File::open(&zip_path).unwrap())).unwrap();
        let manifest = read_manifest(&mut archive).unwrap().unwrap();

        assert_eq!(manifest.format_version, ARCHIVE_FORMAT_VERSION);
        assert_eq!(manifest.schema_version, db::SCHEMA_VERSION);
        assert_eq!(manifest.n_tracks, 1);
    }

    /// An archive claiming a format this build does not know must be refused
    /// rather than partially understood.
    #[test]
    fn import_refuses_an_archive_from_a_newer_version() {
        let dir = TempDir::new().unwrap();
        let cache_dir = TempDir::new().unwrap();
        let library = Library::new(dir.path(), cache_dir.path()).unwrap();

        let zip_path = dir.path().join("future.muslib");
        let mut zip = ZipWriter::new(BufWriter::new(fs::File::create(&zip_path).unwrap()));
        zip.start_file(MANIFEST_NAME, SimpleFileOptions::default())
            .unwrap();
        zip.write_all(
            serde_json::to_string(&ArchiveManifest {
                format_version: ARCHIVE_FORMAT_VERSION + 1,
                schema_version: db::SCHEMA_VERSION,
                created_at: "2026-01-01T00:00:00+00:00".to_owned(),
                n_tracks: 0,
            })
            .unwrap()
            .as_bytes(),
        )
        .unwrap();
        zip.finish().unwrap();

        let err = wait_for_result(
            library
                .import_library_from_zip(&zip_path, Source::Import)
                .unwrap(),
        )
        .unwrap_err();

        assert!(
            err.to_string().contains("newer version of Musicus"),
            "unexpected error: {err}"
        );
    }

    /// Archives exported before manifests existed must still import.
    #[test]
    fn import_accepts_an_archive_without_a_manifest() {
        let source_dir = TempDir::new().unwrap();
        let source_cache_dir = TempDir::new().unwrap();
        let source = Library::new(source_dir.path(), source_cache_dir.path()).unwrap();

        let track_source_file = source_dir.path().join("source_track.mp3");
        fs::write(&track_source_file, b"not actually audio").unwrap();
        let recording = populate(&source, &track_source_file);

        let zip_path = source_dir.path().join("export.muslib");
        wait_for_result(source.export_library_to_zip(&zip_path).unwrap()).unwrap();

        // Rebuild the archive without its manifest entry.
        let legacy_path = source_dir.path().join("legacy.muslib");
        {
            let mut archive =
                zip::ZipArchive::new(BufReader::new(fs::File::open(&zip_path).unwrap())).unwrap();
            let mut legacy =
                ZipWriter::new(BufWriter::new(fs::File::create(&legacy_path).unwrap()));

            for i in 0..archive.len() {
                let mut entry = archive.by_index(i).unwrap();
                let name = entry.name().to_owned();
                if name == MANIFEST_NAME {
                    continue;
                }

                let mut buffer = Vec::new();
                entry.read_to_end(&mut buffer).unwrap();
                legacy
                    .start_file(name, SimpleFileOptions::default())
                    .unwrap();
                legacy.write_all(&buffer).unwrap();
            }

            legacy.finish().unwrap();
        }

        let dest_dir = TempDir::new().unwrap();
        let dest_cache_dir = TempDir::new().unwrap();
        let dest = Library::new(dest_dir.path(), dest_cache_dir.path()).unwrap();

        wait_for_result(
            dest.import_library_from_zip(&legacy_path, Source::Import)
                .unwrap(),
        )
        .unwrap();

        assert_eq!(
            dest.tracks_for_recording(&recording.recording_id)
                .unwrap()
                .len(),
            1
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
        let now = db::now();
        diesel::insert_into(persons::table)
            .values(tables::Person {
                person_id: person.person_id.clone(),
                name: translated("Renamed"),
                source: Source::User,
                enable_updates: true,
                created_at: now,
                edited_at: now,
                last_used_at: now,
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

    /// Export an archive of a library populated with one track, returning the
    /// archive path and the library relative path of that track.
    fn export_with_one_track(dir: &TempDir, cache_dir: &TempDir) -> (PathBuf, PathBuf) {
        let library = Library::new(dir.path(), cache_dir.path()).unwrap();

        let track_source_file = dir.path().join("source_track.mp3");
        fs::write(&track_source_file, b"not actually audio").unwrap();
        let recording = populate(&library, &track_source_file);

        let track_path = library
            .tracks_for_recording(&recording.recording_id)
            .unwrap()
            .remove(0)
            .path;

        let zip_path = dir.path().join("export.muslib");
        wait_for_result(library.export_library_to_zip(&zip_path).unwrap()).unwrap();

        (zip_path, track_path)
    }

    /// A copy that was interrupted half way leaves a temporary file. An import
    /// that runs afterwards has to finish the track, which it could not do if
    /// the incomplete data had been written to the final path.
    #[test]
    fn import_finishes_a_track_left_half_copied() {
        let source_dir = TempDir::new().unwrap();
        let source_cache_dir = TempDir::new().unwrap();
        let (zip_path, track_path) = export_with_one_track(&source_dir, &source_cache_dir);

        let dest_dir = TempDir::new().unwrap();
        let dest_cache_dir = TempDir::new().unwrap();
        let dest = Library::new(dest_dir.path(), dest_cache_dir.path()).unwrap();

        let final_path = dest_dir.path().join(&track_path);
        let mut part_path = final_path.clone();
        part_path.as_mut_os_string().push(".part");
        fs::write(&part_path, b"truncat").unwrap();

        wait_for_result(
            dest.import_library_from_zip(&zip_path, Source::Import)
                .unwrap(),
        )
        .unwrap();

        assert_eq!(fs::read(&final_path).unwrap(), b"not actually audio");
        assert!(!part_path.exists(), "the temporary file should be gone");
    }

    #[test]
    fn import_removes_a_stale_temporary_file_next_to_a_finished_track() {
        let source_dir = TempDir::new().unwrap();
        let source_cache_dir = TempDir::new().unwrap();
        let (zip_path, track_path) = export_with_one_track(&source_dir, &source_cache_dir);

        let dest_dir = TempDir::new().unwrap();
        let dest_cache_dir = TempDir::new().unwrap();
        let dest = Library::new(dest_dir.path(), dest_cache_dir.path()).unwrap();

        wait_for_result(
            dest.import_library_from_zip(&zip_path, Source::Import)
                .unwrap(),
        )
        .unwrap();

        let final_path = dest_dir.path().join(&track_path);
        let mut part_path = final_path.clone();
        part_path.as_mut_os_string().push(".part");
        fs::write(&part_path, b"left over").unwrap();

        // Running the same import again is how an interrupted import is
        // finished, so it has to tidy up after the interrupted one.
        wait_for_result(
            dest.import_library_from_zip(&zip_path, Source::Import)
                .unwrap(),
        )
        .unwrap();

        assert!(
            !part_path.exists(),
            "the stale temporary file should be gone"
        );
        assert_eq!(fs::read(&final_path).unwrap(), b"not actually audio");
    }

    /// A library whose files are incomplete — after a cancelled import, say —
    /// must still be exportable, otherwise it cannot be backed up at all.
    #[test]
    fn export_skips_a_missing_track_file() {
        let dir = TempDir::new().unwrap();
        let cache_dir = TempDir::new().unwrap();
        let library = Library::new(dir.path(), cache_dir.path()).unwrap();

        let track_source_file = dir.path().join("source_track.mp3");
        fs::write(&track_source_file, b"not actually audio").unwrap();
        let recording = populate(&library, &track_source_file);

        let track_path = library
            .tracks_for_recording(&recording.recording_id)
            .unwrap()
            .remove(0)
            .path;
        fs::remove_file(dir.path().join(&track_path)).unwrap();

        let zip_path = dir.path().join("export.muslib");
        let (result, warnings) =
            wait_for_result_and_warnings(library.export_library_to_zip(&zip_path).unwrap());
        result.unwrap();

        assert_eq!(warnings.len(), 1, "the user has to be told what is missing");
        assert!(warnings[0].contains('1'));

        let mut archive =
            zip::ZipArchive::new(BufReader::new(fs::File::open(&zip_path).unwrap())).unwrap();
        assert!(archive.by_name(MANIFEST_NAME).is_ok());
        assert!(archive.by_name("musicus.db").is_ok());
        assert!(
            archive.by_name(&path_to_zip(&track_path).unwrap()).is_err(),
            "the missing track must simply not be in the archive"
        );
    }
}
