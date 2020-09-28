use crate::database::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use std::rc::Rc;

pub struct PersonEditor {
    window: gtk::Window,
    id: i64,
    first_name_entry: gtk::Entry,
    last_name_entry: gtk::Entry,
}

impl PersonEditor {
    pub fn new<F: Fn(Person) -> () + 'static, P: IsA<gtk::Window>>(
        db: Rc<Database>,
        parent: &P,
        person: Option<Person>,
        callback: F,
    ) -> Rc<Self> {
        let builder = gtk::Builder::from_resource("/de/johrpan/musicus_editor/ui/person_editor.ui");

        get_widget!(builder, gtk::Window, window);
        get_widget!(builder, gtk::Button, cancel_button);
        get_widget!(builder, gtk::Button, save_button);
        get_widget!(builder, gtk::Entry, first_name_entry);
        get_widget!(builder, gtk::Entry, last_name_entry);

        let id = match person {
            Some(person) => {
                first_name_entry.set_text(&person.first_name);
                last_name_entry.set_text(&person.last_name);
                person.id
            }
            None => rand::random::<u32>().into(),
        };

        let result = Rc::new(PersonEditor {
            id: id,
            window: window,
            first_name_entry: first_name_entry,
            last_name_entry: last_name_entry,
        });

        cancel_button.connect_clicked(clone!(@strong result => move |_| {
            result.window.close();
        }));

        save_button.connect_clicked(clone!(@strong result => move |_| {
            result.window.close();

            let person = Person {
                id: result.id,
                first_name: result.first_name_entry.get_text().to_string(),
                last_name: result.last_name_entry.get_text().to_string(),
            };

            db.update_person(person.clone());
            callback(person);
        }));

        result.window.set_transient_for(Some(parent));

        result
    }

    pub fn show(&self) {
        self.window.show();
    }
}
