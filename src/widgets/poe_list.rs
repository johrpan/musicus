use super::*;
use crate::backend::Backend;
use crate::database::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use libhandy::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone)]
pub enum PersonOrEnsemble {
    Person(Person),
    Ensemble(Ensemble),
}

impl PersonOrEnsemble {
    pub fn get_title(&self) -> String {
        match self {
            PersonOrEnsemble::Person(person) => person.name_lf(),
            PersonOrEnsemble::Ensemble(ensemble) => ensemble.name.clone(),
        }
    }
}

pub struct PoeList {
    pub widget: gtk::Box,
    backend: Rc<Backend>,
    stack: gtk::Stack,
    search_entry: gtk::SearchEntry,
    list: Rc<List>,
    data: RefCell<Vec<PersonOrEnsemble>>,
    selected_cb: RefCell<Option<Box<dyn Fn(&PersonOrEnsemble)>>>,
}

impl PoeList {
    pub fn new(backend: Rc<Backend>) -> Rc<Self> {
        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/poe_list.ui");

        get_widget!(builder, gtk::Box, widget);
        get_widget!(builder, gtk::SearchEntry, search_entry);
        get_widget!(builder, gtk::Stack, stack);
        get_widget!(builder, gtk::ScrolledWindow, scrolled_window);

        let list = List::new();
        list.widget.add_css_class("navigation-sidebar");

        scrolled_window.set_child(Some(&list.widget));

        let this = Rc::new(Self {
            widget,
            backend,
            stack,
            search_entry,
            list,
            data: RefCell::new(Vec::new()),
            selected_cb: RefCell::new(None),
        });

        this.search_entry.connect_search_changed(clone!(@strong this => move |_| {
            this.list.invalidate_filter();
        }));

        this.list.set_make_widget_cb(clone!(@strong this => move |index| {
            let poe = &this.data.borrow()[index];

            let row = libhandy::ActionRow::new();
            row.set_activatable(true);
            row.set_title(Some(&poe.get_title()));

            let poe = poe.to_owned();
            row.connect_activated(clone!(@strong this => move |_| {
                if let Some(cb) = &*this.selected_cb.borrow() {
                    cb(&poe);
                }
            }));

            row.upcast()
        }));

        this.list.set_filter_cb(clone!(@strong this => move |index| {
            let poe = &this.data.borrow()[index];
            let search = this.search_entry.get_text().unwrap().to_string().to_lowercase();
            let title = poe.get_title().to_lowercase();
            search.is_empty() || title.contains(&search)
        }));

        this
    }

    pub fn set_selected_cb<F: Fn(&PersonOrEnsemble) + 'static>(&self, cb: F) {
        self.selected_cb.replace(Some(Box::new(cb)));
    }

    pub fn reload(self: Rc<Self>) {
        self.stack.set_visible_child_name("loading");

        let context = glib::MainContext::default();
        let backend = self.backend.clone();

        context.spawn_local(async move {
            let persons = backend.db().get_persons().await.unwrap();
            let ensembles = backend.db().get_ensembles().await.unwrap();
            let mut poes: Vec<PersonOrEnsemble> = Vec::new();

            for person in persons {
                poes.push(PersonOrEnsemble::Person(person));
            }

            for ensemble in ensembles {
                poes.push(PersonOrEnsemble::Ensemble(ensemble));
            }

            let length = poes.len();
            self.data.replace(poes);
            self.list.update(length);

            self.stack.set_visible_child_name("content");
        });
    }
}
