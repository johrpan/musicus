use super::selector::Selector;
use crate::editors::PersonEditor;
use crate::navigator::{NavigationHandle, Screen};
use crate::widgets::Widget;
use adw::builders::ActionRowBuilder;
use adw::prelude::*;
use gettextrs::gettext;
use glib::clone;
use log::warn;
use musicus_backend::db::{Person, self};
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

        let this = Rc::new(Self { handle, selector });

        // Connect signals and callbacks

        this.selector.set_back_cb(clone!(@weak this =>  move || {
            this.handle.pop(None);
        }));

        this.selector.set_add_cb(clone!(@weak this =>  move || {
            spawn!(@clone this, async move {
                if let Some(person) = push!(this.handle, PersonEditor, None).await {
                    this.handle.pop(Some(person));
                }
            });
        }));

        this.selector
            .set_make_widget(clone!(@weak this =>  @default-panic, move |person| {
                let row = ActionRowBuilder::new()
                    .activatable(true)
                    .title(&person.name_lf())
                    .build();

                let person = person.to_owned();

                row.connect_activated(clone!(@weak this =>  move |_| {
                    if let Err(err) = db::update_person(&mut this.handle.backend.db().lock().unwrap(), person.clone()) {
                        warn!("Failed to update access time. {err}");
                    }

                    this.handle.pop(Some(person.clone()));
                }));

                row.upcast()
            }));

        this.selector
            .set_filter(|search, person| person.name_fl().to_lowercase().contains(search));

        this.selector
            .set_items(db::get_recent_persons(&mut this.handle.backend.db().lock().unwrap(), ).unwrap());

        this
    }
}

impl Widget for PersonSelector {
    fn get_widget(&self) -> gtk::Widget {
        self.selector.widget.clone().upcast()
    }
}
