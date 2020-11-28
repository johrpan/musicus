use super::work_selector_person_screen::*;
use crate::backend::Backend;
use crate::database::*;
use crate::widgets::*;
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use libhandy::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// A widget for selecting a work from a list of existing ones.
pub struct WorkSelector {
    pub widget: libhandy::Leaflet,
    backend: Rc<Backend>,
    sidebar_box: gtk::Box,
    server_check_button: gtk::CheckButton,
    stack: gtk::Stack,
    list: Rc<List<Person>>,
    selected_cb: RefCell<Option<Box<dyn Fn(Work) -> ()>>>,
    add_cb: RefCell<Option<Box<dyn Fn() -> ()>>>,
    navigator: Rc<Navigator>,
}

impl WorkSelector {
    /// Create a new work selector.
    pub fn new(backend: Rc<Backend>) -> Rc<Self> {
        // Create UI

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/work_selector.ui");

        get_widget!(builder, libhandy::Leaflet, widget);
        get_widget!(builder, gtk::Button, add_button);
        get_widget!(builder, gtk::Box, sidebar_box);
        get_widget!(builder, gtk::CheckButton, server_check_button);
        get_widget!(builder, gtk::SearchEntry, search_entry);
        get_widget!(builder, gtk::Stack, stack);
        get_widget!(builder, gtk::ScrolledWindow, scroll);
        get_widget!(builder, gtk::Button, try_again_button);
        get_widget!(builder, gtk::Box, empty_screen);

        let list = List::<Person>::new(&gettext("No persons found."));
        scroll.add(&list.widget);

        let navigator = Navigator::new(&empty_screen);
        widget.add(&navigator.widget);

        let this = Rc::new(Self {
            widget,
            backend,
            sidebar_box,
            server_check_button,
            stack,
            list,
            selected_cb: RefCell::new(None),
            add_cb: RefCell::new(None),
            navigator,
        });

        // Connect signals and callbacks

        add_button.connect_clicked(clone!(@strong this => move |_| {
            if let Some(cb) = &*this.add_cb.borrow() {
                cb();
            }
        }));

        search_entry.connect_search_changed(clone!(@strong this => move |_| {
            this.list.invalidate_filter();
        }));

        let load_online = Rc::new(clone!(@strong this => move || {
            this.stack.set_visible_child_name("loading");

            let context = glib::MainContext::default();
            let clone = this.clone();
            context.spawn_local(async move {
                match clone.backend.get_persons().await {
                    Ok(persons) => {
                        clone.list.show_items(persons);
                        clone.stack.set_visible_child_name("content");
                    }
                    Err(_) => {
                        clone.list.show_items(Vec::new());
                        clone.stack.set_visible_child_name("error");
                    }
                }
            });
        }));

        let load_local = Rc::new(clone!(@strong this => move || {
            this.stack.set_visible_child_name("loading");

            let context = glib::MainContext::default();
            let clone = this.clone();
            context.spawn_local(async move {
                let persons = clone.backend.db().get_persons().await.unwrap();
                clone.list.show_items(persons);
                clone.stack.set_visible_child_name("content");
            });
        }));

        this.server_check_button.connect_toggled(
            clone!(@strong this, @strong load_local, @strong load_online => move |_| {
                if this.server_check_button.get_active() {
                    load_online();
                } else {
                    load_local();
                }
            }),
        );

        this.list.set_make_widget(|person: &Person| {
            let label = gtk::Label::new(Some(&person.name_lf()));
            label.set_halign(gtk::Align::Start);
            label.set_margin_start(6);
            label.set_margin_end(6);
            label.set_margin_top(6);
            label.set_margin_bottom(6);
            label.upcast()
        });

        this.list
            .set_filter(clone!(@strong search_entry => move |person: &Person| {
                let search = search_entry.get_text().to_string().to_lowercase();
                let name = person.name_fl().to_lowercase();
                search.is_empty() || name.contains(&search)
            }));

        this.list
            .set_selected(clone!(@strong this => move |person| {
                let online = this.server_check_button.get_active();

                let person_screen = WorkSelectorPersonScreen::new(
                    this.backend.clone(),
                    person.clone(),
                    online,
                );

                person_screen.set_selected_cb(clone!(@strong this => move |work| {
                    if let Some(cb) = &*this.selected_cb.borrow() {
                        cb(work);
                    }
                }));

                this.navigator.clone().replace(person_screen);
                this.widget.set_visible_child(&this.navigator.widget);
            }));

        try_again_button.connect_clicked(clone!(@strong load_online => move |_| {
            load_online();
        }));

        // Initialize
        load_online();

        this.navigator.set_back_cb(clone!(@strong this => move || {
            this.widget.set_visible_child(&this.sidebar_box);
        }));

        this
    }

    /// Set the closure to be called if the user wants to add a new work.
    pub fn set_add_cb<F: Fn() -> () + 'static>(&self, cb: F) {
        self.add_cb.replace(Some(Box::new(cb)));
    }

    /// Set the closure to be called when the user has selected a work.
    pub fn set_selected_cb<F: Fn(Work) -> () + 'static>(&self, cb: F) {
        self.selected_cb.replace(Some(Box::new(cb)));
    }
}
