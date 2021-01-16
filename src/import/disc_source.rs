use anyhow::{anyhow, bail, Result};
use discid::DiscId;
use futures_channel::oneshot;
use gstreamer::prelude::*;
use gstreamer::{Element, ElementFactory, Pipeline};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::thread;

/// Representation of an audio CD being imported as a medium.
#[derive(Clone, Debug)]
pub struct DiscSource {
    /// The MusicBrainz DiscID of the CD.
    pub discid: String,

    /// The path to the temporary directory where the audio files will be.
    pub path: PathBuf,

    /// The tracks on this disc.
    pub tracks: Vec<TrackSource>,
}

/// Representation of a single track on an audio CD.
#[derive(Clone, Debug)]
pub struct TrackSource {
    /// The track number. This is different from the index in the disc
    /// source's tracks list, because it is not defined from which number the
    /// the track numbers start.
    pub number: u32,

    /// The path to the temporary file to which the track will be ripped. The
    /// file will not exist until the track is actually ripped.
    pub path: PathBuf,
}

impl DiscSource {
    /// Try to create a new disc source by asynchronously reading the
    /// information from the default disc drive.
    pub async fn load() -> Result<Self> {
        let (sender, receiver) = oneshot::channel();

        thread::spawn(|| {
            let disc = Self::load_priv();
            sender.send(disc).unwrap();
        });

        let disc = receiver.await??;

        Ok(disc)
    }

    /// Rip the whole disc asynchronously. After this method has finished
    /// successfully, the audio files will be available in the specified
    /// location for each track source.
    pub async fn rip(&self) -> Result<()> {
        for track in &self.tracks {
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

    /// Load the disc from the default disc drive.
    fn load_priv() -> Result<Self> {
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

            let track = TrackSource {
                number,
                path,
            };

            tracks.push(track);
        }

        let disc = DiscSource {
            discid: id,
            tracks,
            path: tmp_dir,
        };

        Ok(disc)
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
