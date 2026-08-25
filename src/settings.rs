use gtk::{gio, gio::prelude::*};
use musicus_library::library::Patterns;

use crate::config;

pub fn patterns() -> Patterns {
    let settings = settings();

    Patterns {
        filename: settings.string("track-filename-pattern").into(),
        album: settings.string("track-tag-album-pattern").into(),
        artist: settings.string("track-tag-artist-pattern").into(),
        title: settings.string("track-tag-title-pattern").into(),
    }
}

fn settings() -> gio::Settings {
    gio::Settings::new(config::APP_ID)
}
