use crate::{disc, folder};
use crate::error::Result;
use std::path::PathBuf;
use std::thread;
use std::sync::Arc;
use tokio::sync::oneshot;

/// Interface for importing audio tracks from a medium or folder.
pub struct ImportSession {
    /// A string identifying the source as specific as possible across platforms and formats.
    pub(super) source_id: String,

    /// The tracks that are available on the source.
    pub(super) tracks: Vec<ImportTrack>,

    /// A closure that has to be called to copy the tracks if set.
    pub(super) copy: Option<Box<dyn Fn() -> Result<()> + Send + Sync>>,
}

impl ImportSession {
    /// Create a new import session for a audio CD.
    pub async fn audio_cd() -> Result<Arc<Self>> {
        let (sender, receiver) = oneshot::channel();

        thread::spawn(move || {
            let result = disc::new();
            let _ = sender.send(result);
        });

        Ok(Arc::new(receiver.await??))
    }

    /// Create a new import session for a folder.
    pub async fn folder(path: PathBuf) -> Result<Arc<Self>> {
        let (sender, receiver) = oneshot::channel();

        thread::spawn(move || {
            let result = folder::new(path);
            let _ = sender.send(result);
        });

        Ok(Arc::new(receiver.await??))
    }

    /// Get a string identifying the source as specific as possible across platforms and mediums.
    pub fn source_id(&self) -> &str {
        &self.source_id
    }

    /// Get the tracks that are available on the source.
    pub fn tracks(&self) -> &[ImportTrack] {
        &self.tracks
    }

    /// Copy the tracks to their advertised locations, if neccessary.
    pub async fn copy(self: &Arc<Self>) -> Result<()> {
        if self.copy.is_some() {
            let clone = Arc::clone(self);
            let (sender, receiver) = oneshot::channel();

            thread::spawn(move || {
                let copy = clone.copy.as_ref().unwrap();
                sender.send(copy()).unwrap();
            });

            receiver.await?
        } else {
            Ok(())
        }
    }
}

/// A track on an import source.
#[derive(Clone, Debug)]
pub struct ImportTrack {
    /// The track number.
    pub number: u32,

    /// A human readable identifier for the track. This will be used to present the track for
    /// selection.
    pub name: String,

    /// The path to the file where the corresponding audio file is. This file is only required to
    /// exist, once the import was successfully completed. This will not be the actual file within
    /// the user's music library, but the temporary location from which it can be copied to the
    /// music library.
    pub path: PathBuf,

    /// The track's duration in milliseconds.
    pub duration: u64,
}
