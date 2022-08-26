use crate::widgets::Widget;
use futures_channel::oneshot;
use futures_channel::oneshot::{Receiver, Sender};
use glib::clone;
use gtk::builders::StackBuilder;
use gtk::prelude::*;
use musicus_backend::Backend;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

pub mod window;
pub use window::*;

/// A widget that represents a logical unit of transient user interaction and
/// that optionally resolves to a specific return value.
pub trait Screen<I, O>: Widget {
    /// Create a new screen and initialize it with the provided input value.
    fn new(input: I, navigation_handle: NavigationHandle<O>) -> Rc<Self>
    where
        Self: Sized;
}

/// An accessor to navigation functionality for screens.
pub struct NavigationHandle<O> {
    /// The backend, in case the screen needs it.
    pub backend: Rc<Backend>,

    /// The toplevel window, in case the screen needs it.
    pub window: gtk::Window,

    /// The navigator that created this navigation handle.
    navigator: Weak<Navigator>,

    /// The sender through which the result should be sent.
    sender: Cell<Option<Sender<Option<O>>>>,
}

impl<O> NavigationHandle<O> {
    /// Switch to another screen and wait for that screen's result.
    pub async fn push<I, R, S: Screen<I, R> + 'static>(&self, input: I) -> Option<R> {
        let navigator = self.unwrap_navigator();
        let receiver = navigator.push::<I, R, S>(input);

        // If the sender is dropped, return None.
        receiver.await.unwrap_or(None)
    }

    /// Go back to the previous screen optionally returning something.
    pub fn pop(&self, output: Option<O>) {
        self.unwrap_navigator().pop();

        let sender = self
            .sender
            .take()
            .expect("Tried to send result from screen through a dropped sender.");

        if sender.send(output).is_err() {
            panic!("Tried to send result from screen to non-existing previous screen.");
        }
    }

    /// Get the navigator and panic if it doesn't exist.
    fn unwrap_navigator(&self) -> Rc<Navigator> {
        Weak::upgrade(&self.navigator)
            .expect("Tried to access non-existing navigator from a screen.")
    }
}

/// A toplevel widget for managing screens.
pub struct Navigator {
    /// The underlying GTK widget.
    pub widget: gtk::Stack,

    /// The backend, in case screens need it.
    backend: Rc<Backend>,

    /// The toplevel window of the navigator, in case screens need it.
    window: gtk::Window,

    /// The currently active screens. The last screen in this vector is the one
    /// that is currently visible.
    screens: RefCell<Vec<Rc<dyn Widget>>>,

    /// A vector holding the widgets of the old screens that are waiting to be
    /// removed after the animation has finished.
    old_widgets: RefCell<Vec<gtk::Widget>>,

    /// A closure that will be called when the last screen is popped.
    back_cb: RefCell<Option<Box<dyn Fn()>>>,
}

impl Navigator {
    /// Create a new navigator which will display the provided widget
    /// initially.
    pub fn new<W, E>(backend: Rc<Backend>, window: &W, empty_screen: &E) -> Rc<Self>
    where
        W: IsA<gtk::Window>,
        E: IsA<gtk::Widget>,
    {
        let widget = StackBuilder::new()
            .hhomogeneous(false)
            .vhomogeneous(false)
            .interpolate_size(true)
            .transition_type(gtk::StackTransitionType::Crossfade)
            .hexpand(true)
            .vexpand(true)
            .build();

        widget.add_named(empty_screen, Some("empty_screen"));

        let this = Rc::new(Self {
            widget,
            backend,
            window: window.to_owned().upcast(),
            screens: RefCell::new(Vec::new()),
            old_widgets: RefCell::new(Vec::new()),
            back_cb: RefCell::new(None),
        });

        this.widget
            .connect_transition_running_notify(clone!(@strong this => move |_| {
                if !this.widget.is_transition_running() {
                    this.clear_old_widgets();
                }
            }));

        this
    }

    /// Set the closure to be called when the last screen is popped so that
    /// the navigator shows its empty state.
    pub fn set_back_cb<F: Fn() + 'static>(&self, cb: F) {
        self.back_cb.replace(Some(Box::new(cb)));
    }

    /// Drop all screens and show the provided screen instead.
    pub async fn replace<I, O, S: Screen<I, O> + 'static>(self: &Rc<Self>, input: I) -> Option<O> {
        for screen in self.screens.replace(Vec::new()) {
            self.old_widgets.borrow_mut().push(screen.get_widget());
        }

        let receiver = self.push::<I, O, S>(input);

        if !self.widget.is_transition_running() {
            self.clear_old_widgets();
        }

        // We ignore the case, if a sender is dropped.
        receiver.await.unwrap_or(None)
    }

    /// Drop all screens and go back to the initial screen. The back callback
    /// will not be called.
    pub fn reset(&self) {
        self.widget.set_visible_child_name("empty_screen");

        for screen in self.screens.replace(Vec::new()) {
            self.old_widgets.borrow_mut().push(screen.get_widget());
        }

        if !self.widget.is_transition_running() {
            self.clear_old_widgets();
        }
    }

    /// Show a screen with the provided input. This should only be called from
    /// within a navigation handle.
    fn push<I, O, S: Screen<I, O> + 'static>(self: &Rc<Self>, input: I) -> Receiver<Option<O>> {
        let (sender, receiver) = oneshot::channel();

        let handle = NavigationHandle {
            backend: Rc::clone(&self.backend),
            window: self.window.clone(),
            navigator: Rc::downgrade(self),
            sender: Cell::new(Some(sender)),
        };

        let screen = S::new(input, handle);

        let widget = screen.get_widget();
        self.widget.add_child(&widget);
        self.widget.set_visible_child(&widget);

        self.screens.borrow_mut().push(screen);

        receiver
    }

    /// Pop the last screen from the list of screens.
    fn pop(&self) {
        let popped = if let Some(screen) = self.screens.borrow_mut().pop() {
            let widget = screen.get_widget();
            self.old_widgets.borrow_mut().push(widget);
            true
        } else {
            false
        };

        if popped {
            if let Some(screen) = self.screens.borrow().last() {
                let widget = screen.get_widget();
                self.widget.set_visible_child(&widget);
            } else {
                self.widget.set_visible_child_name("empty_screen");

                if let Some(cb) = &*self.back_cb.borrow() {
                    cb()
                }
            }

            if !self.widget.is_transition_running() {
                self.clear_old_widgets();
            }
        }
    }

    /// Drop the old widgets.
    fn clear_old_widgets(&self) {
        for widget in self.old_widgets.borrow().iter() {
            self.widget.remove(widget);
        }

        self.old_widgets.borrow_mut().clear();
    }
}
