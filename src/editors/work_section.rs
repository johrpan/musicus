use crate::database::*;
use crate::widgets::{Navigator, NavigatorScreen};
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use std::cell::RefCell;
use std::rc::Rc;

/// A dialog for creating or editing a work section.
pub struct WorkSectionEditor {
    widget: gtk::Box,
    title_entry: gtk::Entry,
    ready_cb: RefCell<Option<Box<dyn Fn(WorkSection) -> ()>>>,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl WorkSectionEditor {
    /// Create a new section editor and optionally initialize it.
    pub fn new(section: Option<WorkSection>) -> Rc<Self> {
        // Create UI

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/work_section_editor.ui");

        get_widget!(builder, gtk::Box, widget);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::Button, save_button);
        get_widget!(builder, gtk::Entry, title_entry);

        if let Some(section) = section {
            title_entry.set_text(&section.title);
        }

        let this = Rc::new(Self {
            widget,
            title_entry,
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
                cb(WorkSection {
                    before_index: 0,
                    title: this.title_entry.get_text().unwrap().to_string(),
                });
            }

            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                navigator.pop();
            }
        }));

        this
    }

    /// Set the closure to be called when the user wants to save the section. Note that the
    /// resulting object will always have `before_index` set to 0. The caller is expected to
    /// change that later before adding the section to the database.
    pub fn set_ready_cb<F: Fn(WorkSection) -> () + 'static>(&self, cb: F) {
        self.ready_cb.replace(Some(Box::new(cb)));
    }
}

impl NavigatorScreen for WorkSectionEditor {
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
