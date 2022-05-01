use crate::navigator::{NavigationHandle, Screen};
use crate::widgets::{Editor, Section, Widget};
use anyhow::Result;
use gettextrs::gettext;
use glib::clone;
use gtk::{builders::ListBoxBuilder, prelude::*};
use musicus_backend::db::{generate_id, Person};
use std::rc::Rc;

/// A dialog for creating or editing a person.
pub struct PersonEditor {
    handle: NavigationHandle<Person>,

    /// The ID of the person that is edited or a newly generated one.
    id: String,

    editor: Editor,
    first_name: adw::EntryRow,
    last_name: adw::EntryRow,
}

impl Screen<Option<Person>, Person> for PersonEditor {
    /// Create a new person editor and optionally initialize it.
    fn new(person: Option<Person>, handle: NavigationHandle<Person>) -> Rc<Self> {
        let editor = Editor::new();
        editor.set_title("Person");

        let list = ListBoxBuilder::new()
            .selection_mode(gtk::SelectionMode::None)
            .css_classes(vec![String::from("boxed-list")])
            .build();

        let first_name = adw::EntryRow::builder()
            .title(&gettext("First name"))
            .build();

        let last_name = adw::EntryRow::builder()
            .title(&gettext("Last name"))
            .build();

        list.append(&first_name);
        list.append(&last_name);

        let section = Section::new(&gettext("General"), &list);
        editor.add_content(&section.widget);

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
        });

        // Connect signals and callbacks

        this.editor.set_back_cb(clone!(@weak this =>  move || {
            this.handle.pop(None);
        }));

        this.editor.set_save_cb(clone!(@strong this => move || {
            match this.save() {
                Ok(person) => {
                    this.handle.pop(Some(person));
                }
                Err(err) => {
                    let description = gettext!("Cause: {}", err);
                    this.editor.error(&gettext("Failed to save person!"), &description);
                }
            }
        }));

        this.first_name
            .connect_changed(clone!(@weak this =>  move |_| this.validate()));

        this.last_name
            .connect_changed(clone!(@weak this =>  move |_| this.validate()));

        this.validate();

        this
    }
}

impl PersonEditor {
    /// Validate inputs and enable/disable saving.
    fn validate(&self) {
        self.editor
            .set_may_save(!self.first_name.text().is_empty() && !self.last_name.text().is_empty());
    }

    /// Save the person.
    fn save(self: &Rc<Self>) -> Result<Person> {
        let first_name = self.first_name.text();
        let last_name = self.last_name.text();

        let person = Person::new(
            self.id.clone(),
            first_name.to_string(),
            last_name.to_string(),
        );

        self.handle.backend.db().update_person(person.clone())?;
        self.handle.backend.library_changed();

        Ok(person)
    }
}

impl Widget for PersonEditor {
    fn get_widget(&self) -> gtk::Widget {
        self.editor.widget.clone().upcast()
    }
}
