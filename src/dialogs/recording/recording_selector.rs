use super::recording_selector_person_screen::*;
use crate::backend::Backend;
use crate::database::*;
use crate::widgets::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use libhandy::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// A widget for selecting a recording from a list of existing ones.
pub struct RecordingSelector {
    pub widget: libhandy::Leaflet,
    backend: Rc<Backend>,
    sidebar_box: gtk::Box,
    selected_cb: RefCell<Option<Box<dyn Fn(RecordingDescription) -> ()>>>,
    add_cb: RefCell<Option<Box<dyn Fn() -> ()>>>,
    navigator: Rc<Navigator>,
}

impl RecordingSelector {
    /// Create a new recording selector.
    pub fn new(backend: Rc<Backend>) -> Rc<Self> {
        // Create UI

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/recording_selector.ui");

        get_widget!(builder, libhandy::Leaflet, widget);
        get_widget!(builder, gtk::Button, add_button);
        get_widget!(builder, gtk::Box, sidebar_box);
        get_widget!(builder, gtk::Box, empty_screen);

        let person_list = PersonList::new(backend.clone());
        sidebar_box.pack_start(&person_list.widget, true, true, 0);

        let navigator = Navigator::new(&empty_screen);
        widget.add(&navigator.widget);

        let this = Rc::new(Self {
            widget,
            backend,
            sidebar_box,
            selected_cb: RefCell::new(None),
            add_cb: RefCell::new(None),
            navigator,
        });

        // Connect signals and callbacks

        add_button.connect_clicked(clone!(@strong this => move |_| {
            if let Some(cb) = &*this.add_cb.borrow() {
                cb();
            }
        }));

        person_list.set_selected(clone!(@strong this => move |person| {
            let person_screen = RecordingSelectorPersonScreen::new(
                this.backend.clone(),
                person.clone(),
            );

            person_screen.set_selected_cb(clone!(@strong this => move |recording| {
                if let Some(cb) = &*this.selected_cb.borrow() {
                    cb(recording);
                }
            }));

            this.navigator.clone().push(person_screen);
            this.widget.set_visible_child(&this.navigator.widget);
        }));

        this.navigator.set_back_cb(clone!(@strong this => move || {
            this.widget.set_visible_child(&this.sidebar_box);
        }));

        this
    }

    /// Set the closure to be called if the editor is user wants to add a new recording.
    pub fn set_add_cb<F: Fn() -> () + 'static>(&self, cb: F) {
        self.add_cb.replace(Some(Box::new(cb)));
    }

    /// Set the closure to be called when the user has selected a recording.
    pub fn set_selected_cb<F: Fn(RecordingDescription) -> () + 'static>(&self, cb: F) {
        self.selected_cb.replace(Some(Box::new(cb)));
    }
}
