use crate::error::{Error, Result};
use crate::session::{ImportSession, ImportTrack, State};
use base64::Engine;
use gstreamer::prelude::*;
use gstreamer::tags::{Duration, TrackNumber};
use gstreamer::{ClockTime, ElementFactory, MessageType, MessageView, TocEntryType};
use log::info;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tokio::sync::watch;

/// Create a new import session for the default disc drive.
pub(super) fn new() -> Result<ImportSession> {
    let (state_sender, state_receiver) = watch::channel(State::Waiting);

    let mut tracks = Vec::new();
    let mut hasher = Sha256::new();

    // Build the GStreamer pipeline. It will contain a fakesink initially to be able to run it
    // forward to the paused state without specifying a file name before knowing the tracks.

    let cdparanoiasrc = ElementFactory::make("cdparanoiasrc").build()?;
    let queue = ElementFactory::make("queue").build()?;
    let audioconvert = ElementFactory::make("audioconvert").build()?;
    let flacenc = ElementFactory::make("flacenc").build()?;
    let fakesink = gstreamer::ElementFactory::make("fakesink").build()?;

    let pipeline = gstreamer::Pipeline::new(None);
    pipeline.add_many(&[&cdparanoiasrc, &queue, &audioconvert, &flacenc, &fakesink])?;
    gstreamer::Element::link_many(&[&cdparanoiasrc, &queue, &audioconvert, &flacenc, &fakesink])?;

    let bus = pipeline
        .bus()
        .ok_or_else(|| Error::u(String::from("Failed to get bus from pipeline.")))?;

    // Run the pipeline into the paused state and wait for the resulting TOC message on the bus.

    pipeline.set_state(gstreamer::State::Paused)?;

    let msg = bus.timed_pop_filtered(
        ClockTime::from_seconds(5),
        &[MessageType::Toc, MessageType::Error],
    );

    let toc = match msg {
        Some(msg) => match msg.view() {
            MessageView::Error(err) => Err(Error::os(err.error())),
            MessageView::Toc(toc) => Ok(toc.toc().0),
            _ => Err(Error::u(format!(
                "Unexpected message from GStreamer: {:?}",
                msg
            ))),
        },
        None => Err(Error::Timeout(
            "Timeout while waiting for first message from GStreamer.".to_string(),
        )),
    }?;

    pipeline.set_state(gstreamer::State::Ready)?;

    // Replace the fakesink with the real filesink. This won't need to be synced to the pipeline
    // state, because we will set the whole pipeline's state to playing later.

    gstreamer::Element::unlink(&flacenc, &fakesink);
    fakesink.set_state(gstreamer::State::Null)?;
    pipeline.remove(&fakesink)?;

    let filesink = gstreamer::ElementFactory::make("filesink").build()?;
    pipeline.add(&filesink)?;
    gstreamer::Element::link(&flacenc, &filesink)?;

    // Get track data from the toc message that was received above.

    let tmp_dir = create_tmp_dir()?;

    for entry in toc.entries() {
        if entry.entry_type() == TocEntryType::Track {
            let duration = entry
                .tags()
                .ok_or_else(|| Error::u(String::from("No tags in TOC entry.")))?
                .get::<Duration>()
                .ok_or_else(|| Error::u(String::from("No duration tag found in TOC entry.")))?
                .get()
                .mseconds();

            let number = entry
                .tags()
                .ok_or_else(|| Error::u(String::from("No tags in TOC entry.")))?
                .get::<TrackNumber>()
                .ok_or_else(|| Error::u(String::from("No track number tag found in TOC entry.")))?
                .get();

            hasher.update(duration.to_le_bytes());

            let name = format!("Track {}", number);

            let file_name = format!("track_{:02}.flac", number);
            let mut path = tmp_dir.clone();
            path.push(file_name);

            let track = ImportTrack {
                number,
                name,
                path,
                duration,
            };

            tracks.push(track);
        }
    }

    let source_id = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hasher.finalize());

    info!("Successfully loaded audio CD with {} tracks.", tracks.len());
    info!("Source ID: {}", source_id);

    let tracks_clone = tracks.clone();
    let copy = move || {
        for track in &tracks_clone {
            info!("Starting to rip track {}.", track.number);

            cdparanoiasrc.set_property("track", &track.number);

            // The filesink needs to be reset to be able to change the file location.
            filesink.set_state(gstreamer::State::Null)?;

            let path = track.path.to_str().unwrap();
            filesink.set_property("location", &path);

            // This will also affect the filesink as expected.
            pipeline.set_state(gstreamer::State::Playing)?;

            for msg in bus.iter_timed(None) {
                match msg.view() {
                    MessageView::Eos(..) => {
                        info!("Finished ripping track {}.", track.number);
                        pipeline.set_state(gstreamer::State::Ready)?;
                        break;
                    }
                    MessageView::Error(err) => {
                        pipeline.set_state(gstreamer::State::Null)?;
                        return Err(Error::os(err.error()));
                    }
                    _ => (),
                }
            }
        }

        pipeline.set_state(gstreamer::State::Null)?;

        Ok(())
    };

    let session = ImportSession {
        source_id,
        tracks,
        copy: Some(Box::new(copy)),
        state_sender,
        state_receiver,
    };

    Ok(session)
}

/// Create a new temporary directory and return its path.
fn create_tmp_dir() -> Result<PathBuf> {
    let mut tmp_dir = glib::tmp_dir();

    let dir_name = format!("musicus-{}", rand::random::<u64>());
    tmp_dir.push(dir_name);

    std::fs::create_dir(&tmp_dir)?;

    Ok(tmp_dir)
}
