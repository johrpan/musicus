use crate::backend::Backend;
use crate::database::*;
use crate::widgets::{Navigator, NavigatorScreen};
use anyhow::Result;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use std::cell::RefCell;
use std::rc::Rc;

/// A dialog for creating or editing a ensemble.
pub struct EnsembleEditor {
    backend: Rc<Backend>,
    id: String,
    widget: gtk::Stack,
    info_bar: gtk::InfoBar,
    name_entry: gtk::Entry,
    upload_switch: gtk::Switch,
    saved_cb: RefCell<Option<Box<dyn Fn(Ensemble) -> ()>>>,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl EnsembleEditor {
    /// Create a new ensemble editor and optionally initialize it.
    pub fn new(backend: Rc<Backend>, ensemble: Option<Ensemble>) -> Rc<Self> {
        // Create UI

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/ensemble_editor.ui");

        get_widget!(builder, gtk::Stack, widget);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::Button, save_button);
        get_widget!(builder, gtk::InfoBar, info_bar);
        get_widget!(builder, gtk::Entry, name_entry);
        get_widget!(builder, gtk::Switch, upload_switch);

        let id = match ensemble {
            Some(ensemble) => {
                name_entry.set_text(&ensemble.name);

                ensemble.id
            }
            None => generate_id(),
        };

        let this = Rc::new(Self {
            backend,
            id,
            widget,
            info_bar,
            name_entry,
            upload_switch,
            saved_cb: RefCell::new(None),
            navigator: RefCell::new(None),
        });

        // Connect signals and callbacks

        back_button.connect_clicked(clone!(@strong this => move |_| {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                navigator.pop();
            }
        }));

        save_button.connect_clicked(clone!(@strong this => move |_| {
            let context = glib::MainContext::default();
            let clone = this.clone();
            context.spawn_local(async move {
                clone.widget.set_visible_child_name("loading");
                match clone.clone().save().await {
                    Ok(_) => {
                        let navigator = clone.navigator.borrow().clone();
                        if let Some(navigator) = navigator {
                            navigator.pop();
                        }
                    }
                    Err(_) => {
                        clone.info_bar.set_revealed(true);
                        clone.widget.set_visible_child_name("content");
                    }
                }

            });
        }));

        this
    }

    /// Set the closure to be called if the ensemble was saved.
    pub fn set_saved_cb<F: Fn(Ensemble) -> () + 'static>(&self, cb: F) {
        self.saved_cb.replace(Some(Box::new(cb)));
    }

    /// Save the ensemble and possibly upload it to the server.
    async fn save(self: Rc<Self>) -> Result<()> {
        let name = self.name_entry.get_text().to_string();

        let ensemble = Ensemble {
            id: self.id.clone(),
            name,
        };

        let upload = self.upload_switch.get_active();
        if upload {
            self.backend.post_ensemble(&ensemble).await?;
        }

        self.backend
            .db()
            .update_ensemble(ensemble.clone())
            .await?;
        self.backend.library_changed();

        if let Some(cb) = &*self.saved_cb.borrow() {
            cb(ensemble.clone());
        }

        Ok(())
    }
}

impl NavigatorScreen for EnsembleEditor {
    fn attach_navigator(&self, navigator: Rc<Navigator>) {
        self.navigator.replace(Some(navigator));
    }

    fn get_widget(&self) -> gtk::Widget {
        self.widget.clone().upcast()
    }

    fn detach_navigator(&self) {
        self.navigator.replace(None);
    }
}
