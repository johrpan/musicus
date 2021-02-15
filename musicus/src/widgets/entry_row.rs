use gtk::prelude::*;
use libadwaita::prelude::*;

/// A list box row with an entry.
pub struct EntryRow {
    /// The actual GTK widget.
    pub widget: libadwaita::ActionRow,

    /// The managed entry.
    entry: gtk::Entry,
}

impl EntryRow {
    /// Create a new entry row.
    pub fn new(title: &str) -> Self {
        let entry = gtk::EntryBuilder::new()
            .hexpand(true)
            .valign(gtk::Align::Center)
            .build();

        let widget = libadwaita::ActionRowBuilder::new()
            .activatable(true)
            .activatable_widget(&entry)
            .title(title)
            .build();

        widget.add_suffix(&entry);

        Self {
            widget,
            entry,
        }
    }

    /// Set the text of the entry.
    pub fn set_text(&self, text: &str) {
        self.entry.set_text(text);
    }

    /// Get the text that was entered by the user.
    pub fn get_text(&self) -> String {
        self.entry.get_text().to_string()
    }
}
