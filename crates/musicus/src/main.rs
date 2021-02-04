use gio::prelude::*;
use glib::clone;
use std::cell::RefCell;
use std::rc::Rc;

#[macro_use]
mod macros;

mod config;
mod editors;
mod import;
mod navigator;
mod preferences;
mod screens;
mod selectors;
mod widgets;

mod window;
use window::Window;

mod resources;

fn main() {
    gettextrs::setlocale(gettextrs::LocaleCategory::LcAll, "");
    gettextrs::bindtextdomain("musicus", config::LOCALEDIR);
    gettextrs::textdomain("musicus");

    gstreamer::init().expect("Failed to initialize GStreamer!");
    gtk::init().expect("Failed to initialize GTK!");
    libadwaita::init();
    resources::init().expect("Failed to initialize resources!");

    let app = gtk::Application::new(Some("de.johrpan.musicus"), gio::ApplicationFlags::empty())
        .expect("Failed to initialize GTK application!");

    let window: RefCell<Option<Rc<Window>>> = RefCell::new(None);

    app.connect_activate(clone!(@strong app => move |_| {
        let mut window = window.borrow_mut();
        if window.is_none() {
            window.replace(Window::new(&app));
        }
        window.as_ref().unwrap().present();
    }));

    let args = std::env::args().collect::<Vec<String>>();
    app.run(&args);
}
