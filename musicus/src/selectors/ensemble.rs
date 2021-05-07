use super::selector::Selector;
use crate::editors::EnsembleEditor;
use crate::navigator::{NavigationHandle, Screen};
use crate::widgets::Widget;
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use libadwaita::prelude::*;
use musicus_backend::db::Ensemble;
use std::rc::Rc;

/// A screen for selecting a ensemble.
pub struct EnsembleSelector {
    handle: NavigationHandle<Ensemble>,
    selector: Rc<Selector<Ensemble>>,
}

impl Screen<(), Ensemble> for EnsembleSelector {
    /// Create a new ensemble selector.
    fn new(_: (), handle: NavigationHandle<Ensemble>) -> Rc<Self> {
        // Create UI

        let selector = Selector::<Ensemble>::new(Rc::clone(&handle.backend));
        selector.set_title(&gettext("Select ensemble"));

        let this = Rc::new(Self {
            handle,
            selector,
        });

        // Connect signals and callbacks

        this.selector.set_back_cb(clone!(@weak this => move || {
            this.handle.pop(None);
        }));

        this.selector.set_add_cb(clone!(@weak this => move || {
            spawn!(@clone this, async move {
                if let Some(ensemble) = push!(this.handle, EnsembleEditor, None).await {
                    this.handle.pop(Some(ensemble));
                }
            });
        }));

        this.selector.set_load_online(clone!(@weak this => move || {
            let clone = this.clone();
            async move { Ok(clone.handle.backend.cl().get_ensembles().await?) }
        }));

        this.selector.set_load_local(clone!(@weak this => move || {
            let clone = this.clone();
            async move { clone.handle.backend.db().get_ensembles().await.unwrap() }
        }));

        this.selector.set_make_widget(clone!(@weak this => move |ensemble| {
            let row = libadwaita::ActionRow::new();
            row.set_activatable(true);
            row.set_title(Some(&ensemble.name));

            let ensemble = ensemble.to_owned();
            row.connect_activated(clone!(@weak this => move |_| {
                this.handle.pop(Some(ensemble.clone()))
            }));

            row.upcast()
        }));

        this.selector
            .set_filter(|search, ensemble| ensemble.name.to_lowercase().contains(search));

        this
    }
}

impl Widget for EnsembleSelector {
    fn get_widget(&self) -> gtk::Widget {
        self.selector.widget.clone().upcast()
    }
}
