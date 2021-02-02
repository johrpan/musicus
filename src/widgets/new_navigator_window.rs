use crate::backend::Backend;
use super::new_navigator::{Navigator, Screen};
use glib::clone;
use gtk::prelude::*;
use std::rc::Rc;

/// A window hosting a navigator.
pub struct NavigatorWindow {
    pub navigator: Rc<Navigator>,
    window: libadwaita::Window,
}

impl NavigatorWindow {
    /// Create a new navigator window.
    pub fn new(backend: Rc<Backend>) -> Rc<Self> {
        let window = libadwaita::Window::new();
        window.set_default_size(600, 424);
        let placeholder = gtk::Label::new(None);
        let navigator = Navigator::new(backend, &window, &placeholder);
        libadwaita::WindowExt::set_child(&window, Some(&navigator.widget));

        let this = Rc::new(Self { navigator, window });

        this.navigator.set_back_cb(clone!(@strong this => move || {
            this.window.close();
        }));

        this
    }

    /// Make the wrapped window transient. This will make the window modal.
    pub fn set_transient_for<W: IsA<gtk::Window>>(&self, window: &W) {
        self.window.set_modal(true);
        self.window.set_transient_for(Some(window));
    }

    /// Show the navigator window.
    pub fn show(&self) {
        self.window.show();
    }
}