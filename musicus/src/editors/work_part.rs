use crate::navigator::{NavigationHandle, Screen};
use crate::widgets::Widget;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use musicus_backend::db::WorkPart;
use std::rc::Rc;

/// A dialog for creating or editing a work section.
pub struct WorkPartEditor {
    handle: NavigationHandle<WorkPart>,
    widget: gtk::Box,
    save_button: gtk::Button,
    title_row: adw::EntryRow,
}

impl Screen<Option<WorkPart>, WorkPart> for WorkPartEditor {
    /// Create a new part editor and optionally initialize it.
    fn new(section: Option<WorkPart>, handle: NavigationHandle<WorkPart>) -> Rc<Self> {
        // Create UI

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/work_part_editor.ui");

        get_widget!(builder, gtk::Box, widget);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::Button, save_button);
        get_widget!(builder, adw::EntryRow, title_row);

        if let Some(section) = section {
            title_row.set_text(&section.title);
        }

        let this = Rc::new(Self {
            handle,
            widget,
            save_button,
            title_row,
        });

        // Connect signals and callbacks

        back_button.connect_clicked(clone!(@weak this =>  move |_| {
            this.handle.pop(None);
        }));

        this.save_button
            .connect_clicked(clone!(@weak this =>  move |_| {
                let section = WorkPart {
                    title: this.title_row.text().to_string(),
                };

                this.handle.pop(Some(section));
            }));

        this.title_row
            .connect_changed(clone!(@weak this =>  move |_| this.validate()));

        this.validate();

        this
    }
}

impl WorkPartEditor {
    /// Validate inputs and enable/disable saving.
    fn validate(&self) {
        self.save_button
            .set_sensitive(!self.title_row.text().is_empty());
    }
}

impl Widget for WorkPartEditor {
    fn get_widget(&self) -> gtk::Widget {
        self.widget.clone().upcast()
    }
}
