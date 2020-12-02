use super::selector::Selector;
use crate::backend::Backend;
use crate::database::Person;
use crate::editors::PersonEditor;
use crate::widgets::{Navigator, NavigatorScreen};
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// A screen for selecting a person.
pub struct PersonSelector {
    backend: Rc<Backend>,
    selector: Rc<Selector<Person>>,
    selected_cb: RefCell<Option<Box<dyn Fn(&Person) -> ()>>>,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl PersonSelector {
    /// Create a new person selector.
    pub fn new(backend: Rc<Backend>) -> Rc<Self> {
        // Create UI

        let selector = Selector::<Person>::new();
        selector.set_title(&gettext("Select person"));

        let this = Rc::new(Self {
            backend,
            selector,
            selected_cb: RefCell::new(None),
            navigator: RefCell::new(None),
        });

        // Connect signals and callbacks

        this.selector.set_back_cb(clone!(@strong this => move || {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                navigator.pop();
            }
        }));

        this.selector.set_add_cb(clone!(@strong this => move || {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                let editor = PersonEditor::new(this.backend.clone(), None);
                editor
                    .set_saved_cb(clone!(@strong this => move |person| this.select(&person)));
                navigator.push(editor);
            }
        }));

        this.selector
            .set_load_online(clone!(@strong this => move || {
                let clone = this.clone();
                async move { clone.backend.get_persons().await }
            }));

        this.selector
            .set_load_local(clone!(@strong this => move || {
                let clone = this.clone();
                async move { clone.backend.db().get_persons().await.unwrap() }
            }));

        this.selector.set_make_widget(|person| {
            let label = gtk::Label::new(Some(&person.name_lf()));
            label.set_halign(gtk::Align::Start);
            label.set_margin_start(6);
            label.set_margin_end(6);
            label.set_margin_top(6);
            label.set_margin_bottom(6);
            label.upcast()
        });

        this.selector
            .set_filter(|search, person| person.name_fl().to_lowercase().contains(search));

        this.selector
            .set_selected_cb(clone!(@strong this => move |person| this.select(person)));

        this
    }

    /// Set the closure to be called when an item is selected.
    pub fn set_selected_cb<F: Fn(&Person) -> () + 'static>(&self, cb: F) {
        self.selected_cb.replace(Some(Box::new(cb)));
    }

    /// Select a person.
    fn select(&self, person: &Person) {
        if let Some(cb) = &*self.selected_cb.borrow() {
            cb(&person);
        }
    }
}

impl NavigatorScreen for PersonSelector {
    fn attach_navigator(&self, navigator: Rc<Navigator>) {
        self.navigator.replace(Some(navigator));
    }

    fn get_widget(&self) -> gtk::Widget {
        self.selector.widget.clone().upcast()
    }

    fn detach_navigator(&self) {
        self.navigator.replace(None);
    }
}
