use crate::backend::Backend;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use std::cell::RefCell;
use std::rc::Rc;

/// A dialog for setting up the server.
pub struct ServerDialog {
    backend: Rc<Backend>,
    window: libhandy::Window,
    url_entry: gtk::Entry,
    selected_cb: RefCell<Option<Box<dyn Fn(String) -> ()>>>,
}

impl ServerDialog {
    /// Create a new server dialog.
    pub fn new<P: IsA<gtk::Window>>(backend: Rc<Backend>, parent: &P) -> Rc<Self> {
        // Create UI
        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/server_dialog.ui");

        get_widget!(builder, libhandy::Window, window);
        get_widget!(builder, gtk::Button, cancel_button);
        get_widget!(builder, gtk::Button, set_button);
        get_widget!(builder, gtk::Entry, url_entry);

        window.set_transient_for(Some(parent));

        let this = Rc::new(Self {
            backend,
            window,
            url_entry,
            selected_cb: RefCell::new(None),
        });

        // Connect signals and callbacks

        cancel_button.connect_clicked(clone!(@strong this => move |_| {
            this.window.close();
        }));

        set_button.connect_clicked(clone!(@strong this => move |_| {
            let url = this.url_entry.get_text().to_string();
            this.backend.set_server_url(&url).unwrap();

            if let Some(cb) = &*this.selected_cb.borrow() {
                cb(url);
            }

            this.window.close();
        }));

        this
    }

    /// The closure to call when the server was set.
    pub fn set_selected_cb<F: Fn(String) -> () + 'static>(&self, cb: F) {
        self.selected_cb.replace(Some(Box::new(cb)));
    }

    /// Show the server dialog.
    pub fn show(&self) {
        self.window.show();
    }
}
