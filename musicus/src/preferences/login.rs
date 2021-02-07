use super::register::RegisterDialog;
use crate::push;
use crate::navigator::{NavigationHandle, Screen};
use crate::widgets::Widget;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use musicus_backend::client::LoginData;
use std::rc::Rc;

/// A dialog for entering login credentials.
pub struct LoginDialog {
    handle: NavigationHandle<Option<LoginData>>,
    widget: gtk::Stack,
    info_bar: gtk::InfoBar,
    username_entry: gtk::Entry,
    password_entry: gtk::Entry,
}

impl Screen<Option<LoginData>, Option<LoginData>> for LoginDialog {
    fn new(data: Option<LoginData>, handle: NavigationHandle<Option<LoginData>>) -> Rc<Self> {
        // Create UI
        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/login_dialog.ui");

        get_widget!(builder, gtk::Stack, widget);
        get_widget!(builder, gtk::InfoBar, info_bar);
        get_widget!(builder, gtk::Button, cancel_button);
        get_widget!(builder, gtk::Button, login_button);
        get_widget!(builder, gtk::Entry, username_entry);
        get_widget!(builder, gtk::Entry, password_entry);
        get_widget!(builder, gtk::Box, register_box);
        get_widget!(builder, gtk::Button, register_button);
        get_widget!(builder, gtk::Box, logout_box);
        get_widget!(builder, gtk::Button, logout_button);

        if let Some(data) = data {
            username_entry.set_text(&data.username);
            register_box.hide();
            logout_box.show();
        }

        let this = Rc::new(Self {
            handle,
            widget,
            info_bar,
            username_entry,
            password_entry,
        });

        // Connect signals and callbacks

        cancel_button.connect_clicked(clone!(@weak this => move |_| {
            this.handle.pop(None);
        }));

        login_button.connect_clicked(clone!(@weak this => move |_| {
            this.widget.set_visible_child_name("loading");

            let data = LoginData {
                username: this.username_entry.get_text().unwrap().to_string(),
                password: this.password_entry.get_text().unwrap().to_string(),
            };

            spawn!(@clone this, async move {
                this.handle.backend.set_login_data(Some(data.clone())).await;
                if this.handle.backend.cl().login().await.unwrap() {
                    this.handle.pop(Some(Some(data)));
                } else {
                    this.widget.set_visible_child_name("content");
                    this.info_bar.set_revealed(true);
                }
            });
        }));

        register_button.connect_clicked(clone!(@weak this => move |_| {
            spawn!(@clone this, async move {
                if let Some(data) = push!(this.handle, RegisterDialog).await {
                    this.handle.pop(Some(Some(data)));
                }
            });
        }));

        logout_button.connect_clicked(clone!(@weak this => move |_| {
            spawn!(@clone this, async move {
                this.handle.backend.set_login_data(None).await;
                this.handle.pop(Some(None));
            });
        }));

        this
    }
}

impl Widget for LoginDialog {
    fn get_widget(&self) -> gtk::Widget {
        self.widget.clone().upcast()
    }
}
