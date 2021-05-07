use super::indexed_list_model::{IndexedListModel, ItemIndex};
use glib::clone;
use gtk::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A simple list of widgets.
pub struct List {
    pub widget: gtk::ListBox,
    model: IndexedListModel,
    filter: gtk::CustomFilter,
    enable_dnd: Cell<bool>,
    make_widget_cb: RefCell<Option<Box<dyn Fn(usize) -> gtk::Widget>>>,
    filter_cb: RefCell<Option<Box<dyn Fn(usize) -> bool>>>,
    move_cb: RefCell<Option<Box<dyn Fn(usize, usize)>>>,
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
        widget.set_selection_mode(gtk::SelectionMode::None);

        let this = Rc::new(Self {
            widget,
            model,
            filter,
            enable_dnd: Cell::new(false),
            make_widget_cb: RefCell::new(None),
            filter_cb: RefCell::new(None),
            move_cb: RefCell::new(None),
        });

        this.filter
            .set_filter_func(clone!(@strong this => move |index| {
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
                let widget = cb(index);

                if this.enable_dnd.get() {
                    let drag_source = gtk::DragSource::new();

                    drag_source.connect_drag_begin(clone!(@strong widget => move |_, drag| {
                        // TODO: Replace with a better solution.
                        let paintable = gtk::WidgetPaintable::new(Some(&widget));
                        gtk::DragIcon::set_from_paintable(drag, &paintable, 0, 0);
                    }));

                    let drag_value = (index as u32).to_value();
                    drag_source.set_content(Some(&gdk::ContentProvider::for_value(&drag_value)));

                    let drop_target = gtk::DropTarget::new(glib::Type::U32, gdk::DragAction::COPY);

                    drop_target.connect_drop(clone!(@strong this => move |_, value, _, _| {
                        if let Some(cb) = &*this.move_cb.borrow() {
                            let old_index: u32 = value.get().unwrap();
                            cb(old_index as usize, index);
                            true
                        } else {
                            false
                        }
                    }));

                    widget.add_controller(&drag_source);
                    widget.add_controller(&drop_target);
                }

                widget
            } else {
                // This shouldn't be reachable under normal circumstances.
                gtk::Label::new(None).upcast()
            }
        }));

        this
    }

    /// Whether the list should support drag and drop.
    pub fn set_enable_dnd(&self, enable: bool) {
        self.enable_dnd.set(enable);
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

    /// Set the closure to be called to when the use has dragged an item to a
    /// new position.
    pub fn set_move_cb<F: Fn(usize, usize) + 'static>(&self, cb: F) {
        self.move_cb.replace(Some(Box::new(cb)));
    }

    /// Set the lists selection mode to single.
    pub fn enable_selection(&self) {
        self.widget.set_selection_mode(gtk::SelectionMode::Single);
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
