use super::selector::Selector;
use crate::backend::{Backend, Person, Work};
use crate::editors::{PersonEditor, WorkEditor};
use crate::navigator::{NavigationHandle, Screen};
use crate::widgets::Widget;
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// A screen for selecting a work.
pub struct WorkSelector {
    handle: NavigationHandle<Work>,
    selector: Rc<Selector<Person>>,
}

impl Screen<(), Work> for WorkSelector {
    fn new(_: (), handle: NavigationHandle<Work>) -> Rc<Self> {
        // Create UI

        let selector = Selector::<Person>::new();
        selector.set_title(&gettext("Select composer"));

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
                    // We can assume that there are no existing works of this composer and
                    // immediately show the work editor. Going back from the work editor will
                    // correctly show the person selector again.

                    let work = Work::new(person);
                    if let Some(work) = push!(this.handle, WorkEditor, Some(work)).await {
                        this.handle.pop(Some(work));
                    }
                }
            });
        }));

        this.selector.set_load_online(clone!(@weak this => move || {
            async move { this.handle.backend.get_persons().await }
        }));

        this.selector.set_load_local(clone!(@weak this => move || {
            async move { this.handle.backend.db().get_persons().await.unwrap() }
        }));

        this.selector.set_make_widget(clone!(@weak this => move |person| {
            let row = libadwaita::ActionRow::new();
            row.set_activatable(true);
            row.set_title(Some(&person.name_lf()));

            let person = person.to_owned();
            row.connect_activated(clone!(@weak this => move |_| {
                // Instead of returning the person from here, like the person selector does, we
                // show a second selector for choosing the work.

                let person = person.clone();
                spawn!(@clone this, async move {
                    if let Some(work) = push!(this.handle, WorkSelectorWorkScreen, person).await {
                        this.handle.pop(Some(work));
                    }
                });
            }));

            row.upcast()
        }));

        this.selector
            .set_filter(|search, person| person.name_fl().to_lowercase().contains(search));

        this
    }
}

impl Widget for WorkSelector {
    fn get_widget(&self) -> gtk::Widget {
        self.selector.widget.clone().upcast()
    }
}

/// The actual work selector that is displayed after the user has selected a composer.
struct WorkSelectorWorkScreen {
    handle: NavigationHandle<Work>,
    person: Person,
    selector: Rc<Selector<Work>>,
}

impl Screen<Person, Work> for WorkSelectorWorkScreen {
    fn new(person: Person, handle: NavigationHandle<Work>) -> Rc<Self> {
        let selector = Selector::<Work>::new();
        selector.set_title(&gettext("Select work"));
        selector.set_subtitle(&person.name_fl());

        let this = Rc::new(Self {
            handle,
            person,
            selector,
        });

        this.selector.set_back_cb(clone!(@weak this => move || {
            this.handle.pop(None);
        }));

        this.selector.set_add_cb(clone!(@weak this => move || {
            spawn!(@clone this, async move {
                let work = Work::new(this.person.clone());
                if let Some(work) = push!(this.handle, WorkEditor, Some(work)).await {
                    this.handle.pop(Some(work));
                }
            });
        }));

        this.selector.set_load_online(clone!(@weak this => move || {
            async move { this.handle.backend.get_works(&this.person.id).await }
        }));

        this.selector.set_load_local(clone!(@weak this => move || {
            async move { this.handle.backend.db().get_works(&this.person.id).await.unwrap() }
        }));

        this.selector.set_make_widget(clone!(@weak this => move |work| {
            let row = libadwaita::ActionRow::new();
            row.set_activatable(true);
            row.set_title(Some(&work.title));

            let work = work.to_owned();
            row.connect_activated(clone!(@weak this => move |_| {
                this.handle.pop(Some(work.clone()));
            }));

            row.upcast()
        }));

        this.selector.set_filter(|search, work| work.title.to_lowercase().contains(search));

        this
    }
}

impl Widget for WorkSelectorWorkScreen {
    fn get_widget(&self) -> gtk::Widget {
        self.selector.widget.clone().upcast()
    }
}
