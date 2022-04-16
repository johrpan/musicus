use super::selector::Selector;
use crate::editors::EnsembleEditor;
use crate::navigator::{NavigationHandle, Screen};
use crate::widgets::Widget;
use adw::builders::ActionRowBuilder;
use adw::prelude::*;
use gettextrs::gettext;
use glib::clone;
use log::warn;
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

        let selector = Selector::<Ensemble>::new();
        selector.set_title(&gettext("Select ensemble"));

        let this = Rc::new(Self { handle, selector });

        // Connect signals and callbacks

        this.selector.set_back_cb(clone!(@weak this =>  move || {
            this.handle.pop(None);
        }));

        this.selector.set_add_cb(clone!(@weak this =>  move || {
            spawn!(@clone this, async move {
                if let Some(ensemble) = push!(this.handle, EnsembleEditor, None).await {
                    this.handle.pop(Some(ensemble));
                }
            });
        }));

        this.selector
            .set_make_widget(clone!(@weak this => @default-panic,  move |ensemble| {
                let row = ActionRowBuilder::new()
                    .activatable(true)
                    .title(&ensemble.name)
                    .build();

                let ensemble = ensemble.to_owned();

                row.connect_activated(clone!(@weak this =>  move |_| {
                    if let Err(err) = this.handle.backend.db().update_ensemble(ensemble.clone()) {
                        warn!("Failed to update access time. {err}");
                    }

                    this.handle.pop(Some(ensemble.clone()))
                }));

                row.upcast()
            }));

        this.selector
            .set_filter(|search, ensemble| ensemble.name.to_lowercase().contains(search));

        this.selector
            .set_items(this.handle.backend.db().get_recent_ensembles().unwrap());

        this
    }
}

impl Widget for EnsembleSelector {
    fn get_widget(&self) -> gtk::Widget {
        self.selector.widget.clone().upcast()
    }
}
