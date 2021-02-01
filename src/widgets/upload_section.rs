use super::Section;

use gettextrs::gettext;
use gtk::prelude::*;
use libadwaita::prelude::*;

/// A section showing a switch to enable uploading an item.
pub struct UploadSection {
    /// The GTK widget of the wrapped section.
    pub widget: gtk::Box,

    /// The section itself.
    section: Section,

    /// The upload switch.
    switch: gtk::Switch,
}

impl UploadSection {
    /// Create a new upload section which will be initially switched on.
    pub fn new() -> Self {
        let list = gtk::ListBoxBuilder::new()
            .selection_mode(gtk::SelectionMode::None)
            .build();

        let switch = gtk::SwitchBuilder::new()
            .active(true)
            .valign(gtk::Align::Center)
            .build();

        let row = libadwaita::ActionRowBuilder::new()
            .title("Upload changes to the server")
            .activatable(true)
            .activatable_widget(&switch)
            .build();

        row.add_suffix(&switch);
        list.append(&row);

        let section = Section::new(&gettext("Upload"), &list);

        Self {
            widget: section.widget.clone(),
            section,
            switch,
        }
    }

    /// Return whether the user has enabled the upload switch.
    pub fn get_active(&self) -> bool {
        self.switch.get_active()
    }
}
