use crate::widgets::{Navigator, NavigatorScreen};
use glib::clone;
use gtk::prelude::*;
use std::rc::Rc;

/// A window hosting a navigator.
pub struct NavigatorWindow {
    window: libhandy::Window,
    navigator: Rc<Navigator>,
}

impl NavigatorWindow {
    /// Create a new navigator window showing an initial screen.
    pub fn new<S: NavigatorScreen + 'static>(initial_screen: Rc<S>) -> Rc<Self> {
        // Create UI

        let window = libhandy::Window::new();
        window.set_default_size(600, 424);
        let placeholder = gtk::Label::new(None);
        let navigator = Navigator::new(&window, &placeholder);
        libhandy::WindowExt::set_child(&window, Some(&navigator.widget));

        let this = Rc::new(Self { window, navigator });

        // Connect signals and callbacks

        this.navigator.set_back_cb(clone!(@strong this => move || {
            this.window.close();
        }));

        // Initialize

        this.navigator.clone().replace(initial_screen);

        this
    }

    /// Show the navigator window.
    pub fn show(&self) {
        self.window.show();
    }
}
