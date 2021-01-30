use super::RegisterDialog;
use crate::backend::{Backend, LoginData};
use crate::widgets::{Navigator, NavigatorScreen};
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use std::cell::RefCell;
use std::rc::Rc;

/// A dialog for entering login credentials.
pub struct LoginDialog {
    backend: Rc<Backend>,
    widget: gtk::Stack,
    info_bar: gtk::InfoBar,
    username_entry: gtk::Entry,
    password_entry: gtk::Entry,
    selected_cb: RefCell<Option<Box<dyn Fn(LoginData) -> ()>>>,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl LoginDialog {
    /// Create a new login dialog.
    pub fn new(backend: Rc<Backend>) -> Rc<Self> {
        // Create UI
        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/login_dialog.ui");

        get_widget!(builder, gtk::Stack, widget);
        get_widget!(builder, gtk::InfoBar, info_bar);
        get_widget!(builder, gtk::Button, cancel_button);
        get_widget!(builder, gtk::Button, login_button);
        get_widget!(builder, gtk::Entry, username_entry);
        get_widget!(builder, gtk::Entry, password_entry);
        get_widget!(builder, gtk::Button, register_button);

        let this = Rc::new(Self {
            backend,
            widget,
            info_bar,
            username_entry,
            password_entry,
            selected_cb: RefCell::new(None),
            navigator: RefCell::new(None),
        });

        // Connect signals and callbacks

        cancel_button.connect_clicked(clone!(@strong this => move |_| {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                navigator.pop();
            }
        }));

        login_button.connect_clicked(clone!(@strong this => move |_| {
            this.widget.set_visible_child_name("loading");

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

                    let navigator = clone.navigator.borrow().clone();
                    if let Some(navigator) = navigator {
                        navigator.pop();
                    }
                } else {
                    clone.widget.set_visible_child_name("content");
                    clone.info_bar.set_revealed(true);
                }
            });
        }));

        register_button.connect_clicked(clone!(@strong this => move |_| {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                let dialog = RegisterDialog::new(this.backend.clone());

                dialog.set_selected_cb(clone!(@strong this => move |data| {
                    if let Some(cb) = &*this.selected_cb.borrow() {
                        cb(data);
                    }

                    let navigator = this.navigator.borrow().clone();
                    if let Some(navigator) = navigator {
                        navigator.pop();
                    }
                }));

                navigator.push(dialog);
            }
        }));

        this
    }

    /// The closure to call when the login succeded.
    pub fn set_selected_cb<F: Fn(LoginData) -> () + 'static>(&self, cb: F) {
        self.selected_cb.replace(Some(Box::new(cb)));
    }
}

impl NavigatorScreen for LoginDialog {
    fn attach_navigator(&self, navigator: Rc<Navigator>) {
        self.navigator.replace(Some(navigator));
    }

    fn get_widget(&self) -> gtk::Widget {
        self.widget.clone().upcast()
    }

    fn detach_navigator(&self) {
        self.navigator.replace(None);
    }
}
