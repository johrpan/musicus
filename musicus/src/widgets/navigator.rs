use glib::clone;
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

pub trait NavigatorScreen {
    fn attach_navigator(&self, navigator: Rc<Navigator>);
    fn get_widget(&self) -> gtk::Widget;
    fn detach_navigator(&self);
}

pub struct Navigator {
    pub window: gtk::Window,
    pub widget: gtk::Stack,
    screens: RefCell<Vec<Rc<dyn NavigatorScreen>>>,
    old_screens: RefCell<Vec<Rc<dyn NavigatorScreen>>>,
    back_cb: RefCell<Option<Box<dyn Fn() -> ()>>>,
}

impl Navigator {
    pub fn new<W, S>(window: &W, empty_screen: &S) -> Rc<Self>
    where
        W: IsA<gtk::Window>,
        S: IsA<gtk::Widget>,
    {
        let widget = gtk::Stack::new();
        widget.set_hhomogeneous(false);
        widget.set_vhomogeneous(false);
        widget.set_interpolate_size(true);
        widget.set_transition_type(gtk::StackTransitionType::Crossfade);
        widget.set_hexpand(true);
        widget.add_named(empty_screen, "empty_screen");
        widget.show();

        let result = Rc::new(Self {
            window: window.clone().upcast(),
            widget,
            screens: RefCell::new(Vec::new()),
            old_screens: RefCell::new(Vec::new()),
            back_cb: RefCell::new(None),
        });

        unsafe {
            result.widget.connect_notify_unsafe(
                Some("transition-running"),
                clone!(@strong result => move |_, _| {
                    if !result.widget.get_transition_running() {
                        result.clear_old_screens();
                    }
                }),
            );
        }

        result
    }

    pub fn set_back_cb<F>(&self, cb: F)
    where
        F: Fn() -> () + 'static,
    {
        self.back_cb.replace(Some(Box::new(cb)));
    }

    pub fn push<S>(self: Rc<Self>, screen: Rc<S>)
    where
        S: NavigatorScreen + 'static,
    {
        if let Some(screen) = self.screens.borrow().last() {
            screen.detach_navigator();
        }

        let widget = screen.get_widget();
        self.widget.add(&widget);
        self.widget.set_visible_child(&widget);

        screen.attach_navigator(self.clone());
        self.screens.borrow_mut().push(screen);
    }

    pub fn pop(self: Rc<Self>) {
        let popped = if let Some(screen) = self.screens.borrow_mut().pop() {
            screen.detach_navigator();
            self.old_screens.borrow_mut().push(screen);

            true
        } else {
            false
        };

        if popped {
            if let Some(screen) = self.screens.borrow().last() {
                let widget = screen.get_widget();
                self.widget.set_visible_child(&widget);

                screen.attach_navigator(self.clone());
            } else {
                self.widget.set_visible_child_name("empty_screen");
                if let Some(cb) = &*self.back_cb.borrow() {
                    cb()
                }
            }

            if !self.widget.get_transition_running() {
                self.clear_old_screens();
            }
        }
    }

    pub fn replace<S>(self: Rc<Self>, screen: Rc<S>)
    where
        S: NavigatorScreen + 'static,
    {
        for screen in self.screens.replace(Vec::new()) {
            screen.detach_navigator();
            self.old_screens.borrow_mut().push(screen);
        }

        let widget = screen.get_widget();
        self.widget.add(&widget);
        self.widget.set_visible_child(&widget);

        screen.attach_navigator(self.clone());
        self.screens.borrow_mut().push(screen);

        if !self.widget.get_transition_running() {
            self.clear_old_screens();
        }
    }

    pub fn reset(&self) {
        for screen in self.screens.replace(Vec::new()) {
            screen.detach_navigator();
            self.old_screens.borrow_mut().push(screen);
        }

        if !self.widget.get_transition_running() {
            self.clear_old_screens();
        }
    }

    fn clear_old_screens(&self) {
        for screen in self.old_screens.borrow().iter() {
            self.widget.remove(&screen.get_widget());
        }

        self.old_screens.borrow_mut().clear();
    }
}
