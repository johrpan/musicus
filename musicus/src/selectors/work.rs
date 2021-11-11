use super::selector::Selector;
use crate::editors::{PersonEditor, WorkEditor};
use crate::navigator::{NavigationHandle, Screen};
use crate::widgets::Widget;
use adw::prelude::*;
use gettextrs::gettext;
use glib::clone;
use musicus_backend::db::{Person, Work};
use std::rc::Rc;

/// A screen for selecting a work.
pub struct WorkSelector {
    handle: NavigationHandle<Work>,
    selector: Rc<Selector<Person>>,
}

impl Screen<(), Work> for WorkSelector {
    fn new(_: (), handle: NavigationHandle<Work>) -> Rc<Self> {
        // Create UI

        let selector = Selector::<Person>::new(Rc::clone(&handle.backend));
        selector.set_title(&gettext("Select composer"));

        let this = Rc::new(Self { handle, selector });

        // Connect signals and callbacks

        this.selector.set_back_cb(clone!(@weak this =>  move || {
            this.handle.pop(None);
        }));

        this.selector.set_add_cb(clone!(@weak this =>  move || {
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

        this.selector
            .set_load_online(clone!(@weak this =>  @default-panic, move || {
                async move { Ok(this.handle.backend.cl().get_persons().await?) }
            }));

        this.selector
            .set_load_local(clone!(@weak this =>  @default-panic, move || {
                async move { this.handle.backend.db().get_persons().await.unwrap() }
            }));

        this.selector.set_make_widget(clone!(@weak this =>  @default-panic, move |person| {
            let row = adw::ActionRowBuilder::new()
                .activatable(true)
                .title(&person.name_lf())
                .build();

            let person = person.to_owned();
            row.connect_activated(clone!(@weak this =>  move |_| {
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
        let selector = Selector::<Work>::new(Rc::clone(&handle.backend));
        selector.set_title(&gettext("Select work"));
        selector.set_subtitle(&person.name_fl());

        let this = Rc::new(Self {
            handle,
            person,
            selector,
        });

        this.selector.set_back_cb(clone!(@weak this =>  move || {
            this.handle.pop(None);
        }));

        this.selector.set_add_cb(clone!(@weak this =>  move || {
            spawn!(@clone this, async move {
                let work = Work::new(this.person.clone());
                if let Some(work) = push!(this.handle, WorkEditor, Some(work)).await {
                    this.handle.pop(Some(work));
                }
            });
        }));

        this.selector
            .set_load_online(clone!(@weak this =>  @default-panic, move || {
                async move { Ok(this.handle.backend.cl().get_works(&this.person.id).await?) }
            }));

        this.selector
            .set_load_local(clone!(@weak this =>  @default-panic, move || {
                async move { this.handle.backend.db().get_works(&this.person.id).await.unwrap() }
            }));

        this.selector
            .set_make_widget(clone!(@weak this =>  @default-panic, move |work| {
                let row = adw::ActionRowBuilder::new()
                    .activatable(true)
                    .title(&work.title)
                    .build();

                let work = work.to_owned();
                row.connect_activated(clone!(@weak this =>  move |_| {
                    this.handle.pop(Some(work.clone()));
                }));

                row.upcast()
            }));

        this.selector
            .set_filter(|search, work| work.title.to_lowercase().contains(search));

        this
    }
}

impl Widget for WorkSelectorWorkScreen {
    fn get_widget(&self) -> gtk::Widget {
        self.selector.widget.clone().upcast()
    }
}
