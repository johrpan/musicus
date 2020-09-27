use gio::prelude::*;
use gtk::prelude::*;
use gtk_macros::{action, get_widget};

pub struct Window {
    window: gtk::ApplicationWindow,
}

impl Window {
    pub fn new(app: &gtk::Application) -> Self {
        let builder = gtk::Builder::from_resource("/de/johrpan/musicus_editor/ui/window.ui");

        get_widget!(builder, gtk::ApplicationWindow, window);

        action!(window, "add-person", |_, _| {
            println!("TODO: Add person.");
        });

        action!(window, "add-instrument", |_, _| {
            println!("TODO: Add instrument.");
        });

        action!(window, "add-work", |_, _| {
            println!("TODO: Add work.");
        });

        action!(window, "add-ensemble", |_, _| {
            println!("TODO: Add ensemble.");
        });

        action!(window, "add-recording", |_, _| {
            println!("TODO: Add recording.");
        });

        window.set_application(Some(app));

        Window { window: window }
    }

    pub fn present(&self) {
        self.window.present();
    }
}
