use crate::backend::generate_id;
use crate::backend::{Backend, Person};
use crate::navigator::{NavigationHandle, Screen};
use crate::widgets::{Editor, EntryRow, Section, UploadSection, Widget};
use anyhow::Result;
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// A dialog for creating or editing a person.
pub struct PersonEditor {
    handle: NavigationHandle<Person>,

    /// The ID of the person that is edited or a newly generated one.
    id: String,

    editor: Editor,
    first_name: EntryRow,
    last_name: EntryRow,
    upload: UploadSection,
}

impl Screen<Option<Person>, Person> for PersonEditor {
    /// Create a new person editor and optionally initialize it.
    fn new(person: Option<Person>, handle: NavigationHandle<Person>) -> Rc<Self> {
        let editor = Editor::new();
        editor.set_title("Person");

        let list = gtk::ListBoxBuilder::new()
            .selection_mode(gtk::SelectionMode::None)
            .build();

        let first_name = EntryRow::new(&gettext("First name"));
        let last_name = EntryRow::new(&gettext("Last name"));

        list.append(&first_name.widget);
        list.append(&last_name.widget);

        let section = Section::new(&gettext("General"), &list);
        let upload = UploadSection::new();

        editor.add_content(&section.widget);
        editor.add_content(&upload.widget);

        let id = match person {
            Some(person) => {
                first_name.set_text(&person.first_name);
                last_name.set_text(&person.last_name);

                person.id
            }
            None => generate_id(),
        };

        let this = Rc::new(Self {
            handle,
            id,
            editor,
            first_name,
            last_name,
            upload,
        });

        // Connect signals and callbacks

        this.editor.set_back_cb(clone!(@weak this => move || {
            this.handle.pop(None);
        }));

        this.editor.set_save_cb(clone!(@strong this => move || {
            spawn!(@clone this, async move {
                this.editor.loading();
                match this.save().await {
                    Ok(person) => {
                        this.handle.pop(Some(person));
                    }
                    Err(err) => {
                        let description = gettext!("Cause: {}", err);
                        this.editor.error(&gettext("Failed to save person!"), &description);
                    }
                }
            });
        }));

        this
    }
}

impl PersonEditor {
    /// Save the person and possibly upload it to the server.
    async fn save(self: &Rc<Self>) -> Result<Person> {
        let first_name = self.first_name.get_text();
        let last_name = self.last_name.get_text();

        let person = Person {
            id: self.id.clone(),
            first_name,
            last_name,
        };

        if self.upload.get_active() {
            self.handle.backend.post_person(&person).await?;
        }

        self.handle.backend.db().update_person(person.clone()).await?;
        self.handle.backend.library_changed();

        Ok(person)
    }
}

impl Widget for PersonEditor {
    fn get_widget(&self) -> gtk::Widget {
        self.editor.widget.clone().upcast()
    }
}

