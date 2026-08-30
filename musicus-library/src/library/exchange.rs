//! Synchronous exchange operations.
//!
//! Archive import, archive export and metadata installation run on the calling
//! thread; the application schedules them on its own worker threads. They are
//! atomic at their boundaries: a failed or cancelled export never destroys a
//! previous valid archive, and an import applies its metadata and its audio
//! files only when the whole batch succeeds.

use std::{
    collections::HashSet,
    fs::{self, File},
    io::{BufReader, BufWriter, Read, Write},
    ops::ControlFlow,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use anyhow::{anyhow, bail, Error, Result};
use diesel::{prelude::*, SqliteConnection};
use tempfile::{NamedTempFile, TempDir};
use zip::{write::SimpleFileOptions, ZipWriter};

use crate::{
    db::{self, schema::*, tables, tables::Source},
    error::{LibraryError, Result as LibraryResult},
    library::{
        import_graph::{apply_import, DestinationState, ImportGraph, ImportReport},
        notify_changed,
        refresh::refresh_metadata,
        Library,
    },
};

/// The name of the manifest entry inside a `.muslib` archive.
pub const MANIFEST_NAME: &str = "manifest.json";

/// The version of the `.muslib` archive layout this build writes.
///
/// Bump when the archive gains, loses or renames entries. Archives with a
/// higher version are refused rather than partially understood.
const ARCHIVE_FORMAT_VERSION: u32 = 1;

/// A progress or status update from a synchronous exchange operation.
#[derive(Clone, Debug)]
pub enum ExchangeEvent {
    Message(String),
    Warning(String),
    /// How much of the work is done, from 0.0 to 1.0.
    Progress(f64),
}

/// What a library import wrote into the destination.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExchangeReport {
    /// Rows written into the destination.
    pub inserted: usize,
    /// Entities that already existed locally and were kept unchanged.
    pub reused_local: usize,
    /// Imported rows skipped because their entity already exists locally.
    pub skipped: usize,
}

impl From<ImportReport> for ExchangeReport {
    fn from(report: ImportReport) -> Self {
        Self {
            inserted: report.inserted,
            reused_local: report.reused_local,
            skipped: report.skipped,
        }
    }
}

/// Forward an event to `observer`, failing with cancellation if the observer
/// asks the operation to stop.
fn observe(
    observer: &mut dyn FnMut(ExchangeEvent) -> ControlFlow<()>,
    event: ExchangeEvent,
) -> Result<()> {
    match observer(event) {
        ControlFlow::Continue(()) => Ok(()),
        ControlFlow::Break(()) => Err(LibraryError::Cancelled.into()),
    }
}

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
fn read_manifest(archive: &mut zip::ZipArchive<BufReader<fs::File>>) -> Result<()> {
    let manifest: ArchiveManifest = match archive.by_name(MANIFEST_NAME) {
        Ok(file) => serde_json::from_reader(BufReader::new(file))?,
        Err(zip::result::ZipError::FileNotFound) => {
            log::info!("Archive has no manifest; assuming the pre-versioning layout");
            return Ok(());
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

    Ok(())
}

/// A thread-safe handle on a library's connection and folders, for scheduling
/// synchronous exchange operations off-thread.
///
/// The application creates one on the main thread and moves a clone into its
/// worker thread; every function in this module only needs the context. The
/// fields are private: callers obtain the context from
/// [`Library::exchange_context`] and hand it to the exchange functions.
#[derive(Clone)]
pub struct ExchangeContext {
    /// The library's database connection.
    connection: Arc<Mutex<SqliteConnection>>,
    /// The folder the library's database and track files live in.
    folder: PathBuf,
    /// The directory cache files (like the metadata database) live in.
    cache_dir: PathBuf,
    /// Emit the library's change notification, for use after a committed
    /// mutation on a worker thread.
    notify: Arc<dyn Fn() + Send + Sync>,
}

impl ExchangeContext {
    /// Notify subscribers that the library changed.
    ///
    /// Only calls this after a mutation has committed; a notification never
    /// precedes the commit.
    pub(crate) fn notify(&self) {
        (self.notify)();
    }
}

impl Library {
    /// A cloneable handle on this library's connection and folders.
    ///
    /// Clone it and move the clone to a worker thread to run the synchronous
    /// [`exchange`](crate::library::exchange) operations there.
    pub fn exchange_context(&self) -> ExchangeContext {
        ExchangeContext {
            connection: self.connection.clone(),
            folder: self.folder.clone(),
            cache_dir: self.cache_dir.clone(),
            notify: {
                let changed_senders = self.changed_senders.clone();
                Arc::new(move || notify_changed(&changed_senders))
            },
        }
    }
}

/// Import a music library from the `.muslib` archive at `path` into the
/// library whose connection `connection` opens.
///
/// This runs on the calling thread; the application schedules it on a worker
/// thread. Cancellation is cooperative: the observer returns
/// [`ControlFlow::Break`] to ask the operation to stop, and the operation
/// stops between units of work, leaving the library in its prior state or in
/// a consistent partially-imported state whose metadata and track files
/// agree. Rerunning the same import copies exactly what is still missing.
///
/// Every audio file is written next to its destination and only moved into
/// place once it is whole, so a file that is present at its final path is
/// always complete — a crash or a full disk in the middle of a copy cannot
/// leave a truncated file that later runs would skip.
pub fn import_library_zip(
    context: &ExchangeContext,
    path: impl AsRef<Path>,
    source: Source,
    observer: &mut dyn FnMut(ExchangeEvent) -> ControlFlow<()>,
) -> LibraryResult<ExchangeReport> {
    log::info!(
        "Importing library from ZIP at {}",
        path.as_ref().to_string_lossy()
    );

    let mut archive = zip::ZipArchive::new(BufReader::new(fs::File::open(path)?))?;

    // Refuse an archive this build cannot read before extracting anything.
    read_manifest(&mut archive)?;

    let archive_db_file = archive.by_name("musicus.musdb")?;
    let tmp_db_file = NamedTempFile::new()?;
    std::io::copy(
        &mut BufReader::new(archive_db_file),
        &mut BufWriter::new(tmp_db_file.as_file()),
    )?;

    observe(observer, ExchangeEvent::Progress(0.0))?;

    // Load and validate the source graph, and check the archive's audio
    // entries against it, before a single row of the destination changes.
    let mut source_connection = db::connect(tmp_db_file.path())?;
    let graph = ImportGraph::load(&mut source_connection)?;
    validate_archive_entries(&mut archive, &graph)?;

    observe(observer, ExchangeEvent::Progress(0.1))?;

    // Import metadata, applying local-wins to whatever already exists.
    let mut this_connection = db::lock_connection(&context.connection);

    let (report, tracks) = import_metadata_from_file(graph, source, &mut this_connection, false)?;

    // The metadata import has committed; exactly one notification follows the
    // successful mutation.
    context.notify();

    log::info!(
        "Import wrote {} rows, kept {} existing entities and skipped {} colliding rows",
        report.inserted,
        report.reused_local,
        report.skipped
    );

    // Import audio files.

    // avoid div by 0
    let n_tracks = tracks.len().max(1);

    for (index, track) in tracks.into_iter().enumerate() {
        // Cancellation points are also where progress is reported.
        observe(
            observer,
            ExchangeEvent::Progress(0.1 + 0.9 * (index as f64 / n_tracks as f64)),
        )?;

        let library_track_file_path = track.path.resolve(&context.folder);
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

            let archive_track_file = archive.by_name(track.path.to_zip_name())?;

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

                return Err(err.into());
            }
        }
    }

    Ok(report.into())
}

/// Check that the audio entries of an opened archive match the track paths of
/// its database, before anything is imported.
///
/// Every entry name must be a usable library-relative track path; entries must
/// be unique; every track the database declares must have its file in the
/// archive; and no extra entries may hide there. This guarantees that the
/// archive paths are never used to place files anywhere outside the managed
/// track subtree.
fn validate_archive_entries(
    archive: &mut zip::ZipArchive<BufReader<fs::File>>,
    graph: &ImportGraph,
) -> Result<()> {
    let mut entries = HashSet::new();

    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        let name = entry.name().to_owned();
        drop(entry);

        if name == MANIFEST_NAME || name == "musicus.musdb" {
            continue;
        }

        tables::LibraryTrackPath::try_from_str(&name)
            .map_err(|err| anyhow!("Archive entry {name:?} is not a usable track path: {err}"))?;

        if entries.contains(&name) {
            bail!("The archive contains two entries named {name:?}");
        }
        entries.insert(name);
    }

    let declared = graph
        .tracks
        .values()
        .map(|track| track.path.to_zip_name())
        .collect::<HashSet<_>>();

    for name in &declared {
        if !entries.contains(*name) {
            bail!("The archive is missing the track file {name:?}");
        }
    }

    for name in &entries {
        if !declared.contains(name.as_str()) {
            bail!("The archive contains the unexpected entry {name:?}");
        }
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
/// history and the private tags, along with the assignments referring to those
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

/// Export the whole music library to a `.muslib` archive at `path`, replacing
/// an existing file only once the new archive is complete.
///
/// This runs on the calling thread. The archive is written to a temporary file
/// next to `path`, synced, and only then moved onto `path`, so a failed or
/// cancelled export never destroys a previous valid archive. Track audio is
/// streamed into the archive without reading a whole track into memory.
///
/// Private tags are personal to this library and are left out, together with
/// every assignment referring to one. The exported track list is read from the
/// sanitized snapshot being archived, never from a live query taken earlier.
pub fn export_library_zip(
    context: &ExchangeContext,
    path: impl AsRef<Path>,
    observer: &mut dyn FnMut(ExchangeEvent) -> ControlFlow<()>,
) -> LibraryResult<()> {
    let path = path.as_ref().to_owned();
    log::info!("Exporting library to ZIP at {}", path.to_string_lossy());
    // Every export ships the sanitized copy rather than the file on disk. Every
    // library has a listening history to strip, so there is no case left where
    // shipping the original would be correct, and skipping the copy on a library
    // that happens to have no private tags would silently reintroduce the leak.
    let database = {
        let connection = &mut *db::lock_connection(&context.connection);
        database_for_export(connection)?
    };

    // The track list comes from the same snapshot that is being archived, so
    // the archive's database and its audio files can never disagree.
    let copy_path = database
        .path()
        .join("musicus.musdb")
        .to_str()
        .ok_or_else(|| {
            LibraryError::InvalidPath(
                "The temporary directory path is not valid Unicode".to_owned(),
            )
        })
        .map(str::to_owned)?;
    let copy = &mut SqliteConnection::establish(&copy_path)
        .map_err(|err| LibraryError::Other(anyhow::anyhow!(err)))?;
    let tracks = tracks::table.load::<tables::Track>(copy)?;

    observe(observer, ExchangeEvent::Progress(0.05))?;

    let destination: &Path = path.as_ref();
    let parent = destination.parent().ok_or_else(|| {
        anyhow!(
            "The destination {} has no parent directory",
            destination.display()
        )
    })?;
    fs::create_dir_all(parent)?;

    let file_name = destination
        .file_name()
        .ok_or_else(|| anyhow!("The destination {} has no file name", destination.display()))?;
    let tmp_path = parent.join(format!(".{}.new", file_name.to_string_lossy()));

    let mut n_missing_total: usize = 0;

    let result = (|| -> Result<()> {
        let file = File::create(&tmp_path)?;
        let mut zip = ZipWriter::new(BufWriter::new(file));

        // Describe the archive first, so that an importer can decide whether it
        // can read the rest before extracting anything.
        let manifest = ArchiveManifest {
            format_version: ARCHIVE_FORMAT_VERSION,
            schema_version: db::SCHEMA_VERSION,
            created_at: db::now().and_utc().to_rfc3339(),
            n_tracks: tracks.len(),
        };

        zip.start_file(MANIFEST_NAME, SimpleFileOptions::default())?;
        zip.write_all(&serde_json::to_vec_pretty(&manifest)?)?;

        // Without the database the archive would be worthless, so this file is
        // the one that is not allowed to be missing.
        let database_path = database.path().join("musicus.musdb");

        if !add_file_to_zip(&mut zip, &database_path, "musicus.musdb")? {
            bail!("The library database is missing");
        }

        // avoid div by 0
        let n_tracks = tracks.len().max(1);

        // Include all tracks that are part of the library, streaming each file.
        for (index, track) in tracks.into_iter().enumerate() {
            observe(
                observer,
                ExchangeEvent::Progress(0.05 + 0.95 * (index as f64 / n_tracks as f64)),
            )?;

            if !add_file_to_zip(
                &mut zip,
                track.path.resolve(&context.folder),
                track.path.to_zip_name(),
            )? {
                n_missing_total += 1;
            }
        }

        // Finish, flush and sync before anything is moved onto the destination.
        let writer = zip.finish()?;
        let file = writer.into_inner()?;
        file.sync_all()?;

        drop(file);

        Ok(())
    })();

    // Replace the destination only once the new archive is complete and on
    // disk; a failure leaves the previous archive untouched.
    match result {
        Ok(()) => {
            fs::rename(&tmp_path, destination)?;
        }
        Err(err) => {
            if let Err(remove_err) = fs::remove_file(&tmp_path) {
                log::warn!(
                    "Failed to remove incomplete archive {}: {remove_err}",
                    tmp_path.display()
                );
            }

            return Err(err.into());
        }
    }

    if n_missing_total > 0 {
        observe(
            observer,
            ExchangeEvent::Warning(format!(
                "{} track files were missing and could not be exported.",
                n_missing_total
            )),
        )?;
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

    // Stream the file into the archive with bounded memory: a track is never
    // read into memory whole.
    zip.start_file(zip_path, SimpleFileOptions::default())?;
    std::io::copy(&mut file, zip)?;

    Ok(true)
}

/// Install the metadata database at `path` into the library with the cache
/// directory `cache_dir`.
///
/// The file is validated (schema and graph) and its contents are applied to
/// the library through the catalog-driven refresh rules **before** the
/// currently usable cached database is replaced, and the replacement is
/// atomic. A malformed or unreadable download therefore never leaves the app
/// with a broken cache file.
pub fn install_metadata(
    context: &ExchangeContext,
    path: impl AsRef<Path>,
    observer: &mut dyn FnMut(ExchangeEvent) -> ControlFlow<()>,
) -> LibraryResult<()> {
    observe(
        observer,
        ExchangeEvent::Message("Validating the downloaded metadata database".to_owned()),
    )?;

    // Validate in place before anything of the current cache is touched.
    let mut other_connection = db::connect(path.as_ref())?;
    let graph = ImportGraph::load(&mut other_connection)?;

    observe(observer, ExchangeEvent::Progress(0.5))?;

    {
        let mut this_connection = db::lock_connection(&context.connection);
        refresh_metadata(&graph, &mut this_connection)?;
    }

    // The refresh has committed; exactly one notification follows it. The
    // cache file below is replaced only afterwards, so a notification never
    // precedes a state the app would observe as current.
    context.notify();

    // Atomically replace the cache file with the validated download.
    let cache = metadata_file_path(&context.cache_dir);
    if let Some(parent) = cache.parent() {
        fs::create_dir_all(parent)?;
    }

    let tmp = NamedTempFile::new_in(
        cache
            .parent()
            .ok_or_else(|| anyhow!("The metadata cache has no parent directory"))?,
    )?;
    fs::copy(path.as_ref(), tmp.path())?;
    tmp.persist(&cache)
        .map_err(|err| LibraryError::Io(err.error))?;

    observe(observer, ExchangeEvent::Progress(1.0))?;

    Ok(())
}

/// The cached metadata database path of `cache_dir`.
pub(crate) fn metadata_file_path(cache_dir: &Path) -> PathBuf {
    cache_dir.join("metadata.musdb")
}

/// Import metadata from the database file at `path` into the library
/// `connection` opens, applying deterministic local-wins semantics.
///
/// Every entity whose ID already exists locally is kept entirely unchanged,
/// including its owned associations. A colliding local track (by ID or by file
/// path) wins and its imported rows and file are skipped. If `ignore_tracks`
/// is `true`, tracks are not imported; the tracks that still need their audio
/// files are returned.
fn import_metadata_from_file(
    graph: ImportGraph,
    source: Source,
    connection: &mut SqliteConnection,
    ignore_tracks: bool,
) -> Result<(ImportReport, Vec<tables::Track>)> {
    // Plan local-wins against the destination before any row changes: a
    // structural conflict fails here, with the destination untouched.
    let destination = DestinationState::load(connection)?;
    let plan = graph.plan_merge(&destination, None)?;

    let tracks_to_import = if ignore_tracks {
        Vec::new()
    } else {
        graph
            .tracks
            .values()
            .filter(|track| !plan.skipped_tracks.contains(&track.track_id))
            .cloned()
            .collect::<Vec<tables::Track>>()
    };

    connection.transaction::<(), Error, _>(|connection| {
        apply_import(
            connection,
            &graph,
            &plan,
            &destination,
            None,
            source,
            !ignore_tracks,
        )
    })?;

    Ok((plan.report.into_inner(), tracks_to_import))
}
