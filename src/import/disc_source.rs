use super::source::{Source, SourceTrack};
use anyhow::{anyhow, bail, Result};
use async_trait::async_trait;
use discid::DiscId;
use futures_channel::oneshot;
use gstreamer::prelude::*;
use gstreamer::{Element, ElementFactory, Pipeline};
use once_cell::sync::OnceCell;
use std::path::{Path, PathBuf};
use std::thread;

/// Representation of an audio CD being imported as a medium.
#[derive(Clone, Debug)]
pub struct DiscSource {
    /// The MusicBrainz DiscID of the CD.
    pub discid: OnceCell<String>,

    /// The tracks on this disc.
    tracks: OnceCell<Vec<SourceTrack>>,
}

impl DiscSource {
    /// Create a new disc source. The source has to be initialized by calling
    /// load() afterwards.
    pub fn new() -> Result<Self> {
        let result = Self {
            discid: OnceCell::new(),
            tracks: OnceCell::new(),
        };

        Ok(result)
    }

    /// Load the disc from the default disc drive and return the MusicBrainz
    /// DiscID as well as descriptions of the contained tracks.
    fn load_priv() -> Result<(String, Vec<SourceTrack>)> {
        let discid = DiscId::read(None)?;
        let id = discid.id();

        let mut tracks = Vec::new();

        let first_track = discid.first_track_num() as u32;
        let last_track = discid.last_track_num() as u32;

        let tmp_dir = Self::create_tmp_dir()?;

        for number in first_track..=last_track {
            let file_name = format!("track_{:02}.flac", number);

            let mut path = tmp_dir.clone();
            path.push(file_name);

            let track = SourceTrack {
                number,
                path,
            };

            tracks.push(track);
        }

        Ok((id, tracks))
    }

    /// Create a new temporary directory and return its path.
    // TODO: Move to a more appropriate place.
    fn create_tmp_dir() -> Result<PathBuf> {
        let mut tmp_dir = glib::get_tmp_dir()
            .ok_or_else(|| {
                anyhow!("Failed to get temporary directory using glib::get_tmp_dir()!")
            })?;

        let dir_name = format!("musicus-{}", rand::random::<u64>());
        tmp_dir.push(dir_name);

        std::fs::create_dir(&tmp_dir)?;

        Ok(tmp_dir)
    }

    /// Rip one track.
    fn rip_track(path: &Path, number: u32) -> Result<()> {
        let pipeline = Self::build_pipeline(path, number)?;

        let bus = pipeline
            .get_bus()
            .ok_or_else(|| anyhow!("Failed to get bus from pipeline!"))?;

        pipeline.set_state(gstreamer::State::Playing)?;

        for msg in bus.iter_timed(gstreamer::CLOCK_TIME_NONE) {
            use gstreamer::MessageView::*;

            match msg.view() {
                Eos(..) => break,
                Error(err) => {
                    pipeline.set_state(gstreamer::State::Null)?;
                    bail!("GStreamer error: {:?}!", err);
                }
                _ => (),
            }
        }

        pipeline.set_state(gstreamer::State::Null)?;

        Ok(())
    }

    /// Build the GStreamer pipeline to rip a track.
    fn build_pipeline(path: &Path, number: u32) -> Result<Pipeline> {
        let cdparanoiasrc = ElementFactory::make("cdparanoiasrc", None)?;
        cdparanoiasrc.set_property("track", &number)?;

        let queue = ElementFactory::make("queue", None)?;
        let audioconvert = ElementFactory::make("audioconvert", None)?;
        let flacenc = ElementFactory::make("flacenc", None)?;

        let path_str = path.to_str().ok_or_else(|| {
            anyhow!("Failed to convert path '{:?}' to string!", path)
        })?;

        let filesink = gstreamer::ElementFactory::make("filesink", None)?;
        filesink.set_property("location", &path_str.to_owned())?;

        let pipeline = gstreamer::Pipeline::new(None);
        pipeline.add_many(&[&cdparanoiasrc, &queue, &audioconvert, &flacenc, &filesink])?;

        Element::link_many(&[&cdparanoiasrc, &queue, &audioconvert, &flacenc, &filesink])?;

        Ok(pipeline)
    }
}

#[async_trait]
impl Source for DiscSource {
    async fn load(&self) -> Result<()> {
        let (sender, receiver) = oneshot::channel();

        thread::spawn(|| {
            let result = Self::load_priv();
            sender.send(result).unwrap();
        });

        let (discid, tracks) = receiver.await??;

        self.discid.set(discid);
        self.tracks.set(tracks);

        Ok(())
    }

    fn tracks(&self) -> Option<&[SourceTrack]> {
        match self.tracks.get() {
            Some(tracks) => Some(tracks.as_slice()),
            None => None,
        }
    }

    fn discid(&self) -> Option<String> {
        match self.discid.get() {
            Some(discid) => Some(discid.to_owned()),
            None => None,
        }
    }

    async fn copy(&self) -> Result<()> {
        let tracks = self.tracks.get()
            .ok_or_else(|| anyhow!("Tried to copy disc before loading has finished!"))?;

        for track in tracks {
            let (sender, receiver) = oneshot::channel();

            let number = track.number;
            let path = track.path.clone();

            thread::spawn(move || {
                let result = Self::rip_track(&path, number);
                sender.send(result).unwrap();
            });

            receiver.await??;
        }

        Ok(())
    }
}
