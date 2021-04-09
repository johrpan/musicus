use crate::{disc, folder};
use crate::error::Result;
use std::path::PathBuf;
use std::thread;
use std::sync::Arc;
use tokio::sync::{oneshot, watch};

/// The current state of the import process.
#[derive(Clone, Debug)]
pub enum State {
    /// The import process has not been started yet.
    Waiting,

    /// The audio is copied from the source.
    Copying,

    /// The audio files are ready to be imported into the music library.
    Ready,

    /// An error has happened.
    Error,
}

/// Interface for importing audio tracks from a medium or folder.
pub struct ImportSession {
    /// A string identifying the source as specific as possible across platforms and formats.
    pub(super) source_id: String,

    /// The tracks that are available on the source.
    pub(super) tracks: Vec<ImportTrack>,

    /// A closure that has to be called to copy the tracks if set.
    pub(super) copy: Option<Box<dyn Fn() -> Result<()> + Send + Sync>>,

    /// Sender through which listeners are notified of state changes.
    pub(super) state_sender: watch::Sender<State>,

    /// Receiver for state changes.
    pub(super) state_receiver: watch::Receiver<State>,
}

impl ImportSession {
    /// Create a new import session for an audio CD.
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

    /// Retrieve the current state of the import process.
    pub fn state(&self) -> State {
        self.state_receiver.borrow().clone()
    }

    /// Wait for the next state change and get the new state.
    pub async fn state_change(&self) -> State {
        let mut receiver = self.state_receiver.clone();
        match receiver.changed().await {
            Ok(()) => self.state(),
            Err(_) => State::Error,
        }
    }

    /// Copy the tracks to their advertised locations in the background, if neccessary. The state
    /// will be updated as the import is done.
    pub fn copy(self: &Arc<Self>) {
        if self.copy.is_some() {
            let clone = Arc::clone(self);

            thread::spawn(move || {
                let copy = clone.copy.as_ref().unwrap();

                match copy() {
                    Ok(()) => clone.state_sender.send(State::Ready).unwrap(),
                    Err(_) => clone.state_sender.send(State::Error).unwrap(),
                }
            });
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
