use super::*;
use crate::backend::Backend;
use crate::database::*;
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
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
    list: Rc<List<PersonOrEnsemble>>,
    backend: Rc<Backend>,
    stack: gtk::Stack,
}

impl PoeList {
    pub fn new(backend: Rc<Backend>) -> Rc<Self> {
        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/poe_list.ui");

        get_widget!(builder, gtk::Box, widget);
        get_widget!(builder, gtk::SearchEntry, search_entry);
        get_widget!(builder, gtk::Stack, stack);
        get_widget!(builder, gtk::ScrolledWindow, scrolled_window);

        let list = List::new(
            |poe: &PersonOrEnsemble| {
                let label = gtk::Label::new(Some(&poe.get_title()));
                label.set_halign(gtk::Align::Start);
                label.upcast()
            },
            clone!(@strong search_entry => move |poe: &PersonOrEnsemble| {
                let search = search_entry.get_text().to_string().to_lowercase();
                let title = poe.get_title().to_lowercase();
                search.is_empty() || title.contains(&search)
            }),
            &gettext("No persons or ensembles found."),
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

        result
    }

    pub fn set_selected<S>(&self, selected: S)
    where
        S: Fn(&PersonOrEnsemble) -> () + 'static,
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
            let ensembles = backend.get_ensembles().await.unwrap();
            let mut poes: Vec<PersonOrEnsemble> = Vec::new();

            for person in persons {
                poes.push(PersonOrEnsemble::Person(person));
            }

            for ensemble in ensembles {
                poes.push(PersonOrEnsemble::Ensemble(ensemble));
            }

            list.show_items(poes);

            self.stack.set_visible_child_name("content");
        });
    }
}
