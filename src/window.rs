use super::database::*;
use super::dialogs::*;
use gio::prelude::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::{action, get_widget};
use std::cell::RefCell;
use std::rc::Rc;

pub struct Window {
    window: gtk::ApplicationWindow,
    db: Rc<Database>,
}

impl Window {
    pub fn new(app: &gtk::Application) -> Rc<Self> {
        let builder = gtk::Builder::from_resource("/de/johrpan/musicus_editor/ui/window.ui");
        get_widget!(builder, gtk::ApplicationWindow, window);

        let db = Rc::new(Database::new("test.sqlite"));

        let result = Rc::new(Window {
            window: window,
            db: db,
        });

        action!(
            result.window,
            "add-person",
            clone!(@strong result => move |_, _| {
                PersonEditor::new(result.db.clone(), &result.window, None, |person| {
                    println!("{:?}", person);
                }).show();
            })
        );

        action!(result.window, "add-instrument", |_, _| {
            println!("TODO: Add instrument.");
        });

        action!(result.window, "add-work", |_, _| {
            println!("TODO: Add work.");
        });

        action!(result.window, "add-ensemble", |_, _| {
            println!("TODO: Add ensemble.");
        });

        action!(result.window, "add-recording", |_, _| {
            println!("TODO: Add recording.");
        });

        result.window.set_application(Some(app));

        result
    }

    pub fn present(&self) {
        self.window.present();
    }
}
