mod application;
mod config;
mod welcome_page;
mod window;

use self::{application::MusicusApplication, window::MusicusWindow};

use config::{GETTEXT_PACKAGE, LOCALEDIR, PKGDATADIR};
use gettextrs::{bind_textdomain_codeset, bindtextdomain, textdomain};
use gtk::{gio, glib, prelude::*};

fn main() -> glib::ExitCode {
    bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR).expect("Unable to bind the text domain");
    bind_textdomain_codeset(GETTEXT_PACKAGE, "UTF-8")
        .expect("Unable to set the text domain encoding");
    textdomain(GETTEXT_PACKAGE).expect("Unable to switch to the text domain");

    let resources = gio::Resource::load(PKGDATADIR.to_owned() + "/musicus.gresource")
        .expect("Could not load resources");
    gio::resources_register(&resources);

    MusicusApplication::new("de.johrpan.musicus", &gio::ApplicationFlags::empty()).run()
}
