use super::*;
use crate::backend::Backend;
use crate::database::*;
use gettextrs::gettext;
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
        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/person_list.ui");

        get_widget!(builder, gtk::Box, widget);
        get_widget!(builder, gtk::SearchEntry, search_entry);
        get_widget!(builder, gtk::Stack, stack);
        get_widget!(builder, gtk::ScrolledWindow, scrolled_window);

        let list = List::new(&gettext("No persons found."));

        list.set_make_widget(|person: &Person| {
            let label = gtk::Label::new(Some(&person.name_lf()));
            label.set_halign(gtk::Align::Start);
            label.set_margin_start(6);
            label.set_margin_end(6);
            label.set_margin_top(6);
            label.set_margin_bottom(6);
            label.upcast()
        });

        list.set_filter(clone!(@strong search_entry => move |person: &Person| {
            let search = search_entry.get_text().to_string().to_lowercase();
            let name = person.name_fl().to_lowercase();
            search.is_empty() || name.contains(&search)
        }));

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
            let persons = backend.db().get_persons().await.unwrap();
            list.show_items(persons);
            self.stack.set_visible_child_name("content");
        });
    }
}
