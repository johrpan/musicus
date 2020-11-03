use super::PersonEditor;
use crate::backend::Backend;
use crate::database::*;
use crate::widgets::*;
use gio::prelude::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use std::rc::Rc;

pub struct PersonSelector {
    window: libhandy::Window,
}

impl PersonSelector {
    pub fn new<P, F>(backend: Rc<Backend>, parent: &P, callback: F) -> Self
    where
        P: IsA<gtk::Window>,
        F: Fn(Person) -> () + 'static,
    {
        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/person_selector.ui");

        get_widget!(builder, libhandy::Window, window);
        get_widget!(builder, gtk::Box, vbox);
        get_widget!(builder, gtk::Button, add_button);

        let callback = Rc::new(callback);

        let list = PersonList::new(backend.clone());

        list.set_selected(clone!(@strong window, @strong callback => move |person| {
            window.close();
            callback(person.clone());
        }));

        vbox.pack_start(&list.widget, true, true, 0);
        window.set_transient_for(Some(parent));

        add_button.connect_clicked(
            clone!(@strong backend, @strong window, @strong callback => move |_| {
                let editor = PersonEditor::new(
                    backend.clone(),
                    &window,
                    None,
                    clone!(@strong window, @strong callback => move |person| {
                        window.close();
                        callback(person);
                    }),
                );

                editor.show();
            }),
        );

        Self { window }
    }

    pub fn show(&self) {
        self.window.show();
    }
}
