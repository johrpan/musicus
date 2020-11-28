use crate::backend::Backend;
use crate::database::*;
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
    window: libhandy::Window,
    stack: gtk::Stack,
    info_bar: gtk::InfoBar,
    name_entry: gtk::Entry,
    upload_switch: gtk::Switch,
    saved_cb: RefCell<Option<Box<dyn Fn(Ensemble) -> ()>>>,
}

impl EnsembleEditor {
    /// Create a new ensemble editor and optionally initialize it.
    pub fn new<P: IsA<gtk::Window>>(
        backend: Rc<Backend>,
        parent: &P,
        ensemble: Option<Ensemble>,
    ) -> Rc<Self> {
        // Create UI

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/ensemble_editor.ui");

        get_widget!(builder, libhandy::Window, window);
        get_widget!(builder, gtk::Button, cancel_button);
        get_widget!(builder, gtk::Button, save_button);
        get_widget!(builder, gtk::Stack, stack);
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
            window,
            stack,
            info_bar,
            name_entry,
            upload_switch,
            saved_cb: RefCell::new(None),
        });

        // Connect signals and callbacks

        cancel_button.connect_clicked(clone!(@strong this => move |_| {
            this.window.close();
        }));

        save_button.connect_clicked(clone!(@strong this => move |_| {
            let context = glib::MainContext::default();
            let clone = this.clone();
            context.spawn_local(async move {
                clone.stack.set_visible_child_name("loading");
                match clone.clone().save().await {
                    Ok(_) => {
                        clone.window.close();
                    }
                    Err(_) => {
                        clone.info_bar.set_revealed(true);
                        clone.stack.set_visible_child_name("content");
                    }
                }

            });
        }));

        this.window.set_transient_for(Some(parent));

        this
    }

    /// Set the closure to be called if the ensemble was saved.
    pub fn set_saved_cb<F: Fn(Ensemble) -> () + 'static>(&self, cb: F) {
        self.saved_cb.replace(Some(Box::new(cb)));
    }

    /// Show the ensemble editor.
    pub fn show(&self) {
        self.window.show();
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

        self.backend.db().update_ensemble(ensemble.clone()).await?;
        self.backend.library_changed();

        if let Some(cb) = &*self.saved_cb.borrow() {
            cb(ensemble.clone());
        }

        Ok(())
    }
}
