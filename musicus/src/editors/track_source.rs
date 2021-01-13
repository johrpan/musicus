use anyhow::Result;
use std::cell::Cell;
use std::path::Path;

/// One track within a [`TrackSource`].
#[derive(Debug, Clone)]
pub struct TrackState {
    pub description: String,
}

/// A live representation of a source of audio tracks.
pub struct TrackSource {
    pub tracks: Vec<TrackState>,
    pub ready: Cell<bool>,
}

impl TrackSource {
    /// Create a new track source for a folder. This will provide the folder's
    /// files as selectable tracks and be ready immediately.
    pub fn folder(path: &Path) -> Result<Self> {
        let mut tracks = Vec::<TrackState>::new();
    
        let entries = std::fs::read_dir(path)?;
        for entry in entries {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let file_name = entry.file_name();
                let track = TrackState { description: file_name.to_str().unwrap().to_owned() };
                tracks.push(track);
            }
        }
        
        tracks.sort_unstable_by(|a, b| {
            a.description.cmp(&b.description)
        });

        Ok(Self {
            tracks,
            ready: Cell::new(true),
        })
    }
}
