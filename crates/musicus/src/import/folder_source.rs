use super::source::{Source, SourceTrack};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures_channel::oneshot;
use once_cell::sync::OnceCell;
use std::path::{Path, PathBuf};
use std::thread;

/// A folder outside of the music library that contains tracks to import.
#[derive(Clone, Debug)]
pub struct FolderSource {
    /// The path to the folder.
    path: PathBuf,

    /// The tracks within the folder.
    tracks: OnceCell<Vec<SourceTrack>>,
}

impl FolderSource {
    /// Create a new folder source.
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            tracks: OnceCell::new(),
        }
    }

    /// Load the contents of the folder as tracks.
    fn load_priv(path: &Path) -> Result<Vec<SourceTrack>> {
        let mut tracks = Vec::new();
        let mut number = 1;

        for entry in std::fs::read_dir(path)? {
            let entry = entry?;

            if entry.file_type()?.is_file() {
                let name = entry
                    .file_name()
                    .into_string()
                    .or_else(|_| Err(anyhow!("Failed to convert OsString to String!")))?;

                let path = entry.path();

                let track = SourceTrack {
                    number,
                    name,
                    path,
                };

                tracks.push(track);
                number += 1;
            }
        }

        Ok(tracks)
    }
}

#[async_trait]
impl Source for FolderSource {
    async fn load(&self) -> Result<()> {
        let (sender, receiver) = oneshot::channel();

        let path = self.path.clone();
        thread::spawn(move || {
            let result = Self::load_priv(&path);
            sender.send(result).unwrap();
        });

        let tracks = receiver.await??;
        self.tracks.set(tracks).unwrap();

        Ok(())
    }

    fn tracks(&self) -> Option<&[SourceTrack]> {
        match self.tracks.get() {
            Some(tracks) => Some(tracks.as_slice()),
            None => None,
        }
    }

    fn discid(&self) -> Option<String> {
        None
    }

    async fn copy(&self) -> Result<()> {
        Ok(())
    }
}
