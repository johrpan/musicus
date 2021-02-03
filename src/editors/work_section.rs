use crate::database::*;
use crate::navigator::{NavigationHandle, Screen};
use crate::widgets::Widget;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use std::cell::RefCell;
use std::rc::Rc;

/// A dialog for creating or editing a work section.
pub struct WorkSectionEditor {
    handle: NavigationHandle<WorkSection>,
    widget: gtk::Box,
    title_entry: gtk::Entry,
}

impl Screen<Option<WorkSection>, WorkSection> for  WorkSectionEditor {
    /// Create a new section editor and optionally initialize it.
    fn new(section: Option<WorkSection>, handle: NavigationHandle<WorkSection>) -> Rc<Self> {
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
            handle,
            widget,
            title_entry,
        });

        // Connect signals and callbacks

        back_button.connect_clicked(clone!(@weak this => move |_| {
            this.handle.pop(None);
        }));

        save_button.connect_clicked(clone!(@weak this => move |_| {
            let section = WorkSection {
                before_index: 0,
                title: this.title_entry.get_text().unwrap().to_string(),
            };

            this.handle.pop(Some(section));
        }));

        this
    }
}

impl Widget for WorkSectionEditor {
    fn get_widget(&self) -> gtk::Widget {
        self.widget.clone().upcast()
    }
}
