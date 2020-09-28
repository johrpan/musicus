use super::selector_row::SelectorRow;
use super::PersonEditor;
use crate::database::*;
use gio::prelude::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use std::cell::RefCell;
use std::convert::TryInto;
use std::rc::Rc;

pub struct PersonSelector<F>
where
    F: Fn(Person) -> () + 'static,
{
    db: Rc<Database>,
    window: gtk::Window,
    callback: F,
    persons: RefCell<Vec<Person>>,
    list: gtk::ListBox,
    search_entry: gtk::SearchEntry,
}

impl<F> PersonSelector<F>
where
    F: Fn(Person) -> () + 'static,
{
    pub fn new<P: IsA<gtk::Window>>(db: Rc<Database>, parent: &P, callback: F) -> Rc<Self> {
        let builder =
            gtk::Builder::from_resource("/de/johrpan/musicus_editor/ui/person_selector.ui");

        get_widget!(builder, gtk::Window, window);
        get_widget!(builder, gtk::Button, add_button);
        get_widget!(builder, gtk::SearchEntry, search_entry);
        get_widget!(builder, gtk::ListBox, list);

        let persons = db.get_persons();

        for (index, person) in persons.iter().enumerate() {
            let label = gtk::Label::new(Some(&person.name_lf()));
            label.set_halign(gtk::Align::Start);
            let row = SelectorRow::new(index.try_into().unwrap(), &label);
            row.show_all();
            list.insert(&row, -1);
        }

        let result = Rc::new(PersonSelector {
            db: db,
            window: window,
            callback: callback,
            persons: RefCell::new(persons),
            search_entry: search_entry,
            list: list,
        });

        result
            .list
            .connect_row_activated(clone!(@strong result => move |_, row| {
                result.window.close();
                let row = row.get_child().unwrap().downcast::<SelectorRow>().unwrap();
                let index: usize = row.get_index().try_into().unwrap();
                (result.callback)(result.persons.borrow()[index].clone());
            }));

        result
            .list
            .set_filter_func(Some(Box::new(clone!(@strong result => move |row| {
                let row = row.get_child().unwrap().downcast::<SelectorRow>().unwrap();
                let index: usize = row.get_index().try_into().unwrap();
                let search = result.search_entry.get_text().to_string().to_lowercase();

                search.is_empty() || result.persons.borrow()[index]
                    .name_lf()
                    .to_lowercase()
                    .contains(&search)
            }))));

        result
            .search_entry
            .connect_search_changed(clone!(@strong result => move |_| {
                result.list.invalidate_filter();
            }));

        add_button.connect_clicked(clone!(@strong result => move |_| {
            let editor = PersonEditor::new(
                result.db.clone(),
                &result.window,
                None,
                clone!(@strong result => move |person| {
                    result.window.close();
                    (result.callback)(person);
                }),
            );

            editor.show();
        }));

        result.window.set_transient_for(Some(parent));

        result
    }

    pub fn show(&self) {
        self.window.show();
    }
}
