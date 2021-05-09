use super::Section;
use adw::prelude::*;
use gettextrs::gettext;
use glib::clone;
use musicus_backend::Backend;
use std::rc::Rc;

/// A section showing a switch to enable uploading an item.
pub struct UploadSection {
    /// The GTK widget of the wrapped section.
    pub widget: gtk::Box,

    backend: Rc<Backend>,

    /// The upload switch.
    switch: gtk::Switch,
}

impl UploadSection {
    /// Create a new upload section which will be initially switched on.
    pub fn new(backend: Rc<Backend>) -> Rc<Self> {
        let list = gtk::ListBoxBuilder::new()
            .selection_mode(gtk::SelectionMode::None)
            .build();

        let switch = gtk::SwitchBuilder::new()
            .active(backend.use_server())
            .valign(gtk::Align::Center)
            .build();

        let row = adw::ActionRowBuilder::new()
            .title("Upload changes to the server")
            .activatable(true)
            .activatable_widget(&switch)
            .build();

        row.add_suffix(&switch);
        list.append(&row);

        let section = Section::new(&gettext("Upload"), &list);

        let this = Rc::new(Self {
            widget: section.widget,
            backend,
            switch,
        });

        this.switch
            .connect_state_notify(clone!(@weak this =>  move |_| {
                this.backend.set_use_server(this.switch.state());
            }));

        this
    }

    /// Return whether the user has enabled the upload switch.
    pub fn get_active(&self) -> bool {
        self.switch.state()
    }
}
