use anyhow::{anyhow, bail, Result};
use discid::DiscId;
use futures_channel::oneshot;
use gstreamer::prelude::*;
use gstreamer::{Element, ElementFactory, Pipeline};
use std::cell::RefCell;
use std::thread;

/// A disc that can be ripped.
#[derive(Debug, Clone)]
pub struct RipDisc {
    pub discid: String,
    pub first_track: u32,
    pub last_track: u32,
}

/// An interface for ripping an audio compact disc.
pub struct Ripper {
    path: String,
    disc: RefCell<Option<RipDisc>>,
}

impl Ripper {
    /// Create a new ripper that stores its tracks within the specified folder.
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
            disc: RefCell::new(None),
        }
    }

    /// Load the disc and return its metadata.
    pub async fn load_disc(&self) -> Result<RipDisc> {
        let (sender, receiver) = oneshot::channel();

        thread::spawn(|| {
            let disc = Self::load_disc_priv();
            sender.send(disc).unwrap();
        });

        let disc = receiver.await??;
        self.disc.replace(Some(disc.clone()));

        Ok(disc)
    }

    /// Rip one track.
    pub async fn rip_track(&self, track: u32) -> Result<()> {
        let (sender, receiver) = oneshot::channel();

        let path = self.path.clone();
        thread::spawn(move || {
            let result = Self::rip_track_priv(&path, track);
            sender.send(result).unwrap();
        });

        receiver.await?
    }

    /// Load the disc and return its metadata.
    fn load_disc_priv() -> Result<RipDisc> {
        let discid = DiscId::read(None)?;
        let id = discid.id();
        let first_track = discid.first_track_num() as u32;
        let last_track = discid.last_track_num() as u32;

        let disc = RipDisc {
            discid: id,
            first_track,
            last_track,
        };

        Ok(disc)
    }

    /// Rip one track.
    fn rip_track_priv(path: &str, track: u32) -> Result<()> {
        let pipeline = Self::build_pipeline(path, track)?;

        let bus = pipeline
            .get_bus()
            .ok_or(anyhow!("Failed to get bus from pipeline!"))?;

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
    fn build_pipeline(path: &str, track: u32) -> Result<Pipeline> {
        let cdparanoiasrc = ElementFactory::make("cdparanoiasrc", None)?;
        cdparanoiasrc.set_property("track", &track)?;

        let queue = ElementFactory::make("queue", None)?;
        let audioconvert = ElementFactory::make("audioconvert", None)?;
        let flacenc = ElementFactory::make("flacenc", None)?;

        let filesink = gstreamer::ElementFactory::make("filesink", None)?;
        filesink.set_property("location", &format!("{}/track_{:02}.flac", path, track))?;

        let pipeline = gstreamer::Pipeline::new(None);
        pipeline.add_many(&[&cdparanoiasrc, &queue, &audioconvert, &flacenc, &filesink])?;

        Element::link_many(&[&cdparanoiasrc, &queue, &audioconvert, &flacenc, &filesink])?;

        Ok(pipeline)
    }
}
