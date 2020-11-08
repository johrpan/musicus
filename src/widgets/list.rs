use super::*;
use glib::clone;
use gtk::prelude::*;
use std::cell::RefCell;
use std::convert::TryInto;
use std::rc::Rc;

pub struct List<T>
where
    T: 'static,
{
    pub widget: gtk::ListBox,
    items: RefCell<Vec<T>>,
    make_widget: RefCell<Option<Box<dyn Fn(&T) -> gtk::Widget>>>,
    filter: RefCell<Option<Box<dyn Fn(&T) -> bool>>>,
    selected: RefCell<Option<Box<dyn Fn(&T) -> ()>>>,
}

impl<T> List<T>
where
    T: 'static,
{
    pub fn new(placeholder_text: &str) -> Rc<Self> {
        let placeholder_label = gtk::Label::new(Some(placeholder_text));
        placeholder_label.set_margin_top(6);
        placeholder_label.set_margin_bottom(6);
        placeholder_label.set_margin_start(6);
        placeholder_label.set_margin_end(6);
        placeholder_label.show();

        let widget = gtk::ListBox::new();
        widget.set_placeholder(Some(&placeholder_label));
        widget.show();

        let this = Rc::new(Self {
            widget,
            items: RefCell::new(Vec::new()),
            make_widget: RefCell::new(None),
            filter: RefCell::new(None),
            selected: RefCell::new(None),
        });

        this.widget
            .connect_row_activated(clone!(@strong this => move |_, row| {
                if let Some(selected) = &*this.selected.borrow() {
                    let row = row.get_child().unwrap().downcast::<SelectorRow>().unwrap();
                    let index: usize = row.get_index().try_into().unwrap();
                    selected(&this.items.borrow()[index]);
                }
            }));

        this.widget
            .set_filter_func(Some(Box::new(clone!(@strong this => move |row| {
                if let Some(filter) = &*this.filter.borrow() {
                    let row = row.get_child().unwrap().downcast::<SelectorRow>().unwrap();
                    let index: usize = row.get_index().try_into().unwrap();
                    filter(&this.items.borrow()[index])
                } else {
                    true
                }
            }))));

        this
    }

    pub fn set_make_widget<F: Fn(&T) -> gtk::Widget + 'static>(&self, make_widget: F) {
        self.make_widget.replace(Some(Box::new(make_widget)));
    }

    pub fn set_filter<F: Fn(&T) -> bool + 'static>(&self, filter: F) {
        self.filter.replace(Some(Box::new(filter)));
    }

    pub fn set_selected<S: Fn(&T) -> () + 'static>(&self, selected: S) {
        self.selected.replace(Some(Box::new(selected)));
    }

    pub fn get_selected_index(&self) -> Option<usize> {
        match self.widget.get_selected_rows().first() {
            Some(row) => match row.get_child() {
                Some(child) => Some(
                    child
                        .downcast::<SelectorRow>()
                        .unwrap()
                        .get_index()
                        .try_into()
                        .unwrap(),
                ),
                None => None,
            },
            None => None,
        }
    }

    pub fn select_index(&self, index: usize) {
        self.widget.select_row(
            self.widget
                .get_row_at_index(index.try_into().unwrap())
                .as_ref(),
        );
    }

    pub fn show_items(&self, items: Vec<T>) {
        self.items.replace(items);
        self.update();
    }

    pub fn invalidate_filter(&self) {
        self.widget.invalidate_filter();
    }

    pub fn update(&self) {
        for child in self.widget.get_children() {
            self.widget.remove(&child);
        }

        if let Some(make_widget) = &*self.make_widget.borrow() {
            for (index, item) in self.items.borrow().iter().enumerate() {
                let row = SelectorRow::new(index.try_into().unwrap(), &make_widget(item));
                row.show_all();
                self.widget.insert(&row, -1);
            }
        }
    }

    pub fn clear_selection(&self) {
        self.widget.unselect_all();
    }
}
