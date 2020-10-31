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
    make_widget: Box<dyn Fn(&T) -> gtk::Widget + 'static>,
    selected: RefCell<Option<Box<dyn Fn(&T) -> () + 'static>>>,
}

impl<T> List<T>
where
    T: 'static,
{
    pub fn new<M, F>(make_widget: M, filter: F, placeholder_text: &str) -> Rc<Self>
    where
        M: Fn(&T) -> gtk::Widget + 'static,
        F: Fn(&T) -> bool + 'static,
    {
        let placeholder_label = gtk::Label::new(Some(placeholder_text));
        placeholder_label.set_margin_top(6);
        placeholder_label.set_margin_bottom(6);
        placeholder_label.set_margin_start(6);
        placeholder_label.set_margin_end(6);
        placeholder_label.show();

        let widget = gtk::ListBox::new();
        widget.set_placeholder(Some(&placeholder_label));
        widget.show();

        let result = Rc::new(Self {
            widget,
            items: RefCell::new(Vec::new()),
            make_widget: Box::new(make_widget),
            selected: RefCell::new(None),
        });

        result
            .widget
            .connect_row_activated(clone!(@strong result => move |_, row| {
                if let Some(selected) = &*result.selected.borrow() {
                    let row = row.get_child().unwrap().downcast::<SelectorRow>().unwrap();
                    let index: usize = row.get_index().try_into().unwrap();
                    selected(&result.items.borrow()[index]);
                }
            }));

        result
            .widget
            .set_filter_func(Some(Box::new(clone!(@strong result => move |row| {
                let row = row.get_child().unwrap().downcast::<SelectorRow>().unwrap();
                let index: usize = row.get_index().try_into().unwrap();
                filter(&result.items.borrow()[index])
            }))));

        result
    }

    pub fn set_selected<S>(&self, selected: S)
    where
        S: Fn(&T) -> () + 'static,
    {
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
        self.widget.select_row(self.widget.get_row_at_index(index.try_into().unwrap()).as_ref());
    }

    pub fn show_items(&self, items: Vec<T>) {
        self.items.replace(items);

        for child in self.widget.get_children() {
            self.widget.remove(&child);
        }

        for (index, item) in self.items.borrow().iter().enumerate() {
            let row = SelectorRow::new(index.try_into().unwrap(), &(self.make_widget)(item));
            row.show_all();
            self.widget.insert(&row, -1);
        }
    }

    pub fn invalidate_filter(&self) {
        self.widget.invalidate_filter();
    }

    pub fn clear_selection(&self) {
        self.widget.unselect_all();
    }
}
