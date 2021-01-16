use gtk::prelude::*;
use std::cell::RefCell;

/// A simple list of widgets.
pub struct List {
    pub widget: gtk::ListBox,
    make_widget: RefCell<Option<Box<dyn Fn(usize) -> gtk::Widget>>>,
}

impl List {
    /// Create a new list. The list will be empty.
    pub fn new(placeholder_text: &str) -> Self {
        let placeholder_label = gtk::Label::new(Some(placeholder_text));
        placeholder_label.set_margin_top(6);
        placeholder_label.set_margin_bottom(6);
        placeholder_label.set_margin_start(6);
        placeholder_label.set_margin_end(6);
        placeholder_label.show();

        let widget = gtk::ListBox::new();
        widget.set_selection_mode(gtk::SelectionMode::None);
        widget.set_placeholder(Some(&placeholder_label));
        widget.show();

        Self {
            widget,
            make_widget: RefCell::new(None),
        }
    }

    /// Set the closure to be called to construct widgets for the items.
    pub fn set_make_widget<F: Fn(usize) -> gtk::Widget + 'static>(&self, make_widget: F) {
        self.make_widget.replace(Some(Box::new(make_widget)));
    }

    /// Call the make_widget function for each item. This will automatically
    /// show all children by indices 0..length.
    pub fn update(&self, length: usize) {
        for child in self.widget.get_children() {
            self.widget.remove(&child);
        }

        if let Some(make_widget) = &*self.make_widget.borrow() {
            for index in 0..length {
                let row = make_widget(index);
                self.widget.insert(&row, -1);
            }
        }
    }
}
