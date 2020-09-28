use super::database::*;
use super::dialogs::*;
use gio::prelude::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::{action, get_widget};
use libhandy::prelude::*;
use libhandy::HeaderBarExt;
use std::cell::RefCell;
use std::convert::TryInto;
use std::rc::Rc;

pub struct Window {
    window: libhandy::ApplicationWindow,
    db: Rc<Database>,
    leaflet: libhandy::Leaflet,
    persons: RefCell<Vec<Person>>,
    person_search_entry: gtk::SearchEntry,
    person_list: gtk::ListBox,
    header_stack: gtk::Stack,
    header: libhandy::HeaderBar,
    content_box: gtk::Box,
    content: gtk::ScrolledWindow,
}

impl Window {
    pub fn new(app: &gtk::Application) -> Rc<Self> {
        let builder = gtk::Builder::from_resource("/de/johrpan/musicus_editor/ui/window.ui");

        get_widget!(builder, libhandy::ApplicationWindow, window);
        get_widget!(builder, libhandy::Leaflet, leaflet);
        get_widget!(builder, gtk::SearchEntry, person_search_entry);
        get_widget!(builder, gtk::ListBox, person_list);
        get_widget!(builder, gtk::Stack, header_stack);
        get_widget!(builder, libhandy::HeaderBar, header);
        get_widget!(builder, gtk::Box, content_box);
        get_widget!(builder, gtk::ScrolledWindow, content);

        let db = Rc::new(Database::new("test.sqlite"));
        let persons = db.get_persons();

        let result = Rc::new(Window {
            window: window,
            db: db,
            leaflet: leaflet,
            persons: RefCell::new(persons),
            person_list: person_list,
            person_search_entry: person_search_entry,
            header_stack: header_stack,
            header: header,
            content_box: content_box,
            content: content,
        });

        result
            .person_list
            .connect_row_activated(clone!(@strong result => move |_, row| {
                let row = row.get_child().unwrap().downcast::<SelectorRow>().unwrap();
                let index: usize = row.get_index().try_into().unwrap();
                let person = result.persons.borrow()[index].clone();
                result.show_person(person);
            }));

        result
            .person_list
            .set_filter_func(Some(Box::new(clone!(@strong result => move |row| {
                let row = row.get_child().unwrap().downcast::<SelectorRow>().unwrap();
                let index: usize = row.get_index().try_into().unwrap();
                let search = result.person_search_entry.get_text().to_string().to_lowercase();

                search.is_empty() || result.persons.borrow()[index]
                    .name_lf()
                    .to_lowercase()
                    .contains(&search)
            }))));

        result
            .person_search_entry
            .connect_search_changed(clone!(@strong result => move |_| {
                result.person_list.invalidate_filter();
            }));

        action!(
            result.window,
            "add-person",
            clone!(@strong result => move |_, _| {
                PersonEditor::new(result.db.clone(), &result.window, None, clone!(@strong result => move |_| {
                    result.persons.replace(result.db.get_persons());
                    result.show_persons();
                })).show();
            })
        );

        action!(
            result.window,
            "add-instrument",
            clone!(@strong result => move |_, _| {
                InstrumentEditor::new(result.db.clone(), &result.window, None, |instrument| {
                    println!("{:?}", instrument);
                }).show();
            })
        );

        action!(
            result.window,
            "add-work",
            clone!(@strong result => move |_, _| {
                WorkEditor::new(result.db.clone(), &result.window, None, clone!(@strong result => move |_| {
                    result.persons.replace(result.db.get_persons());
                    result.show_persons();
                })).show();
            })
        );

        action!(
            result.window,
            "add-ensemble",
            clone!(@strong result => move |_, _| {
                EnsembleEditor::new(result.db.clone(), &result.window, None, |ensemble| {
                    println!("{:?}", ensemble);
                }).show();
            })
        );

        action!(result.window, "add-recording", |_, _| {
            println!("TODO: Add recording.");
        });

        result.window.set_application(Some(app));
        result.show_persons();

        result
    }

    pub fn present(&self) {
        self.window.present();
    }

    fn show_persons(&self) {
        for child in self.person_list.get_children() {
            self.person_list.remove(&child);
        }

        for (index, person) in self.persons.borrow().iter().enumerate() {
            let label = gtk::Label::new(Some(&person.name_lf()));
            label.set_halign(gtk::Align::Start);
            let row = SelectorRow::new(index.try_into().unwrap(), &label);
            row.show_all();
            self.person_list.insert(&row, -1);
        }
    }

    fn show_person(&self, person: Person) {
        self.header.set_title(Some(&person.name_fl()));
        self.header_stack.set_visible_child_name("header");
        self.set_view(&gtk::Label::new(Some(&person.name_fl())));
    }

    fn set_view<T: IsA<gtk::Widget>>(&self, widget: &T) {
        match self.content.get_child() {
            Some(child) => self.content.remove(&child),
            None => (),
        }

        self.content.add(widget);
        self.leaflet.set_visible_child(&self.content_box);
    }
}
