use crate::backend::Backend;
use crate::database::generate_id;
use crate::database::Instrument;
use crate::navigator::{NavigationHandle, Screen};
use crate::widgets::{Editor, EntryRow, Section, UploadSection, Widget};
use anyhow::Result;
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// A dialog for creating or editing a instrument.
pub struct InstrumentEditor {
    handle: NavigationHandle<Instrument>,

    /// The ID of the instrument that is edited or a newly generated one.
    id: String,

    editor: Editor,
    name: EntryRow,
    upload: UploadSection,
}

impl Screen<Option<Instrument>, Instrument> for InstrumentEditor {
    /// Create a new instrument editor and optionally initialize it.
    fn new(instrument: Option<Instrument>, handle: NavigationHandle<Instrument>) -> Rc<Self> {
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
            handle,
            id,
            editor,
            name,
            upload,
        });

        // Connect signals and callbacks

        this.editor.set_back_cb(clone!(@weak this => move || {
            this.handle.pop(None);
        }));

        this.editor.set_save_cb(clone!(@weak this => move || {
            spawn!(@clone this, async move {
                this.editor.loading();
                match this.save().await {
                    Ok(instrument) => {
                        this.handle.pop(Some(instrument));
                    }
                    Err(err) => {
                        let description = gettext!("Cause: {}", err);
                        this.editor.error(&gettext("Failed to save instrument!"), &description);
                    }
                }
            });
        }));

        this
    }
}

impl InstrumentEditor {
    /// Save the instrument and possibly upload it to the server.
    async fn save(&self) -> Result<Instrument> {
        let name = self.name.get_text();

        let instrument = Instrument {
            id: self.id.clone(),
            name,
        };

        if self.upload.get_active() {
            self.handle.backend.post_instrument(&instrument).await?;
        }

        self.handle.backend.db().update_instrument(instrument.clone()).await?;
        self.handle.backend.library_changed();

        Ok(instrument)
    }
}

impl Widget for InstrumentEditor {
    fn get_widget(&self) -> gtk::Widget {
        self.editor.widget.clone().upcast()
    }
}

