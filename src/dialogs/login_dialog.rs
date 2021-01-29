use crate::backend::{Backend, LoginData};
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use std::cell::RefCell;
use std::rc::Rc;

/// A dialog for entering login credentials.
pub struct LoginDialog {
    backend: Rc<Backend>,
    window: libadwaita::Window,
    stack: gtk::Stack,
    info_bar: gtk::InfoBar,
    username_entry: gtk::Entry,
    password_entry: gtk::Entry,
    selected_cb: RefCell<Option<Box<dyn Fn(LoginData) -> ()>>>,
}

impl LoginDialog {
    /// Create a new login dialog.
    pub fn new<P: IsA<gtk::Window>>(backend: Rc<Backend>, parent: &P) -> Rc<Self> {
        // Create UI
        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/login_dialog.ui");

        get_widget!(builder, libadwaita::Window, window);
        get_widget!(builder, gtk::Stack, stack);
        get_widget!(builder, gtk::InfoBar, info_bar);
        get_widget!(builder, gtk::Button, cancel_button);
        get_widget!(builder, gtk::Button, login_button);
        get_widget!(builder, gtk::Entry, username_entry);
        get_widget!(builder, gtk::Entry, password_entry);

        window.set_transient_for(Some(parent));

        let this = Rc::new(Self {
            backend,
            window,
            stack,
            info_bar,
            username_entry,
            password_entry,
            selected_cb: RefCell::new(None),
        });

        // Connect signals and callbacks

        cancel_button.connect_clicked(clone!(@strong this => move |_| {
            this.window.close();
        }));

        login_button.connect_clicked(clone!(@strong this => move |_| {
            this.stack.set_visible_child_name("loading");

            let data = LoginData {
                username: this.username_entry.get_text().unwrap().to_string(),
                password: this.password_entry.get_text().unwrap().to_string(),
            };

            let c = glib::MainContext::default();
            let clone = this.clone();
            c.spawn_local(async move {
                clone.backend.set_login_data(data.clone()).await.unwrap();
                if clone.backend.login().await.unwrap() {
                    if let Some(cb) = &*clone.selected_cb.borrow() {
                        cb(data);
                    }

                    clone.window.close();
                } else {
                    clone.stack.set_visible_child_name("content");
                    clone.info_bar.set_revealed(true);
                }
            });
        }));

        this
    }

    /// The closure to call when the login succeded.
    pub fn set_selected_cb<F: Fn(LoginData) -> () + 'static>(&self, cb: F) {
        self.selected_cb.replace(Some(Box::new(cb)));
    }

    /// Show the login dialog.
    pub fn show(&self) {
        self.window.show();
    }
}
