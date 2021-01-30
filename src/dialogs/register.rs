use crate::backend::{Backend, LoginData, UserRegistration};
use crate::widgets::{Navigator, NavigatorScreen};
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// A dialog for creating a new user account.
pub struct RegisterDialog {
    backend: Rc<Backend>,
    widget: gtk::Stack,
    username_entry: gtk::Entry,
    email_entry: gtk::Entry,
    password_entry: gtk::Entry,
    repeat_password_entry: gtk::Entry,
    captcha_row: libadwaita::ActionRow,
    captcha_entry: gtk::Entry,
    captcha_id: RefCell<Option<String>>,
    selected_cb: RefCell<Option<Box<dyn Fn(LoginData)>>>,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl RegisterDialog {
    /// Create a new register dialog.
    pub fn new(backend: Rc<Backend>) -> Rc<Self> {
        // Create UI
        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/register_dialog.ui");

        get_widget!(builder, gtk::Stack, widget);
        get_widget!(builder, gtk::Button, cancel_button);
        get_widget!(builder, gtk::Button, register_button);
        get_widget!(builder, gtk::Entry, username_entry);
        get_widget!(builder, gtk::Entry, email_entry);
        get_widget!(builder, gtk::Entry, password_entry);
        get_widget!(builder, gtk::Entry, repeat_password_entry);
        get_widget!(builder, libadwaita::ActionRow, captcha_row);
        get_widget!(builder, gtk::Entry, captcha_entry);

        let this = Rc::new(Self {
            backend,
            widget,
            username_entry,
            email_entry,
            password_entry,
            repeat_password_entry,
            captcha_row,
            captcha_entry,
            captcha_id: RefCell::new(None),
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

        register_button.connect_clicked(clone!(@strong this => move |_| {
            let password = this.password_entry.get_text().unwrap().to_string();
            let repeat = this.repeat_password_entry.get_text().unwrap().to_string();

            if (password != repeat) {
                // TODO: Show error and validate other input.
            } else {
                this.widget.set_visible_child_name("loading");

                let context = glib::MainContext::default();
                let clone = this.clone();
                context.spawn_local(async move {
                    let username = clone.username_entry.get_text().unwrap().to_string();
                    let email = clone.email_entry.get_text().unwrap().to_string();
                    let captcha_id = clone.captcha_id.borrow().clone().unwrap();
                    let answer = clone.captcha_entry.get_text().unwrap().to_string();

                    let email = if email.len() == 0 {
                        None
                    } else {
                        Some(email)
                    };

                    let registration = UserRegistration {
                        username: username.clone(),
                        password: password.clone(),
                        email,
                        captcha_id,
                        answer,
                    };

                    // TODO: Handle errors.
                    if clone.backend.register(registration).await.unwrap() {
                        if let Some(cb) = &*clone.selected_cb.borrow() {
                            let data = LoginData {
                                username,
                                password,
                            };

                            cb(data);
                        }

                        let navigator = clone.navigator.borrow().clone();
                        if let Some(navigator) = navigator {
                            navigator.pop();
                        }
                    } else {
                        clone.widget.set_visible_child_name("content");
                    }
                });
            }
        }));

        // Initialize

        let context = glib::MainContext::default();
        let clone = this.clone();
        context.spawn_local(async move {
            let captcha = clone.backend.get_captcha().await.unwrap();
            clone.captcha_row.set_title(Some(&captcha.question));
            clone.captcha_id.replace(Some(captcha.id));
            clone.widget.set_visible_child_name("content");
        });

        this
    }

    /// The closure to call when the login succeded.
    pub fn set_selected_cb<F: Fn(LoginData) + 'static>(&self, cb: F) {
        self.selected_cb.replace(Some(Box::new(cb)));
    }
}

impl NavigatorScreen for RegisterDialog {
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
