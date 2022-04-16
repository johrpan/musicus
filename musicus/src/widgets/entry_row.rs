use adw::{prelude::*, builders::ActionRowBuilder};
use gtk::builders::EntryBuilder;

/// A list box row with an entry.
pub struct EntryRow {
    /// The actual GTK widget.
    pub widget: adw::ActionRow,

    /// The managed entry.
    pub entry: gtk::Entry,
}

impl EntryRow {
    /// Create a new entry row.
    pub fn new(title: &str) -> Self {
        let entry = EntryBuilder::new()
            .hexpand(true)
            .valign(gtk::Align::Center)
            .build();

        let widget = ActionRowBuilder::new()
            .focusable(false)
            .activatable_widget(&entry)
            .title(title)
            .build();

        widget.add_suffix(&entry);

        Self { widget, entry }
    }

    /// Set the text of the entry.
    pub fn set_text(&self, text: &str) {
        self.entry.set_text(text);
    }

    /// Get the text that was entered by the user.
    pub fn get_text(&self) -> String {
        self.entry.text().to_string()
    }
}
