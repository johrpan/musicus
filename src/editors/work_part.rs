use crate::backend::Backend;
use crate::database::*;
use crate::selectors::PersonSelector;
use crate::widgets::{Navigator, NavigatorScreen};
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// A dialog for creating or editing a work part.
pub struct WorkPartEditor {
    backend: Rc<Backend>,
    widget: gtk::Box,
    title_entry: gtk::Entry,
    composer_row: libadwaita::ActionRow,
    reset_composer_button: gtk::Button,
    composer: RefCell<Option<Person>>,
    ready_cb: RefCell<Option<Box<dyn Fn(WorkPart) -> ()>>>,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl WorkPartEditor {
    /// Create a new part editor and optionally initialize it.
    pub fn new(backend: Rc<Backend>, part: Option<WorkPart>) -> Rc<Self> {
        // Create UI

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/work_part_editor.ui");

        get_widget!(builder, gtk::Box, widget);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::Button, save_button);
        get_widget!(builder, gtk::Entry, title_entry);
        get_widget!(builder, gtk::Button, composer_button);
        get_widget!(builder, libadwaita::ActionRow, composer_row);
        get_widget!(builder, gtk::Button, reset_composer_button);

        let composer = match part {
            Some(part) => {
                title_entry.set_text(&part.title);
                part.composer
            }
            None => None,
        };

        let this = Rc::new(Self {
            backend,
            widget,
            title_entry,
            composer_row,
            reset_composer_button,
            composer: RefCell::new(composer),
            ready_cb: RefCell::new(None),
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
            if let Some(cb) = &*this.ready_cb.borrow() {
                cb(WorkPart {
                    title: this.title_entry.get_text().unwrap().to_string(),
                    composer: this.composer.borrow().clone(),
                });
            }

            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                navigator.pop();
            }
        }));

        composer_button.connect_clicked(clone!(@strong this => move |_| {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                let selector = PersonSelector::new(this.backend.clone());

                selector.set_selected_cb(clone!(@strong this, @strong navigator => move |person| {
                    this.show_composer(Some(person));
                    this.composer.replace(Some(person.clone()));
                    navigator.clone().pop();
                }));

                navigator.push(selector);
            }

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

    /// Update the UI according to person.
    fn show_composer(&self, person: Option<&Person>) {
        if let Some(person) = person {
            self.composer_row.set_title(Some(&gettext("Composer")));
            self.composer_row.set_subtitle(Some(&person.name_fl()));
            self.reset_composer_button.show();
        } else {
            self.composer_row.set_title(Some(&gettext("Select a composer")));
            self.composer_row.set_subtitle(None);
            self.reset_composer_button.hide();
        }
    }
}

impl NavigatorScreen for WorkPartEditor {
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
