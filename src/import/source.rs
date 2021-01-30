use anyhow::Result;
use async_trait::async_trait;
use std::path::PathBuf;

/// A source for tracks that can be imported into the music library.
#[async_trait]
pub trait Source {
    /// Load the source and discover the contained tracks.
    async fn load(&self) -> Result<()>;

    /// Get a reference to the tracks within this source, if they are ready.
    fn tracks(&self) -> Option<&[SourceTrack]>;

    /// Get the DiscID of the corresponging medium, if possible.
    fn discid(&self) -> Option<String>;

    /// Asynchronously copy the tracks to the files that are advertised within
    /// their corresponding objects.
    async fn copy(&self) -> Result<()>;
}

/// Representation of a single track on a source.
#[derive(Clone, Debug)]
pub struct SourceTrack {
    /// The track number. This is different from the index in the disc
    /// source's tracks list, because it is not defined from which number the
    /// the track numbers start.
    pub number: u32,

    /// A human readable identifier for the track. This will be used to
    /// present the track for selection.
    pub name: String,

    /// The path to the file where the corresponding audio file is. This file
    /// is only required to exist, once the source's copy method has finished.
    /// This will not be the actual file within the user's music library, but
    /// the location from which it can be copied to the music library.
    pub path: PathBuf,
}
