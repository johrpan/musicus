use crate::widgets::{Navigator, NavigatorScreen};
use glib::clone;
use gtk::prelude::*;
use std::rc::Rc;

/// A window hosting a navigator.
pub struct NavigatorWindow {
    window: libadwaita::Window,
    navigator: Rc<Navigator>,
}

impl NavigatorWindow {
    /// Create a new navigator window showing an initial screen.
    pub fn new<S: NavigatorScreen + 'static>(initial_screen: Rc<S>) -> Rc<Self> {
        // Create UI

        let window = libadwaita::Window::new();
        window.set_default_size(600, 424);
        let placeholder = gtk::Label::new(None);
        let navigator = Navigator::new(&window, &placeholder);
        libadwaita::WindowExt::set_child(&window, Some(&navigator.widget));

        let this = Rc::new(Self { window, navigator });

        // Connect signals and callbacks

        this.navigator.set_back_cb(clone!(@strong this => move || {
            this.window.close();
        }));

        // Initialize

        this.navigator.clone().replace(initial_screen);

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
