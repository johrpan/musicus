mod album_tile;
mod application;
mod config;
mod db;
mod editor;
mod home_page;
mod library;
mod library_manager;
mod player;
mod player_bar;
mod playlist_item;
mod playlist_page;
mod playlist_tile;
mod program;
mod program_tile;
mod recording_tile;
mod search_entry;
mod search_tag;
mod tag_tile;
mod util;
mod welcome_page;
mod window;

use self::{application::MusicusApplication, window::MusicusWindow};
use gettextrs::LocaleCategory;
use gstreamer_play::gst;
use gtk::{gio, glib, prelude::*};

fn main() -> glib::ExitCode {
    tracing_subscriber::fmt::init();
    gtk::init().expect("Failed to initialize GTK!");
    gst::init().expect("Failed to initialize GStreamer!");

    glib::set_application_name(config::NAME);
    gtk::Window::set_default_icon_name(config::APP_ID);

    gettextrs::setlocale(LocaleCategory::LcAll, "");
    gettextrs::bindtextdomain(config::PKGNAME, config::LOCALEDIR).unwrap();
    gettextrs::textdomain(config::PKGNAME).unwrap();

    gio::resources_register(
        &gio::Resource::load(&format!(
            "{}/{}/{}.gresource",
            config::DATADIR,
            config::PKGNAME,
            config::APP_ID
        ))
        .expect("Could not load resources"),
    );

    MusicusApplication::new().run()
}
