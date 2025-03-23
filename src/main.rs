mod album_page;
mod album_tile;
mod application;
mod config;
mod db;
mod editor;
mod empty_page;
mod library;
mod library_manager;
mod player;
mod player_bar;
mod playlist_item;
mod playlist_page;
mod playlist_tile;
mod preferences_dialog;
mod process;
mod process_manager;
mod process_row;
mod program;
mod program_tile;
mod recording_tile;
mod search_page;
mod search_tag;
mod selector;
mod slider_row;
mod tag_tile;
mod util;
mod welcome_page;
mod window;

use gettextrs::LocaleCategory;
use gstreamer_play::gst;
use gtk::{gio, glib, prelude::*};

use self::{application::Application, window::Window};

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

    Application::new().run()
}
