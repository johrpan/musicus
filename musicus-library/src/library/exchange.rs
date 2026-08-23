use std::{
    fs::{self, File},
    io::{BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
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
    library::process::{spawn_process, Cancellation, ProcessHandle, ProcessMsg},
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
    /// The schema version of the `musicus.musdb` inside the archive, so that an
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

    let archive_db_file = archive.by_name("musicus.musdb")?;
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
    let path = dir.path().join("musicus.musdb");
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
    let database_path = database.path().join("musicus.musdb");

    if !add_file_to_zip(&mut zip, &database_path, "musicus.musdb")? {
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
    cache_dir.join("metadata.musdb")
}
