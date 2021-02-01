use crate::backend::Backend;
use crate::database::generate_id;
use crate::database::Person;
use crate::widgets::{Editor, EntryRow, Navigator, NavigatorScreen, Section, UploadSection};
use anyhow::Result;
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use std::cell::RefCell;
use std::rc::Rc;

/// A dialog for creating or editing a person.
pub struct PersonEditor {
    backend: Rc<Backend>,

    /// The ID of the person that is edited or a newly generated one.
    id: String,

    editor: Editor,
    first_name: EntryRow,
    last_name: EntryRow,
    upload: UploadSection,
    saved_cb: RefCell<Option<Box<dyn Fn(Person) -> ()>>>,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl PersonEditor {
    /// Create a new person editor and optionally initialize it.
    pub fn new(backend: Rc<Backend>, person: Option<Person>) -> Rc<Self> {
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
            backend,
            id,
            editor,
            first_name,
            last_name,
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
                        clone.editor.error(&gettext("Failed to save person!"), &description);
                    }
                }

            });
        }));

        this
    }

    /// Set the closure to be called if the person was saved.
    pub fn set_saved_cb<F: Fn(Person) -> () + 'static>(&self, cb: F) {
        self.saved_cb.replace(Some(Box::new(cb)));
    }

    /// Save the person and possibly upload it to the server.
    async fn save(self: Rc<Self>) -> Result<()> {
        let first_name = self.first_name.get_text();
        let last_name = self.last_name.get_text();

        let person = Person {
            id: self.id.clone(),
            first_name,
            last_name,
        };

        if self.upload.get_active() {
            self.backend.post_person(&person).await?;
        }

        self.backend.db().update_person(person.clone()).await?;
        self.backend.library_changed();

        if let Some(cb) = &*self.saved_cb.borrow() {
            cb(person.clone());
        }

        Ok(())
    }
}

impl NavigatorScreen for PersonEditor {
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

