use super::database::*;
use super::dialogs::*;
use gio::prelude::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::{action, get_widget};
use std::cell::RefCell;
use std::convert::TryInto;
use std::rc::Rc;

pub struct Window {
    window: gtk::ApplicationWindow,
    db: Rc<Database>,
    persons: RefCell<Vec<Person>>,
    person_search_entry: gtk::SearchEntry,
    person_list: gtk::ListBox,
    works: RefCell<Vec<WorkDescription>>,
    work_search_entry: gtk::SearchEntry,
    work_list: gtk::ListBox,
}

impl Window {
    pub fn new(app: &gtk::Application) -> Rc<Self> {
        let builder = gtk::Builder::from_resource("/de/johrpan/musicus_editor/ui/window.ui");
        get_widget!(builder, gtk::ApplicationWindow, window);
        get_widget!(builder, gtk::SearchEntry, person_search_entry);
        get_widget!(builder, gtk::ListBox, person_list);
        get_widget!(builder, gtk::SearchEntry, work_search_entry);
        get_widget!(builder, gtk::ListBox, work_list);

        let db = Rc::new(Database::new("test.sqlite"));
        let persons = db.get_persons();

        let result = Rc::new(Window {
            window: window,
            db: db,
            persons: RefCell::new(persons),
            person_list: person_list,
            person_search_entry: person_search_entry,
            works: RefCell::new(Vec::new()),
            work_search_entry: work_search_entry,
            work_list: work_list,
        });

        result
            .person_list
            .connect_row_activated(clone!(@strong result => move |_, row| {
                let row = row.get_child().unwrap().downcast::<SelectorRow>().unwrap();
                let index: usize = row.get_index().try_into().unwrap();

                let works = result.db.get_work_descriptions(result.persons.borrow()[index].id);
                result.works.replace(works);
                result.show_works();
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

        // result
        //     .work_list
        //     .connect_row_activated(clone!(@strong result => move |_, row| {
        //         let row = row.get_child().unwrap().downcast::<SelectorRow>().unwrap();
        //         let index: usize = row.get_index().try_into().unwrap();
        //     }));

        result
            .work_list
            .set_filter_func(Some(Box::new(clone!(@strong result => move |row| {
                let row = row.get_child().unwrap().downcast::<SelectorRow>().unwrap();
                let index: usize = row.get_index().try_into().unwrap();
                let search = result.work_search_entry.get_text().to_string().to_lowercase();

                search.is_empty() || result.works.borrow()[index]
                    .title
                    .to_lowercase()
                    .contains(&search)
            }))));

        result
            .work_search_entry
            .connect_search_changed(clone!(@strong result => move |_| {
                result.work_list.invalidate_filter();
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

    fn show_works(&self) {
        for child in self.work_list.get_children() {
            self.work_list.remove(&child);
        }

        for (index, work) in self.works.borrow().iter().enumerate() {
            let label = gtk::Label::new(Some(&work.title));
            label.set_halign(gtk::Align::Start);
            let row = SelectorRow::new(index.try_into().unwrap(), &label);
            row.show_all();
            self.work_list.insert(&row, -1);
        }
    }
}
