use crate::backend::{Backend, LoginData, UserRegistration};
use crate::widgets::new_navigator::{NavigationHandle, Screen, Widget};
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// A dialog for creating a new user account.
pub struct RegisterDialog {
    handle: NavigationHandle<LoginData>,
    widget: gtk::Stack,
    username_entry: gtk::Entry,
    email_entry: gtk::Entry,
    password_entry: gtk::Entry,
    repeat_password_entry: gtk::Entry,
    captcha_row: libadwaita::ActionRow,
    captcha_entry: gtk::Entry,
    captcha_id: RefCell<Option<String>>,
}

impl Screen<(), LoginData> for RegisterDialog {
    /// Create a new register dialog.
    fn new(_: (), handle: NavigationHandle<LoginData>) -> Rc<Self> {
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
            handle,
            widget,
            username_entry,
            email_entry,
            password_entry,
            repeat_password_entry,
            captcha_row,
            captcha_entry,
            captcha_id: RefCell::new(None),
        });

        // Connect signals and callbacks

        cancel_button.connect_clicked(clone!(@weak this => move |_| {
            this.handle.pop(None);
        }));

        register_button.connect_clicked(clone!(@weak this => move |_| {
            let password = this.password_entry.get_text().unwrap().to_string();
            let repeat = this.repeat_password_entry.get_text().unwrap().to_string();

            if (password != repeat) {
                // TODO: Show error and validate other input.
            } else {
                this.widget.set_visible_child_name("loading");

                spawn!(@clone this, async move {
                    let username = this.username_entry.get_text().unwrap().to_string();
                    let email = this.email_entry.get_text().unwrap().to_string();
                    let captcha_id = this.captcha_id.borrow().clone().unwrap();
                    let answer = this.captcha_entry.get_text().unwrap().to_string();

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
                    if this.handle.backend.register(registration).await.unwrap() {
                        let data = LoginData {
                            username,
                            password,
                        };

                        this.handle.pop(Some(data));
                    } else {
                        this.widget.set_visible_child_name("content");
                    }
                });
            }
        }));

        // Initialize

        spawn!(@clone this, async move {
            let captcha = this.handle.backend.get_captcha().await.unwrap();
            this.captcha_row.set_title(Some(&captcha.question));
            this.captcha_id.replace(Some(captcha.id));
            this.widget.set_visible_child_name("content");
        });

        this
    }
}

impl Widget for RegisterDialog {
    fn get_widget(&self) -> gtk::Widget {
        self.widget.clone().upcast()
    }
}
