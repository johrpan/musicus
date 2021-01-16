use super::selector::Selector;
use crate::backend::Backend;
use crate::database::Ensemble;
use crate::editors::EnsembleEditor;
use crate::widgets::{Navigator, NavigatorScreen};
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// A screen for selecting a ensemble.
pub struct EnsembleSelector {
    backend: Rc<Backend>,
    selector: Rc<Selector<Ensemble>>,
    selected_cb: RefCell<Option<Box<dyn Fn(&Ensemble) -> ()>>>,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl EnsembleSelector {
    /// Create a new ensemble selector.
    pub fn new(backend: Rc<Backend>) -> Rc<Self> {
        // Create UI

        let selector = Selector::<Ensemble>::new();
        selector.set_title(&gettext("Select ensemble"));

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
                let editor = EnsembleEditor::new(this.backend.clone(), None);
                editor
                    .set_saved_cb(clone!(@strong this => move |ensemble| this.select(&ensemble)));
                navigator.push(editor);
            }
        }));

        this.selector
            .set_load_online(clone!(@strong this => move || {
                let clone = this.clone();
                async move { clone.backend.get_ensembles().await }
            }));

        this.selector
            .set_load_local(clone!(@strong this => move || {
                let clone = this.clone();
                async move { clone.backend.db().get_ensembles().await.unwrap() }
            }));

        this.selector.set_make_widget(|ensemble| {
            let label = gtk::Label::new(Some(&ensemble.name));
            label.set_halign(gtk::Align::Start);
            label.set_margin_start(6);
            label.set_margin_end(6);
            label.set_margin_top(6);
            label.set_margin_bottom(6);
            label.upcast()
        });

        this.selector
            .set_filter(|search, ensemble| ensemble.name.to_lowercase().contains(search));

        this.selector
            .set_selected_cb(clone!(@strong this => move |ensemble| this.select(ensemble)));

        this
    }

    /// Set the closure to be called when an item is selected.
    pub fn set_selected_cb<F: Fn(&Ensemble) -> () + 'static>(&self, cb: F) {
        self.selected_cb.replace(Some(Box::new(cb)));
    }

    /// Select a ensemble.
    fn select(&self, ensemble: &Ensemble) {        
        if let Some(cb) = &*self.selected_cb.borrow() {
            cb(&ensemble);
        }

    }
}

impl NavigatorScreen for EnsembleSelector {
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
