use crate::navigator::{NavigationHandle, Screen};
use crate::widgets::{Editor, EntryRow, Section, Widget};
use anyhow::Result;
use gettextrs::gettext;
use gtk::{glib::clone, prelude::*, builders::ListBoxBuilder};
use musicus_backend::db::{generate_id, Instrument};
use std::rc::Rc;

/// A dialog for creating or editing a instrument.
pub struct InstrumentEditor {
    handle: NavigationHandle<Instrument>,

    /// The ID of the instrument that is edited or a newly generated one.
    id: String,

    editor: Editor,
    name: EntryRow,
}

impl Screen<Option<Instrument>, Instrument> for InstrumentEditor {
    /// Create a new instrument editor and optionally initialize it.
    fn new(instrument: Option<Instrument>, handle: NavigationHandle<Instrument>) -> Rc<Self> {
        let editor = Editor::new();
        editor.set_title("Instrument/Role");

        let list = ListBoxBuilder::new()
            .selection_mode(gtk::SelectionMode::None)
            .css_classes(vec![String::from("boxed-list")])
            .build();

        let name = EntryRow::new(&gettext("Name"));
        list.append(&name.widget);

        let section = Section::new(&gettext("General"), &list);
        editor.add_content(&section.widget);

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
        });

        // Connect signals and callbacks

        this.editor.set_back_cb(clone!(@weak this => move || {
            this.handle.pop(None);
        }));

        this.editor.set_save_cb(clone!(@weak this => move || {
            match this.save() {
                Ok(instrument) => {
                    this.handle.pop(Some(instrument));
                }
                Err(err) => {
                    let description = gettext!("Cause: {}", err);
                    this.editor.error(&gettext("Failed to save instrument!"), &description);
                }
            }
        }));

        this.name
            .entry
            .connect_changed(clone!(@weak this => move |_| this.validate()));

        this.validate();

        this
    }
}

impl InstrumentEditor {
    /// Validate inputs and enable/disable saving.
    fn validate(&self) {
        self.editor.set_may_save(!self.name.get_text().is_empty());
    }

    /// Save the instrument.
    fn save(&self) -> Result<Instrument> {
        let name = self.name.get_text();

        let instrument = Instrument::new(self.id.clone(), name);

        self.handle
            .backend
            .db()
            .update_instrument(instrument.clone())?;
        self.handle.backend.library_changed();

        Ok(instrument)
    }
}

impl Widget for InstrumentEditor {
    fn get_widget(&self) -> gtk::Widget {
        self.editor.widget.clone().upcast()
    }
}
