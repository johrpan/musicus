use super::selector::Selector;
use crate::backend::Backend;
use crate::database::Instrument;
use crate::editors::InstrumentEditor;
use crate::widgets::{Navigator, NavigatorScreen};
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// A screen for selecting a instrument.
pub struct InstrumentSelector {
    backend: Rc<Backend>,
    selector: Rc<Selector<Instrument>>,
    selected_cb: RefCell<Option<Box<dyn Fn(&Instrument) -> ()>>>,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl InstrumentSelector {
    /// Create a new instrument selector.
    pub fn new(backend: Rc<Backend>) -> Rc<Self> {
        // Create UI

        let selector = Selector::<Instrument>::new();
        selector.set_title(&gettext("Select instrument"));

        let this = Rc::new(Self {
            backend,
            selector,
            selected_cb: RefCell::new(None),
            navigator: RefCell::new(None),
        });

        // Connect signals and callbacks

        this.selector.set_back_cb(clone!(@strong this => move || {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                navigator.pop();
            }
        }));

        this.selector.set_add_cb(clone!(@strong this => move || {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                let editor = InstrumentEditor::new(this.backend.clone(), None);
                editor
                    .set_saved_cb(clone!(@strong this => move |instrument| this.select(&instrument)));
                navigator.push(editor);
            }
        }));

        this.selector
            .set_load_online(clone!(@strong this => move || {
                let clone = this.clone();
                async move { clone.backend.get_instruments().await }
            }));

        this.selector
            .set_load_local(clone!(@strong this => move || {
                let clone = this.clone();
                async move { clone.backend.db().get_instruments().await.unwrap() }
            }));

        this.selector.set_make_widget(clone!(@strong this => move |instrument| {
            let row = libadwaita::ActionRow::new();
            row.set_activatable(true);
            row.set_title(Some(&instrument.name));

            let instrument = instrument.to_owned();
            row.connect_activated(clone!(@strong this => move |_| {
                this.select(&instrument);
            }));

            row.upcast()
        }));

        this.selector
            .set_filter(|search, instrument| instrument.name.to_lowercase().contains(search));

        this
    }

    /// Set the closure to be called when an item is selected.
    pub fn set_selected_cb<F: Fn(&Instrument) -> () + 'static>(&self, cb: F) {
        self.selected_cb.replace(Some(Box::new(cb)));
    }

    /// Select an instrument.
    fn select(&self, instrument: &Instrument) {
        if let Some(cb) = &*self.selected_cb.borrow() {
            cb(&instrument);
        }
    }
}

impl NavigatorScreen for InstrumentSelector {
    fn attach_navigator(&self, navigator: Rc<Navigator>) {
        self.navigator.replace(Some(navigator));
    }

    fn get_widget(&self) -> gtk::Widget {
        self.selector.widget.clone().upcast()
    }

    fn detach_navigator(&self) {
        self.navigator.replace(None);
    }
}
