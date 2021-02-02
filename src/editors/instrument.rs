use crate::backend::Backend;
use crate::database::generate_id;
use crate::database::Instrument;
use crate::widgets::{Editor, EntryRow, Navigator, NavigatorScreen, Section, UploadSection};
use anyhow::Result;
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// A dialog for creating or editing a instrument.
pub struct InstrumentEditor {
    backend: Rc<Backend>,

    /// The ID of the instrument that is edited or a newly generated one.
    id: String,

    editor: Editor,
    name: EntryRow,
    upload: UploadSection,
    saved_cb: RefCell<Option<Box<dyn Fn(Instrument) -> ()>>>,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl InstrumentEditor {
    /// Create a new instrument editor and optionally initialize it.
    pub fn new(backend: Rc<Backend>, instrument: Option<Instrument>) -> Rc<Self> {
        let editor = Editor::new();
        editor.set_title("Instrument/Role");

        let list = gtk::ListBoxBuilder::new()
            .selection_mode(gtk::SelectionMode::None)
            .build();

        let name = EntryRow::new(&gettext("Name"));
        list.append(&name.widget);

        let section = Section::new(&gettext("General"), &list);
        let upload = UploadSection::new();

        editor.add_content(&section.widget);
        editor.add_content(&upload.widget);

        let id = match instrument {
            Some(instrument) => {
                name.set_text(&instrument.name);
                instrument.id
            }
            None => generate_id(),
        };

        let this = Rc::new(Self {
            backend,
            id,
            editor,
            name,
            upload,
            saved_cb: RefCell::new(None),
            navigator: RefCell::new(None),
        });

        // Connect signals and callbacks

        this.editor.set_back_cb(clone!(@strong this => move || {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                navigator.pop();
            }
        }));

        this.editor.set_save_cb(clone!(@strong this => move || {
            let context = glib::MainContext::default();
            let clone = this.clone();
            context.spawn_local(async move {
                clone.editor.loading();
                match clone.clone().save().await {
                    Ok(_) => {
                        let navigator = clone.navigator.borrow().clone();
                        if let Some(navigator) = navigator {
                            navigator.pop();
                        }
                    }
                    Err(err) => {
                        let description = gettext!("Cause: {}", err);
                        clone.editor.error(&gettext("Failed to save instrument!"), &description);
                    }
                }

            });
        }));

        this
    }

    /// Set the closure to be called if the instrument was saved.
    pub fn set_saved_cb<F: Fn(Instrument) -> () + 'static>(&self, cb: F) {
        self.saved_cb.replace(Some(Box::new(cb)));
    }

    /// Save the instrument and possibly upload it to the server.
    async fn save(self: Rc<Self>) -> Result<()> {
        let name = self.name.get_text();

        let instrument = Instrument {
            id: self.id.clone(),
            name,
        };

        if self.upload.get_active() {
            self.backend.post_instrument(&instrument).await?;
        }

        self.backend.db().update_instrument(instrument.clone()).await?;
        self.backend.library_changed();

        if let Some(cb) = &*self.saved_cb.borrow() {
            cb(instrument.clone());
        }

        Ok(())
    }
}

impl NavigatorScreen for InstrumentEditor {
    fn attach_navigator(&self, navigator: Rc<Navigator>) {
        self.navigator.replace(Some(navigator));
    }

    fn get_widget(&self) -> gtk::Widget {
        self.editor.widget.clone().upcast()
    }

    fn detach_navigator(&self) {
        self.navigator.replace(None);
    }
}

