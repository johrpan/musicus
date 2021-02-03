use super::selector::Selector;
use crate::backend::{Backend, Person};
use crate::editors::PersonEditor;
use crate::navigator::{NavigationHandle, Screen};
use crate::widgets::Widget;
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// A screen for selecting a person.
pub struct PersonSelector {
    handle: NavigationHandle<Person>,
    selector: Rc<Selector<Person>>,
}

impl Screen<(), Person> for PersonSelector {
    /// Create a new person selector.
    fn new(_: (), handle: NavigationHandle<Person>) -> Rc<Self> {
        // Create UI

        let selector = Selector::<Person>::new();
        selector.set_title(&gettext("Select person"));

        let this = Rc::new(Self {
            handle,
            selector,
        });

        // Connect signals and callbacks

        this.selector.set_back_cb(clone!(@weak this => move || {
            this.handle.pop(None);
        }));

        this.selector.set_add_cb(clone!(@weak this => move || {
            spawn!(@clone this, async move {
                if let Some(person) = push!(this.handle, PersonEditor, None).await {
                    this.handle.pop(Some(person));
                }
            });
        }));

        this.selector.set_load_online(clone!(@weak this => move || {
            let clone = this.clone();
            async move { clone.handle.backend.get_persons().await }
        }));

        this.selector.set_load_local(clone!(@weak this => move || {
            let clone = this.clone();
            async move { clone.handle.backend.db().get_persons().await.unwrap() }
        }));

        this.selector.set_make_widget(clone!(@weak this => move |person| {
            let row = libadwaita::ActionRow::new();
            row.set_activatable(true);
            row.set_title(Some(&person.name_lf()));

            let person = person.to_owned();
            row.connect_activated(clone!(@weak this => move |_| {
                this.handle.pop(Some(person.clone()));
            }));

            row.upcast()
        }));

        this.selector
            .set_filter(|search, person| person.name_fl().to_lowercase().contains(search));

        this
    }
}

impl Widget for PersonSelector {
    fn get_widget(&self) -> gtk::Widget {
        self.selector.widget.clone().upcast()
    }
}
