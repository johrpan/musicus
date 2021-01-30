use super::selector::Selector;
use crate::backend::Backend;
use crate::database::{Person, Work};
use crate::editors::WorkEditor;
use crate::widgets::{Navigator, NavigatorScreen};
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// A screen for selecting a work.
pub struct WorkSelector {
    backend: Rc<Backend>,
    person: Person,
    selector: Rc<Selector<Work>>,
    selected_cb: RefCell<Option<Box<dyn Fn(&Work) -> ()>>>,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl WorkSelector {
    /// Create a new work selector for works by a specific composer.
    pub fn new(backend: Rc<Backend>, person: Person) -> Rc<Self> {
        // Create UI

        let selector = Selector::<Work>::new();
        selector.set_title(&gettext("Select work"));
        selector.set_subtitle(&person.name_fl());

        let this = Rc::new(Self {
            backend,
            person,
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
                let work = Work::new(this.person.clone());

                let editor = WorkEditor::new(this.backend.clone(), Some(work));

                editor
                    .set_saved_cb(clone!(@strong this, @strong navigator => move |work| {
                        navigator.clone().pop();
                        this.select(&work);
                    }));

                navigator.push(editor);
            }
        }));

        this.selector
            .set_load_online(clone!(@strong this => move || {
                let clone = this.clone();
                async move { clone.backend.get_works(&clone.person.id).await }
            }));

        this.selector
            .set_load_local(clone!(@strong this => move || {
                let clone = this.clone();
                async move { clone.backend.db().get_works(&clone.person.id).await.unwrap() }
            }));

        this.selector.set_make_widget(clone!(@strong this => move |work| {
            let row = libadwaita::ActionRow::new();
            row.set_activatable(true);
            row.set_title(Some(&work.title));

            let work = work.to_owned();
            row.connect_activated(clone!(@strong this => move |_| {
                this.select(&work);
            }));

            row.upcast()
        }));

        this.selector
            .set_filter(|search, work| work.title.to_lowercase().contains(search));

        this
    }

    /// Set the closure to be called when an item is selected.
    pub fn set_selected_cb<F: Fn(&Work) -> () + 'static>(&self, cb: F) {
        self.selected_cb.replace(Some(Box::new(cb)));
    }

    /// Select a work.
    fn select(&self, work: &Work) {
        if let Some(cb) = &*self.selected_cb.borrow() {
            cb(&work);
        }
    }
}

impl NavigatorScreen for WorkSelector {
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
