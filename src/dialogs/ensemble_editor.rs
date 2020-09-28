use crate::database::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use std::rc::Rc;

pub struct EnsembleEditor {
    window: gtk::Window,
    id: i64,
    name_entry: gtk::Entry,
}

impl EnsembleEditor {
    pub fn new<F: Fn(Ensemble) -> () + 'static, P: IsA<gtk::Window>>(
        db: Rc<Database>,
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
            id: id,
            window: window,
            name_entry: name_entry,
        });

        cancel_button.connect_clicked(clone!(@strong result => move |_| {
            result.window.close();
        }));

        save_button.connect_clicked(clone!(@strong result => move |_| {
            result.window.close();

            let ensemble = Ensemble {
                id: result.id,
                name: result.name_entry.get_text().to_string(),
            };

            db.update_ensemble(ensemble.clone());
            callback(ensemble);
        }));

        result.window.set_transient_for(Some(parent));

        result
    }

    pub fn show(&self) {
        self.window.show();
    }
}
