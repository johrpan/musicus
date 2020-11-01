use crate::backend::*;
use crate::database::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use std::rc::Rc;

pub struct InstrumentEditor<F>
where
    F: Fn(Instrument) -> () + 'static,
{
    backend: Rc<Backend>,
    window: gtk::Window,
    callback: F,
    id: i64,
    name_entry: gtk::Entry,
}

impl<F> InstrumentEditor<F>
where
    F: Fn(Instrument) -> () + 'static,
{
    pub fn new<P: IsA<gtk::Window>>(
        backend: Rc<Backend>,
        parent: &P,
        instrument: Option<Instrument>,
        callback: F,
    ) -> Rc<Self> {
        let builder =
            gtk::Builder::from_resource("/de/johrpan/musicus/ui/instrument_editor.ui");

        get_widget!(builder, gtk::Window, window);
        get_widget!(builder, gtk::Button, cancel_button);
        get_widget!(builder, gtk::Button, save_button);
        get_widget!(builder, gtk::Entry, name_entry);

        let id = match instrument {
            Some(instrument) => {
                name_entry.set_text(&instrument.name);
                instrument.id
            }
            None => rand::random::<u32>().into(),
        };

        let result = Rc::new(InstrumentEditor {
            backend: backend,
            window: window,
            callback: callback,
            id: id,
            name_entry: name_entry,
        });

        cancel_button.connect_clicked(clone!(@strong result => move |_| {
            result.window.close();
        }));

        save_button.connect_clicked(clone!(@strong result => move |_| {
            let instrument = Instrument {
                id: result.id,
                name: result.name_entry.get_text().to_string(),
            };

            let c = glib::MainContext::default();
            let clone = result.clone();
            c.spawn_local(async move {
                clone.backend.update_instrument(instrument.clone()).await.unwrap();
                clone.window.close();
                (clone.callback)(instrument.clone());
            });
        }));

        result.window.set_transient_for(Some(parent));

        result
    }

    pub fn show(&self) {
        self.window.show();
    }
}
