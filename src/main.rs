mod album_page;
mod album_tile;
mod application;
mod config;
mod editor;
mod empty_page;
mod entity_browser;
mod facet_tile;
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
mod program_section;
mod program_tile;
mod recording_tile;
mod search_page;
mod selector;
mod slider_row;
mod util;
mod welcome_page;
mod window;
mod work_page;

use gettextrs::LocaleCategory;
use gstreamer_play::gst;
use gtk::{gio, glib, prelude::*};

use self::{application::Application, window::Window};

fn main() -> glib::ExitCode {
    // SAFETY: `gettextrs::setlocale()` is called as early as possible, prior
    // to starting any more threads.
    unsafe { gettextrs::setlocale(LocaleCategory::LcAll, "") };
    gettextrs::bindtextdomain(config::PKGNAME, locale_dir()).unwrap();
    gettextrs::textdomain(config::PKGNAME).unwrap();

    tracing_subscriber::fmt::init();

    gtk::init().expect("Failed to initialize GTK!");
    gst::init().expect("Failed to initialize GStreamer!");

    musicus_library::db::set_language(&*util::LANG);

    glib::set_application_name(config::NAME);
    gtk::Window::set_default_icon_name(config::APP_ID);

    gio::resources_register(
        &gio::Resource::load(format!(
            "{}/{}/{}.gresource",
            data_dir(),
            config::PKGNAME,
            config::APP_ID
        ))
        .expect("Could not load resources"),
    );

    Application::new().run()
}

/// Data directory, dependent on current platform.
fn data_dir() -> String {
    #[cfg(windows)]
    if let Some(dir) = exe_relative_share_dir() {
        return dir.to_string_lossy().into_owned();
    }

    config::DATADIR.to_string()
}

/// Locale directory, dependent on current platform.
fn locale_dir() -> String {
    #[cfg(windows)]
    if let Some(dir) = exe_relative_share_dir() {
        return dir.join("locale").to_string_lossy().into_owned();
    }

    config::LOCALEDIR.to_string()
}

/// Find the shared files directory relative to the EXE on windows.
#[cfg(windows)]
fn exe_relative_share_dir() -> Option<std::path::PathBuf> {
    Some(std::env::current_exe().ok()?.parent()?.parent()?.join("share"))
}
