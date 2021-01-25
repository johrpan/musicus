use super::indexed_list_model::{IndexedListModel, ItemIndex};
use glib::clone;
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// A simple list of widgets.
pub struct List {
    pub widget: gtk::ListBox,
    model: IndexedListModel,
    filter: gtk::CustomFilter,
    make_widget_cb: RefCell<Option<Box<dyn Fn(usize) -> gtk::Widget>>>,
    filter_cb: RefCell<Option<Box<dyn Fn(usize) -> bool>>>,
}

impl List {
    /// Create a new list. The list will be empty initially.
    pub fn new() -> Rc<Self> {
        let model = IndexedListModel::new();
        let filter = gtk::CustomFilter::new(|_| true);
        let filter_model = gtk::FilterListModel::new(Some(&model), Some(&filter));

        // TODO: Switch to gtk::ListView.
        // let selection = gtk::NoSelection::new(Some(&model));
        // let factory = gtk::SignalListItemFactory::new();
        // let widget = gtk::ListView::new(Some(&selection), Some(&factory));

        let widget = gtk::ListBox::new();

        let this = Rc::new(Self {
            widget,
            model,
            filter,
            make_widget_cb: RefCell::new(None),
            filter_cb: RefCell::new(None),
        });

        this.filter.set_filter_func(clone!(@strong this => move |index| {
            if let Some(cb) = &*this.filter_cb.borrow() {
                let index = index.downcast_ref::<ItemIndex>().unwrap().get() as usize;
                cb(index)
            } else {
                true
            }
        }));

        this.widget.bind_model(Some(&filter_model), clone!(@strong this => move |index| {
            let index = index.downcast_ref::<ItemIndex>().unwrap().get() as usize;
            if let Some(cb) = &*this.make_widget_cb.borrow() {
                cb(index)
            } else {
                gtk::Label::new(None).upcast()
            }
        }));

        this
    }

    /// Set the closure to be called to construct widgets for the items.
    pub fn set_make_widget_cb<F: Fn(usize) -> gtk::Widget + 'static>(&self, cb: F) {
        self.make_widget_cb.replace(Some(Box::new(cb)));
    }

    /// Set the closure to be called to filter the items. If this returns
    /// false, the item will not be shown.
    pub fn set_filter_cb<F: Fn(usize) -> bool + 'static>(&self, cb: F) {
        self.filter_cb.replace(Some(Box::new(cb)));
    }

    /// Select an item by its index. If the index is out of range, nothing will happen.
    pub fn select(&self, index: usize) {
        let row = self.widget.get_row_at_index(index as i32);
        if let Some(row) = row {
            self.widget.select_row(Some(&row));
        }
    }

    /// Refilter the list based on the filter callback.
    pub fn invalidate_filter(&self) {
        self.filter.changed(gtk::FilterChange::Different);
    }

    /// Call the make_widget function for each item. This will automatically
    /// show all children by indices 0..length.
    pub fn update(&self, length: usize) {
        self.model.set_length(length as u32);
    }
}
