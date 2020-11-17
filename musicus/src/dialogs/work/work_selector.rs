use super::work_selector_person_screen::*;
use crate::backend::Backend;
use crate::database::*;
use crate::widgets::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use libhandy::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// A widget for selecting a work from a list of existing ones.
pub struct WorkSelector {
    pub widget: libhandy::Leaflet,
    backend: Rc<Backend>,
    sidebar_box: gtk::Box,
    selected_cb: RefCell<Option<Box<dyn Fn(Work) -> ()>>>,
    add_cb: RefCell<Option<Box<dyn Fn() -> ()>>>,
    navigator: Rc<Navigator>,
}

impl WorkSelector {
    /// Create a new work selector.
    pub fn new(backend: Rc<Backend>) -> Rc<Self> {
        // Create UI

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/work_selector.ui");

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
            let person_screen = WorkSelectorPersonScreen::new(
                this.backend.clone(),
                person.clone(),
            );

            person_screen.set_selected_cb(clone!(@strong this => move |work| {
                if let Some(cb) = &*this.selected_cb.borrow() {
                    cb(work);
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

    /// Set the closure to be called if the user wants to add a new work.
    pub fn set_add_cb<F: Fn() -> () + 'static>(&self, cb: F) {
        self.add_cb.replace(Some(Box::new(cb)));
    }

    /// Set the closure to be called when the user has selected a work.
    pub fn set_selected_cb<F: Fn(Work) -> () + 'static>(&self, cb: F) {
        self.selected_cb.replace(Some(Box::new(cb)));
    }
}
