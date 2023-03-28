use super::selector::Selector;
use crate::editors::InstrumentEditor;
use crate::navigator::{NavigationHandle, Screen};
use crate::widgets::Widget;

use adw::prelude::*;
use gettextrs::gettext;
use glib::clone;
use log::warn;
use musicus_backend::db::{self, Instrument};
use std::rc::Rc;

/// A screen for selecting a instrument.
pub struct InstrumentSelector {
    handle: NavigationHandle<Instrument>,
    selector: Rc<Selector<Instrument>>,
}

impl Screen<(), Instrument> for InstrumentSelector {
    /// Create a new instrument selector.
    fn new(_: (), handle: NavigationHandle<Instrument>) -> Rc<Self> {
        // Create UI

        let selector = Selector::<Instrument>::new();
        selector.set_title(&gettext("Select instrument"));

        let this = Rc::new(Self { handle, selector });

        // Connect signals and callbacks

        this.selector.set_back_cb(clone!(@weak this =>  move || {
            this.handle.pop(None);
        }));

        this.selector.set_add_cb(clone!(@weak this =>  move || {
            spawn!(@clone this, async move {
                if let Some(instrument) = push!(this.handle, InstrumentEditor, None).await {
                    this.handle.pop(Some(instrument));
                }
            });
        }));

        this.selector
            .set_make_widget(clone!(@weak this =>  @default-panic, move |instrument| {
                let row = adw::ActionRow::builder()
                    .activatable(true)
                    .title(&instrument.name)
                    .build();

                let instrument = instrument.to_owned();

                row.connect_activated(clone!(@weak this =>  move |_| {
                    if let Err(err) = db::update_instrument(&mut this.handle.backend.db().lock().unwrap(), instrument.clone()) {
                        warn!("Failed to update access time. {err}");
                    }

                    this.handle.pop(Some(instrument.clone()))
                }));

                row.upcast()
            }));

        this.selector
            .set_filter(|search, instrument| instrument.name.to_lowercase().contains(search));

        this.selector.set_items(
            db::get_recent_instruments(&mut this.handle.backend.db().lock().unwrap()).unwrap(),
        );

        this
    }
}

impl Widget for InstrumentSelector {
    fn get_widget(&self) -> gtk::Widget {
        self.selector.widget.clone().upcast()
    }
}
