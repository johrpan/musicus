use crate::database::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use std::rc::Rc;

pub struct InstrumentEditor {
    window: gtk::Window,
    id: i64,
    name_entry: gtk::Entry,
}

impl InstrumentEditor {
    pub fn new<F: Fn(Instrument) -> () + 'static, P: IsA<gtk::Window>>(
        db: Rc<Database>,
        parent: &P,
        instrument: Option<Instrument>,
        callback: F,
    ) -> Rc<Self> {
        let builder =
            gtk::Builder::from_resource("/de/johrpan/musicus_editor/ui/instrument_editor.ui");

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
            id: id,
            window: window,
            name_entry: name_entry,
        });

        cancel_button.connect_clicked(clone!(@strong result => move |_| {
            result.window.close();
        }));

        save_button.connect_clicked(clone!(@strong result => move |_| {
            result.window.close();

            let instrument = Instrument {
                id: result.id,
                name: result.name_entry.get_text().to_string(),
            };

            db.update_instrument(instrument.clone());
            callback(instrument);
        }));

        result.window.set_transient_for(Some(parent));

        result
    }

    pub fn show(&self) {
        self.window.show();
    }
}
