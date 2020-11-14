use super::InstrumentEditor;
use crate::backend::Backend;
use crate::database::*;
use crate::widgets::*;
use gio::prelude::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use std::convert::TryInto;
use std::rc::Rc;

pub struct InstrumentSelector<F>
where
    F: Fn(Instrument) -> () + 'static,
{
    backend: Rc<Backend>,
    window: libhandy::Window,
    callback: F,
    list: gtk::ListBox,
    search_entry: gtk::SearchEntry,
}

impl<F> InstrumentSelector<F>
where
    F: Fn(Instrument) -> () + 'static,
{
    pub fn new<P: IsA<gtk::Window>>(backend: Rc<Backend>, parent: &P, callback: F) -> Rc<Self> {
        let builder =
            gtk::Builder::from_resource("/de/johrpan/musicus/ui/instrument_selector.ui");

        get_widget!(builder, libhandy::Window, window);
        get_widget!(builder, gtk::Button, add_button);
        get_widget!(builder, gtk::SearchEntry, search_entry);
        get_widget!(builder, gtk::ListBox, list);

        let result = Rc::new(InstrumentSelector {
            backend: backend,
            window: window,
            callback: callback,
            search_entry: search_entry,
            list: list,
        });

        let c = glib::MainContext::default();
        let clone = result.clone();
        c.spawn_local(async move {
            let instruments = clone.backend.get_instruments().await.unwrap();

            for (index, instrument) in instruments.iter().enumerate() {
                let label = gtk::Label::new(Some(&instrument.name));
                label.set_halign(gtk::Align::Start);
                label.set_margin_start(6);
                label.set_margin_end(6);
                label.set_margin_top(6);
                label.set_margin_bottom(6);

                let row = SelectorRow::new(index.try_into().unwrap(), &label);
                row.show_all();
                clone.list.insert(&row, -1);
            }

            clone.list.connect_row_activated(
                clone!(@strong clone, @strong instruments => move |_, row| {
                    clone.window.close();
                    let row = row.get_child().unwrap().downcast::<SelectorRow>().unwrap();
                    let index: usize = row.get_index().try_into().unwrap();
                    (clone.callback)(instruments[index].clone());
                }),
            );

            clone
                .list
                .set_filter_func(Some(Box::new(clone!(@strong clone => move |row| {
                    let row = row.get_child().unwrap().downcast::<SelectorRow>().unwrap();
                    let index: usize = row.get_index().try_into().unwrap();
                    let search = clone.search_entry.get_text().to_string();

                    search.is_empty() || instruments[index]
                        .name
                        .to_lowercase()
                        .contains(&clone.search_entry.get_text().to_string().to_lowercase())
                }))));
        });

        result
            .search_entry
            .connect_search_changed(clone!(@strong result => move |_| {
                result.list.invalidate_filter();
            }));

        add_button.connect_clicked(clone!(@strong result => move |_| {
            let editor = InstrumentEditor::new(
                result.backend.clone(),
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
