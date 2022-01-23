use crate::navigator::{NavigationHandle, Screen};
use crate::widgets::{Editor, EntryRow, Section, Widget};
use anyhow::Result;
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use musicus_backend::db::{generate_id, Ensemble};
use std::rc::Rc;

/// A dialog for creating or editing a ensemble.
pub struct EnsembleEditor {
    handle: NavigationHandle<Ensemble>,

    /// The ID of the ensemble that is edited or a newly generated one.
    id: String,

    editor: Editor,
    name: EntryRow,
}

impl Screen<Option<Ensemble>, Ensemble> for EnsembleEditor {
    /// Create a new ensemble editor and optionally initialize it.
    fn new(ensemble: Option<Ensemble>, handle: NavigationHandle<Ensemble>) -> Rc<Self> {
        let editor = Editor::new();
        editor.set_title("Ensemble");

        let list = gtk::ListBoxBuilder::new()
            .selection_mode(gtk::SelectionMode::None)
            .build();

        let name = EntryRow::new(&gettext("Name"));
        list.append(&name.widget);

        let section = Section::new(&gettext("General"), &list);
        editor.add_content(&section.widget);

        let id = match ensemble {
            Some(ensemble) => {
                name.set_text(&ensemble.name);
                ensemble.id
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
                Ok(ensemble) => {
                    this.handle.pop(Some(ensemble));
                }
                Err(err) => {
                    let description = gettext!("Cause: {}", err);
                    this.editor.error(&gettext("Failed to save ensemble!"), &description);
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

impl EnsembleEditor {
    /// Validate inputs and enable/disable saving.
    fn validate(&self) {
        self.editor.set_may_save(!self.name.get_text().is_empty());
    }

    /// Save the ensemble.
    fn save(&self) -> Result<Ensemble> {
        let name = self.name.get_text();

        let ensemble = Ensemble {
            id: self.id.clone(),
            name,
        };

        self.handle.backend.db().update_ensemble(ensemble.clone())?;
        self.handle.backend.library_changed();

        Ok(ensemble)
    }
}

impl Widget for EnsembleEditor {
    fn get_widget(&self) -> gtk::Widget {
        self.editor.widget.clone().upcast()
    }
}
