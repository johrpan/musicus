use crate::error::{Error, Result};
use crate::session::{ImportSession, ImportTrack};
use gstreamer::ClockTime;
use gstreamer_pbutils::Discoverer;
use log::{warn, info};
use sha2::{Sha256, Digest};
use std::path::PathBuf;

/// Create a new import session for the specified folder.
pub(super) fn new(path: PathBuf) -> Result<ImportSession> {
    let mut tracks = Vec::new();
    let mut number: u32 = 1;
    let mut hasher = Sha256::new();
    let discoverer = Discoverer::new(ClockTime::from_seconds(1))?;

    for entry in std::fs::read_dir(path)? {
        let entry = entry?;

        if entry.file_type()?.is_file() {
            let path = entry.path();

            let uri = glib::filename_to_uri(&path, None)
                .or(Err(Error::u(format!("Failed to create URI from path: {:?}", path))))?;

            let info = discoverer.discover_uri(&uri)?;

            if !info.get_audio_streams().is_empty() {
                let duration = info.get_duration().mseconds()
                    .ok_or(Error::u(format!("Failed to get duration for {}.", uri)))?;

                let file_name = entry.file_name();
                let name = file_name.into_string()
                    .or(Err(Error::u(format!(
                        "Failed to convert OsString to String: {:?}", entry.file_name()))))?;

                hasher.update(duration.to_le_bytes());

                let track = ImportTrack {
                    number,
                    name,
                    path,
                    duration,
                };

                tracks.push(track);
                number += 1;
            } else {
                warn!("File {} skipped, because it doesn't contain any audio streams.", uri);
            }
        }
    }

    let source_id = base64::encode_config(hasher.finalize(), base64::URL_SAFE);

    info!("Source ID: {}", source_id);

    let session = ImportSession {
        source_id,
        tracks,
        copy: None,
    };

    Ok(session)
}
