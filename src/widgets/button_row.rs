use super::Widget;
use gtk::prelude::*;
use libadwaita::prelude::*;

/// A list box row with a single button.
pub struct ButtonRow {
    /// The actual GTK widget.
    pub widget: libadwaita::ActionRow,

    /// The managed button.
    button: gtk::Button,
}

impl ButtonRow {
    /// Create a new button row.
    pub fn new(title: &str, label: &str) -> Self {
        let button = gtk::ButtonBuilder::new()
            .valign(gtk::Align::Center)
            .label(label)
            .build();

        let widget = libadwaita::ActionRowBuilder::new()
            .activatable(true)
            .activatable_widget(&button)
            .title(title)
            .build();

        widget.add_suffix(&button);

        Self {
            widget,
            button,
        }
    }

    /// Set the subtitle of the row.
    pub fn set_subtitle(&self, subtitle: Option<&str>) {
        self.widget.set_subtitle(subtitle);
    }

    /// Set the closure to be called on activation
    pub fn set_cb<F: Fn() + 'static>(&self, cb: F) {
        self.button.connect_clicked(move |_| cb());
    }
}

impl Widget for ButtonRow {
    fn get_widget(&self) -> gtk::Widget {
        self.widget.clone().upcast()
    }
}
