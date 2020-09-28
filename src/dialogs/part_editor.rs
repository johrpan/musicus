use super::selector_row::SelectorRow;
use super::{InstrumentSelector, PersonSelector};
use crate::database::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use std::cell::RefCell;
use std::convert::TryInto;
use std::rc::Rc;

pub struct PartEditor {
    db: Rc<Database>,
    window: gtk::Window,
    title_entry: gtk::Entry,
    composer: RefCell<Option<Person>>,
    composer_label: gtk::Label,
    instruments: RefCell<Vec<Instrument>>,
    instrument_list: gtk::ListBox,
}

impl PartEditor {
    pub fn new<F: Fn(WorkPartDescription) -> () + 'static, P: IsA<gtk::Window>>(
        db: Rc<Database>,
        parent: &P,
        part: Option<WorkPartDescription>,
        callback: F,
    ) -> Rc<Self> {
        let builder = gtk::Builder::from_resource("/de/johrpan/musicus_editor/ui/part_editor.ui");

        get_widget!(builder, gtk::Window, window);
        get_widget!(builder, gtk::Button, cancel_button);
        get_widget!(builder, gtk::Button, save_button);
        get_widget!(builder, gtk::Entry, title_entry);
        get_widget!(builder, gtk::Button, composer_button);
        get_widget!(builder, gtk::Label, composer_label);
        get_widget!(builder, gtk::Button, reset_composer_button);
        get_widget!(builder, gtk::ListBox, instrument_list);
        get_widget!(builder, gtk::Button, add_instrument_button);
        get_widget!(builder, gtk::Button, remove_instrument_button);

        match part.clone() {
            Some(part) => {
                title_entry.set_text(&part.title);
            }
            None => (),
        };

        let composer = RefCell::new(match part.clone() {
            Some(work) => {
                match work.composer.clone() {
                    Some(composer) => composer_label.set_text(&composer.name_fl()),
                    None => (),
                }

                work.composer
            },
            None => None,
        });

        let instruments = RefCell::new(match part.clone() {
            Some(work) => work.instruments,
            None => Vec::new(),
        });

        let result = Rc::new(PartEditor {
            db: db,
            window: window,
            title_entry: title_entry,
            composer: composer,
            composer_label: composer_label,
            instruments: instruments,
            instrument_list: instrument_list,
        });

        cancel_button.connect_clicked(clone!(@strong result => move |_| {
            result.window.close();
        }));

        save_button.connect_clicked(clone!(@strong result => move |_| {
            result.window.close();
            callback(WorkPartDescription {
                title: result.title_entry.get_text().to_string(),
                composer: result.composer.borrow().clone(),
                instruments: result.instruments.borrow().clone(),
            });
        }));

        composer_button.connect_clicked(clone!(@strong result => move |_| {
            PersonSelector::new(result.db.clone(), &result.window, clone!(@strong result => move |person| {
                result.composer.replace(Some(person.clone()));
                result.composer_label.set_text(&person.name_fl());
            })).show();
        }));

        reset_composer_button.connect_clicked(clone!(@strong result => move |_| {
            result.composer.replace(None);
            result.composer_label.set_text("Select …");
        }));

        add_instrument_button.connect_clicked(clone!(@strong result => move |_| {
            InstrumentSelector::new(result.db.clone(), &result.window, clone!(@strong result => move |instrument| {
                {
                    let mut instruments = result.instruments.borrow_mut();
                    instruments.push(instrument);
                }
                
                result.show_instruments();
            })).show();
        }));

        remove_instrument_button.connect_clicked(clone!(@strong result => move |_| {
            let row = result.get_selected_instrument_row();
            match row {
                Some(row) => {
                    let index = row.get_index();
                    let index: usize = index.try_into().unwrap();
                    result.instruments.borrow_mut().remove(index);
                    result.show_instruments();
                }
                None => (),
            }
        }));

        result.window.set_transient_for(Some(parent));

        result
    }

    pub fn show(&self) {
        self.window.show();
    }

    fn show_instruments(&self) {
        for child in self.instrument_list.get_children() {
            self.instrument_list.remove(&child);
        }

        for (index, instrument) in self.instruments.borrow().iter().enumerate() {
            let label = gtk::Label::new(Some(&instrument.name));
            label.set_halign(gtk::Align::Start);
            let row = SelectorRow::new(index.try_into().unwrap(), &label);
            row.show_all();
            self.instrument_list.insert(&row, -1);
        }
    }

    fn get_selected_instrument_row(&self) -> Option<SelectorRow> {
        match self.instrument_list.get_selected_rows().first() {
            Some(row) => match row.get_child() {
                Some(child) => Some(child.downcast().unwrap()),
                None => None,
            },
            None => None,
        }
    }
}
