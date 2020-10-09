use crate::backend::*;
use crate::database::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use std::rc::Rc;

pub struct EnsembleEditor<F>
where
    F: Fn(Ensemble) -> () + 'static,
{
    window: gtk::Window,
    callback: F,
    id: i64,
    name_entry: gtk::Entry,
}

impl<F> EnsembleEditor<F>
where
    F: Fn(Ensemble) -> () + 'static,
{
    pub fn new<P: IsA<gtk::Window>>(
        backend: Rc<Backend>,
        parent: &P,
        ensemble: Option<Ensemble>,
        callback: F,
    ) -> Rc<Self> {
        let builder =
            gtk::Builder::from_resource("/de/johrpan/musicus_editor/ui/ensemble_editor.ui");

        get_widget!(builder, gtk::Window, window);
        get_widget!(builder, gtk::Button, cancel_button);
        get_widget!(builder, gtk::Button, save_button);
        get_widget!(builder, gtk::Entry, name_entry);

        let id = match ensemble {
            Some(ensemble) => {
                name_entry.set_text(&ensemble.name);
                ensemble.id
            }
            None => rand::random::<u32>().into(),
        };

        let result = Rc::new(EnsembleEditor {
            window: window,
            callback: callback,
            id: id,
            name_entry: name_entry,
        });

        cancel_button.connect_clicked(clone!(@strong result => move |_| {
            result.window.close();
        }));

        save_button.connect_clicked(clone!(@strong result => move |_| {
            let ensemble = Ensemble {
                id: result.id,
                name: result.name_entry.get_text().to_string(),
            };

            backend.update_ensemble(ensemble.clone(), clone!(@strong result => move |_| {
                result.window.close();
                (result.callback)(ensemble.clone());
            }));
        }));

        result.window.set_transient_for(Some(parent));

        result
    }

    pub fn show(&self) {
        self.window.show();
    }
}
