use glib::clone;
use gtk::prelude::*;
use std::cell::RefCell;

pub struct Stack {
    pub widget: gtk::Stack,
    old_children: RefCell<Vec<gtk::Widget>>,
    current_child: RefCell<Option<gtk::Widget>>,
}

impl Stack {
    pub fn new<W>(empty_screen: &W) -> Self
    where
        W: IsA<gtk::Widget>,
    {
        let old_children = RefCell::new(Vec::new());

        let widget = gtk::Stack::new();
        widget.set_transition_type(gtk::StackTransitionType::Crossfade);
        widget.set_hexpand(true);
        widget.add_named(empty_screen, "empty_screen");

        unsafe {
            widget.connect_notify_unsafe(
                Some("transition-running"),
                clone!(@strong old_children => move |stack, _| {
                    for child in old_children.borrow().iter() {
                        stack.remove(child);
                    }

                    old_children.borrow_mut().clear();
                }),
            );
        }

        widget.show();

        Self {
            widget: widget.clone(),
            old_children,
            current_child: RefCell::new(None),
        }
    }

    pub fn set_child<W>(&self, child: W)
    where
        W: IsA<gtk::Widget>,
    {
        if let Some(child) = self.current_child.borrow_mut().take() {
            self.old_children.borrow_mut().push(child);
        }

        self.current_child.replace(Some(child.clone().upcast()));
        self.widget.add(&child);
        self.widget.set_visible_child(&child);

        if !self.widget.get_transition_running() {
            for child in self.old_children.borrow().iter() {
                self.widget.remove(child);
            }

            self.old_children.borrow_mut().clear();
        }
    }

    pub fn reset_child(&self) {
        self.widget.set_visible_child_name("empty_screen");

        if !self.widget.get_transition_running() {
            for child in self.old_children.borrow().iter() {
                self.widget.remove(child);
            }

            self.old_children.borrow_mut().clear();
        }
    }
}
