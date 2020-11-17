use crate::backend::*;
use crate::database::*;
use crate::dialogs::*;
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use std::cell::RefCell;
use std::rc::Rc;

/// A dialog for creating or editing a work part.
pub struct PartEditor {
    backend: Rc<Backend>,
    window: libhandy::Window,
    title_entry: gtk::Entry,
    composer_label: gtk::Label,
    reset_composer_button: gtk::Button,
    composer: RefCell<Option<Person>>,
    ready_cb: RefCell<Option<Box<dyn Fn(WorkPart) -> ()>>>,
}

impl PartEditor {
    /// Create a new part editor and optionally initialize it.
    pub fn new<P: IsA<gtk::Window>>(
        backend: Rc<Backend>,
        parent: &P,
        part: Option<WorkPart>,
    ) -> Rc<Self> {
        // Create UI

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/part_editor.ui");

        get_widget!(builder, libhandy::Window, window);
        get_widget!(builder, gtk::Button, cancel_button);
        get_widget!(builder, gtk::Button, save_button);
        get_widget!(builder, gtk::Entry, title_entry);
        get_widget!(builder, gtk::Button, composer_button);
        get_widget!(builder, gtk::Label, composer_label);
        get_widget!(builder, gtk::Button, reset_composer_button);

        window.set_transient_for(Some(parent));

        let composer = match part {
            Some(part) => {
                title_entry.set_text(&part.title);
                part.composer
            }
            None => None,
        };

        let this = Rc::new(Self {
            backend,
            window,
            title_entry,
            composer_label,
            reset_composer_button,
            composer: RefCell::new(composer),
            ready_cb: RefCell::new(None),
        });

        // Connect signals and callbacks

        cancel_button.connect_clicked(clone!(@strong this => move |_| {
            this.window.close();
        }));

        save_button.connect_clicked(clone!(@strong this => move |_| {
            if let Some(cb) = &*this.ready_cb.borrow() {
                cb(WorkPart {
                    title: this.title_entry.get_text().to_string(),
                    composer: this.composer.borrow().clone(),
                });
            }

            this.window.close();
        }));

        composer_button.connect_clicked(clone!(@strong this => move |_| {
            PersonSelector::new(this.backend.clone(), &this.window, clone!(@strong this => move |person| {
                this.show_composer(Some(&person));
                this.composer.replace(Some(person));
            })).show();
        }));

        this.reset_composer_button
            .connect_clicked(clone!(@strong this => move |_| {
                this.composer.replace(None);
                this.show_composer(None);
            }));

        // Initialize

        if let Some(composer) = &*this.composer.borrow() {
            this.show_composer(Some(composer));
        }

        this
    }

    /// Set the closure to be called when the user wants to save the part.
    pub fn set_ready_cb<F: Fn(WorkPart) -> () + 'static>(&self, cb: F) {
        self.ready_cb.replace(Some(Box::new(cb)));
    }

    /// Show the part editor.
    pub fn show(&self) {
        self.window.show();
    }

    /// Update the UI according to person.
    fn show_composer(&self, person: Option<&Person>) {
        if let Some(person) = person {
            self.composer_label.set_text(&person.name_fl());
            self.reset_composer_button.show();
        } else {
            self.composer_label.set_text(&gettext("Select …"));
            self.reset_composer_button.hide();
        }
    }
}
