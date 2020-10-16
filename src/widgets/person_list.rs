use super::*;
use crate::backend::Backend;
use crate::database::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use std::rc::Rc;

pub struct PersonList {
    pub widget: gtk::Box,
    list: Rc<List<Person>>,
    backend: Rc<Backend>,
    stack: gtk::Stack,
}

impl PersonList {
    pub fn new(backend: Rc<Backend>) -> Rc<Self> {
        let builder = gtk::Builder::from_resource("/de/johrpan/musicus_editor/ui/person_list.ui");

        get_widget!(builder, gtk::Box, widget);
        get_widget!(builder, gtk::SearchEntry, search_entry);
        get_widget!(builder, gtk::Stack, stack);
        get_widget!(builder, gtk::ScrolledWindow, scrolled_window);

        let list = List::new(
            |person: &Person| {
                let label = gtk::Label::new(Some(&person.name_lf()));
                label.set_halign(gtk::Align::Start);
                label.upcast()
            },
            clone!(@strong search_entry => move |person: &Person| {
                let search = search_entry.get_text().to_string().to_lowercase();
                let name = person.name_fl().to_lowercase();
                search.is_empty() || name.contains(&search)
            }),
            "No persons found.",
        );

        scrolled_window.add(&list.widget);

        let result = Rc::new(Self {
            widget,
            list,
            backend,
            stack,
        });

        search_entry.connect_search_changed(clone!(@strong result => move |_| {
            result.list.invalidate_filter();
        }));

        result.clone().reload();

        result
    }

    pub fn set_selected<S>(&self, selected: S)
    where
        S: Fn(&Person) -> () + 'static,
    {
        self.list.set_selected(selected);
    }

    pub fn reload(self: Rc<Self>) {
        self.stack.set_visible_child_name("loading");

        let context = glib::MainContext::default();
        let backend = self.backend.clone();
        let list = self.list.clone();

        context.spawn_local(async move {
            let persons = backend.get_persons().await.unwrap();
            list.show_items(persons);
            self.stack.set_visible_child_name("content");
        });
    }
}
