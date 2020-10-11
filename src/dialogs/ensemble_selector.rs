use super::selector_row::SelectorRow;
use super::EnsembleEditor;
use crate::backend::Backend;
use crate::database::*;
use gio::prelude::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use std::convert::TryInto;
use std::rc::Rc;

pub struct EnsembleSelector<F>
where
    F: Fn(Ensemble) -> () + 'static,
{
    backend: Rc<Backend>,
    window: gtk::Window,
    callback: F,
    list: gtk::ListBox,
    search_entry: gtk::SearchEntry,
}

impl<F> EnsembleSelector<F>
where
    F: Fn(Ensemble) -> () + 'static,
{
    pub fn new<P: IsA<gtk::Window>>(backend: Rc<Backend>, parent: &P, callback: F) -> Rc<Self> {
        let builder =
            gtk::Builder::from_resource("/de/johrpan/musicus_editor/ui/ensemble_selector.ui");

        get_widget!(builder, gtk::Window, window);
        get_widget!(builder, gtk::Button, add_button);
        get_widget!(builder, gtk::SearchEntry, search_entry);
        get_widget!(builder, gtk::ListBox, list);

        let result = Rc::new(EnsembleSelector {
            backend: backend,
            window: window,
            callback: callback,
            search_entry: search_entry,
            list: list,
        });

        let c = glib::MainContext::default();
        let clone = result.clone();
        c.spawn_local(async move {
            let ensembles = clone.backend.get_ensembles().await.unwrap();

            for (index, ensemble) in ensembles.iter().enumerate() {
                let label = gtk::Label::new(Some(&ensemble.name));
                label.set_halign(gtk::Align::Start);
                let row = SelectorRow::new(index.try_into().unwrap(), &label);
                row.show_all();
                clone.list.insert(&row, -1);
            }

            clone.list.connect_row_activated(
                clone!(@strong clone, @strong ensembles => move |_, row| {
                    clone.window.close();
                    let row = row.get_child().unwrap().downcast::<SelectorRow>().unwrap();
                    let index: usize = row.get_index().try_into().unwrap();
                    (clone.callback)(ensembles[index].clone());
                }),
            );

            clone
                .list
                .set_filter_func(Some(Box::new(clone!(@strong clone => move |row| {
                    let row = row.get_child().unwrap().downcast::<SelectorRow>().unwrap();
                    let index: usize = row.get_index().try_into().unwrap();
                    let search = clone.search_entry.get_text().to_string().to_lowercase();
                    search.is_empty() || ensembles[index]
                        .name
                        .to_lowercase()
                        .contains(&search)
                }))));
        });

        result
            .search_entry
            .connect_search_changed(clone!(@strong result => move |_| {
                result.list.invalidate_filter();
            }));

        add_button.connect_clicked(clone!(@strong result => move |_| {
            let editor = EnsembleEditor::new(
                result.backend.clone(),
                &result.window,
                None,
                clone!(@strong result => move |ensemble| {
                    result.window.close();
                    (result.callback)(ensemble);
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
