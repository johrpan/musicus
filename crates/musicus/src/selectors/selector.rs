use crate::widgets::List;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use musicus_backend::Result;
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

/// A screen that presents a list of items. It allows to switch between the server and the local
/// database and to search within the list.
pub struct Selector<T: 'static> {
    pub widget: gtk::Box,
    title_label: gtk::Label,
    subtitle_label: gtk::Label,
    search_entry: gtk::SearchEntry,
    server_check_button: gtk::CheckButton,
    stack: gtk::Stack,
    list: Rc<List>,
    items: RefCell<Vec<T>>,
    back_cb: RefCell<Option<Box<dyn Fn() -> ()>>>,
    add_cb: RefCell<Option<Box<dyn Fn() -> ()>>>,
    make_widget: RefCell<Option<Box<dyn Fn(&T) -> gtk::Widget>>>,
    load_online: RefCell<Option<Box<dyn Fn() -> Box<dyn Future<Output = Result<Vec<T>>>>>>>,
    load_local: RefCell<Option<Box<dyn Fn() -> Box<dyn Future<Output = Vec<T>>>>>>,
    filter: RefCell<Option<Box<dyn Fn(&str, &T) -> bool>>>,
}

impl<T> Selector<T> {
    /// Create a new selector.
    pub fn new() -> Rc<Self> {
        // Create UI

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/selector.ui");

        get_widget!(builder, gtk::Box, widget);
        get_widget!(builder, gtk::Label, title_label);
        get_widget!(builder, gtk::Label, subtitle_label);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::Button, add_button);
        get_widget!(builder, gtk::SearchEntry, search_entry);
        get_widget!(builder, gtk::CheckButton, server_check_button);
        get_widget!(builder, gtk::Stack, stack);
        get_widget!(builder, gtk::Frame, frame);
        get_widget!(builder, gtk::Button, try_again_button);

        let list = List::new();
        frame.set_child(Some(&list.widget));

        let this = Rc::new(Self {
            widget,
            title_label,
            subtitle_label,
            search_entry,
            server_check_button,
            stack,
            list,
            items: RefCell::new(Vec::new()),
            back_cb: RefCell::new(None),
            add_cb: RefCell::new(None),
            make_widget: RefCell::new(None),
            load_online: RefCell::new(None),
            load_local: RefCell::new(None),
            filter: RefCell::new(None),
        });

        // Connect signals and callbacks

        back_button.connect_clicked(clone!(@strong this => move |_| {
            if let Some(cb) = &*this.back_cb.borrow() {
                cb();
            }
        }));

        add_button.connect_clicked(clone!(@strong this => move |_| {
            if let Some(cb) = &*this.add_cb.borrow() {
                cb();
            }
        }));

        this.search_entry.connect_search_changed(clone!(@strong this => move |_| {
            this.list.invalidate_filter();
        }));

        this.server_check_button
            .connect_toggled(clone!(@strong this => move |_| {
                if this.server_check_button.get_active() {
                    this.clone().load_online();
                } else {
                    this.clone().load_local();
                }
            }));

        this.list.set_make_widget_cb(clone!(@strong this => move |index| {
            if let Some(cb) = &*this.make_widget.borrow() {
                let item = &this.items.borrow()[index];
                cb(item)
            } else {
                gtk::Label::new(None).upcast()
            }
        }));

        this.list.set_filter_cb(clone!(@strong this => move |index| {
            match &*this.filter.borrow() {
                Some(filter) => {
                    let item = &this.items.borrow()[index];
                    let search = this.search_entry.get_text().unwrap().to_string().to_lowercase();
                    search.is_empty() || filter(&search, item)
                }
                None => true,
            }
        }));

        try_again_button.connect_clicked(clone!(@strong this => move |_| {
            this.clone().load_online();
        }));

        // Initialize
        this.clone().load_online();

        this
    }

    /// Set the title to be shown in the header.
    pub fn set_title(&self, title: &str) {
        self.title_label.set_label(&title);
    }

    /// Set the subtitle to be shown in the header.
    pub fn set_subtitle(&self, subtitle: &str) {
        self.subtitle_label.set_label(&subtitle);
        self.subtitle_label.show();
    }

    /// Set the closure to be called when the user wants to go back.
    pub fn set_back_cb<F: Fn() -> () + 'static>(&self, cb: F) {
        self.back_cb.replace(Some(Box::new(cb)));
    }

    /// Set the closure to be called when the user wants to add an item.
    pub fn set_add_cb<F: Fn() -> () + 'static>(&self, cb: F) {
        self.add_cb.replace(Some(Box::new(cb)));
    }

    /// Set the async closure to be called to fetch items from the server. If that results in an
    /// error, an error screen is shown allowing to try again.
    pub fn set_load_online<F, R>(&self, cb: F)
    where
        F: (Fn() -> R) + 'static,
        R: Future<Output = Result<Vec<T>>> + 'static,
    {
        self.load_online
            .replace(Some(Box::new(move || Box::new(cb()))));
    }

    /// Set the async closure to be called to get local items.
    pub fn set_load_local<F, R>(&self, cb: F)
    where
        F: (Fn() -> R) + 'static,
        R: Future<Output = Vec<T>> + 'static,
    {
        self.load_local
            .replace(Some(Box::new(move || Box::new(cb()))));
    }

    /// Set the closure to be called for creating a new list row.
    pub fn set_make_widget<F: Fn(&T) -> gtk::Widget + 'static>(&self, make_widget: F) {
        self.make_widget.replace(Some(Box::new(make_widget)));
    }

    /// Set a closure to call when deciding whether to show an item based on a search string. The
    /// search string will be converted to lowercase.
    pub fn set_filter<F: Fn(&str, &T) -> bool + 'static>(&self, filter: F) {
        self.filter.replace(Some(Box::new(filter)));
    }

    fn load_online(self: Rc<Self>) {
        let context = glib::MainContext::default();
        let clone = self.clone();
        context.spawn_local(async move {
            if let Some(cb) = &*self.load_online.borrow() {
                self.stack.set_visible_child_name("loading");

                match Pin::from(cb()).await {
                    Ok(items) => {
                        clone.show_items(items);
                    }
                    Err(_) => {
                        clone.show_items(Vec::new());
                        clone.stack.set_visible_child_name("error");
                    }
                }
            }
        });
    }

    fn load_local(self: Rc<Self>) {
        let context = glib::MainContext::default();
        let clone = self.clone();
        context.spawn_local(async move {
            if let Some(cb) = &*self.load_local.borrow() {
                self.stack.set_visible_child_name("loading");

                let items = Pin::from(cb()).await;
                clone.show_items(items);
            }
        });
    }

    fn show_items(&self, items: Vec<T>) {
        let length = items.len();
        self.items.replace(items);
        self.list.update(length);
        self.stack.set_visible_child_name("content");
    }
}
