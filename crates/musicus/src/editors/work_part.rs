use crate::selectors::PersonSelector;
use crate::navigator::{NavigationHandle, Screen};
use crate::widgets::Widget;
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use libadwaita::prelude::*;
use musicus_backend::{Person, WorkPart};
use std::cell::RefCell;
use std::rc::Rc;

/// A dialog for creating or editing a work part.
pub struct WorkPartEditor {
    handle: NavigationHandle<WorkPart>,
    widget: gtk::Box,
    title_entry: gtk::Entry,
    composer_row: libadwaita::ActionRow,
    reset_composer_button: gtk::Button,
    composer: RefCell<Option<Person>>,
}

impl Screen<Option<WorkPart>, WorkPart> for WorkPartEditor {
    /// Create a new part editor and optionally initialize it.
    fn new(part: Option<WorkPart>, handle: NavigationHandle<WorkPart>) -> Rc<Self> {
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
            handle,
            widget,
            title_entry,
            composer_row,
            reset_composer_button,
            composer: RefCell::new(composer),
        });

        // Connect signals and callbacks

        back_button.connect_clicked(clone!(@weak this => move |_| {
            this.handle.pop(None);
        }));

        save_button.connect_clicked(clone!(@weak this => move |_| {
            let part = WorkPart {
                title: this.title_entry.get_text().unwrap().to_string(),
                composer: this.composer.borrow().clone(),
            };

            this.handle.pop(Some(part));
        }));

        composer_button.connect_clicked(clone!(@strong this => move |_| {
            spawn!(@clone this, async move {
                if let Some(person) = push!(this.handle, PersonSelector).await {
                    this.show_composer(Some(&person));
                    this.composer.replace(Some(person.to_owned()));
                }
            });
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
}

impl WorkPartEditor {
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

impl Widget for WorkPartEditor {
    fn get_widget(&self) -> gtk::Widget {
        self.widget.clone().upcast()
    }
}
