// Required for database/schema.rs
#[macro_use]
extern crate diesel;

// Required for embed_migrations macro in database/database.rs
#[macro_use]
extern crate diesel_migrations;

use gio::prelude::*;
use glib::clone;
use std::cell::RefCell;

mod database;

mod window;
use window::Window;

fn main() {
    let bytes = glib::Bytes::from(include_bytes!("../res/resources.gresource").as_ref());
    let resource = gio::Resource::from_data(&bytes).expect("Failed to load resources!");
    gio::resources_register(&resource);

    let app = gtk::Application::new(
        Some("de.johrpan.musicus_desktop"),
        gio::ApplicationFlags::empty(),
    )
    .expect("Failed to initialize GTK application!");

    let window = RefCell::new(None::<Window>);

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
