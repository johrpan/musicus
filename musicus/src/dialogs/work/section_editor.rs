use crate::database::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use std::cell::RefCell;
use std::rc::Rc;

/// A dialog for creating or editing a work section.
pub struct SectionEditor {
    window: libhandy::Window,
    title_entry: gtk::Entry,
    ready_cb: RefCell<Option<Box<dyn Fn(WorkSection) -> ()>>>,
}

impl SectionEditor {
    /// Create a new section editor and optionally initialize it.
    pub fn new<P: IsA<gtk::Window>>(parent: &P, section: Option<WorkSection>) -> Rc<Self> {
        // Create UI

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/section_editor.ui");

        get_widget!(builder, libhandy::Window, window);
        get_widget!(builder, gtk::Button, cancel_button);
        get_widget!(builder, gtk::Button, save_button);
        get_widget!(builder, gtk::Entry, title_entry);

        window.set_transient_for(Some(parent));

        if let Some(section) = section {
            title_entry.set_text(&section.title);
        }

        let this = Rc::new(Self {
            window,
            title_entry,
            ready_cb: RefCell::new(None),
        });

        // Connect signals and callbacks

        cancel_button.connect_clicked(clone!(@strong this => move |_| {
            this.window.close();
        }));

        save_button.connect_clicked(clone!(@strong this => move |_| {
            if let Some(cb) = &*this.ready_cb.borrow() {
                cb(WorkSection {
                    before_index: 0,
                    title: this.title_entry.get_text().to_string(),
                });
            }

            this.window.close();
        }));

        this
    }

    /// Set the closure to be called when the user wants to save the section. Note that the
    /// resulting object will always have `before_index` set to 0. The caller is expected to
    /// change that later before adding the section to the database.
    pub fn set_ready_cb<F: Fn(WorkSection) -> () + 'static>(&self, cb: F) {
        self.ready_cb.replace(Some(Box::new(cb)));
    }

    /// Show the section editor.
    pub fn show(&self) {
        self.window.show();
    }
}
