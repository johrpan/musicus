use super::selector_row::SelectorRow;
use super::InstrumentEditor;
use crate::database::*;
use gio::prelude::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use std::cell::RefCell;
use std::convert::TryInto;
use std::rc::Rc;

pub struct InstrumentSelector<F>
where
    F: Fn(Instrument) -> () + 'static,
{
    db: Rc<Database>,
    window: gtk::Window,
    callback: F,
    instruments: RefCell<Vec<Instrument>>,
    list: gtk::ListBox,
    search_entry: gtk::SearchEntry,
}

impl<F> InstrumentSelector<F>
where
    F: Fn(Instrument) -> () + 'static,
{
    pub fn new<P: IsA<gtk::Window>>(db: Rc<Database>, parent: &P, callback: F) -> Rc<Self> {
        let builder =
            gtk::Builder::from_resource("/de/johrpan/musicus_editor/ui/instrument_selector.ui");

        get_widget!(builder, gtk::Window, window);
        get_widget!(builder, gtk::Button, add_button);
        get_widget!(builder, gtk::SearchEntry, search_entry);
        get_widget!(builder, gtk::ListBox, list);

        let instruments = db.get_instruments();

        for (index, instrument) in instruments.iter().enumerate() {
            let label = gtk::Label::new(Some(&instrument.name));
            label.set_halign(gtk::Align::Start);
            let row = SelectorRow::new(index.try_into().unwrap(), &label);
            row.show_all();
            list.insert(&row, -1);
        }

        let result = Rc::new(InstrumentSelector {
            db: db,
            window: window,
            callback: callback,
            instruments: RefCell::new(instruments),
            search_entry: search_entry,
            list: list,
        });

        result
            .list
            .connect_row_activated(clone!(@strong result => move |_, row| {
                result.window.close();
                let row = row.get_child().unwrap().downcast::<SelectorRow>().unwrap();
                let index: usize = row.get_index().try_into().unwrap();
                (result.callback)(result.instruments.borrow()[index].clone());
            }));

        result
            .list
            .set_filter_func(Some(Box::new(clone!(@strong result => move |row| {
                let row = row.get_child().unwrap().downcast::<SelectorRow>().unwrap();
                let index: usize = row.get_index().try_into().unwrap();
                let search = result.search_entry.get_text().to_string();

                search.is_empty() || result.instruments.borrow()[index]
                    .name
                    .to_lowercase()
                    .contains(&result.search_entry.get_text().to_string().to_lowercase())
            }))));

        result
            .search_entry
            .connect_search_changed(clone!(@strong result => move |_| {
                result.list.invalidate_filter();
            }));

        add_button.connect_clicked(clone!(@strong result => move |_| {
            let editor = InstrumentEditor::new(
                result.db.clone(),
                &result.window,
                None,
                clone!(@strong result => move |instrument| {
                    result.window.close();
                    (result.callback)(instrument);
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
