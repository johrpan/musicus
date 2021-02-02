use super::RegisterDialog;
use crate::push;
use crate::backend::{Backend, LoginData};
use crate::widgets::new_navigator::{NavigationHandle, Screen, Widget};
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use std::cell::RefCell;
use std::rc::Rc;

/// A dialog for entering login credentials.
pub struct LoginDialog {
    handle: NavigationHandle<LoginData>,
    widget: gtk::Stack,
    info_bar: gtk::InfoBar,
    username_entry: gtk::Entry,
    password_entry: gtk::Entry,
}

impl Screen<(), LoginData> for LoginDialog {
    fn new(_: (), handle: NavigationHandle<LoginData>) -> Rc<Self> {
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

            let c = glib::MainContext::default();
            let clone = this.clone();
            c.spawn_local(async move {
                clone.handle.backend.set_login_data(data.clone()).await.unwrap();
                if clone.handle.backend.login().await.unwrap() {
                    clone.handle.pop(Some(data));
                } else {
                    clone.widget.set_visible_child_name("content");
                    clone.info_bar.set_revealed(true);
                }
            });
        }));

        register_button.connect_clicked(clone!(@weak this => move |_| {
            let context = glib::MainContext::default();
            let clone = this.clone();
            context.spawn_local(async move {
                if let Some(data) = push!(clone.handle, RegisterDialog).await {
                    clone.handle.pop(Some(data));
                }
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
