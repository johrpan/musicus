use super::selector::Selector;
use crate::navigator::{NavigationHandle, Screen};
use crate::widgets::Widget;
use adw::builders::ActionRowBuilder;
use adw::prelude::*;
use gettextrs::gettext;
use glib::clone;
use log::warn;
use musicus_backend::db::{Medium, PersonOrEnsemble};
use std::rc::Rc;

/// A screen for selecting a medium.
pub struct MediumSelector {
    handle: NavigationHandle<Medium>,
    selector: Rc<Selector<PersonOrEnsemble>>,
}

impl Screen<(), Medium> for MediumSelector {
    fn new(_: (), handle: NavigationHandle<Medium>) -> Rc<Self> {
        // Create UI

        let selector = Selector::<PersonOrEnsemble>::new();
        selector.set_title(&gettext("Select performer"));

        let this = Rc::new(Self { handle, selector });

        // Connect signals and callbacks

        this.selector.set_back_cb(clone!(@weak this =>  move || {
            this.handle.pop(None);
        }));

        this.selector.set_make_widget(clone!(@weak this =>  @default-panic, move |poe| {
            let row = ActionRowBuilder::new()
                .activatable(true)
                .title(&poe.get_title())
                .build();

            let poe = poe.to_owned();
            row.connect_activated(clone!(@weak this =>  move |_| {
                let poe = poe.clone();
                spawn!(@clone this, async move {
                    if let Some(medium) = push!(this.handle, MediumSelectorMediumScreen, poe).await {
                        this.handle.pop(Some(medium));
                    }
                });
            }));

            row.upcast()
        }));

        this.selector
            .set_filter(|search, poe| poe.get_title().to_lowercase().contains(search));

        // Initialize items.

        let mut poes = Vec::new();

        let persons = this.handle.backend.db().get_recent_persons().unwrap();
        let ensembles = this.handle.backend.db().get_recent_ensembles().unwrap();

        for person in persons {
            poes.push(PersonOrEnsemble::Person(person));
        }

        for ensemble in ensembles {
            poes.push(PersonOrEnsemble::Ensemble(ensemble));
        }

        this.selector.set_items(poes);

        this
    }
}

impl Widget for MediumSelector {
    fn get_widget(&self) -> gtk::Widget {
        self.selector.widget.clone().upcast()
    }
}

/// The actual medium selector that is displayed after the user has selected a person or ensemble.
struct MediumSelectorMediumScreen {
    handle: NavigationHandle<Medium>,
    poe: PersonOrEnsemble,
    selector: Rc<Selector<Medium>>,
}

impl Screen<PersonOrEnsemble, Medium> for MediumSelectorMediumScreen {
    fn new(poe: PersonOrEnsemble, handle: NavigationHandle<Medium>) -> Rc<Self> {
        let selector = Selector::<Medium>::new();
        selector.set_title(&gettext("Select medium"));
        selector.set_subtitle(&poe.get_title());

        let this = Rc::new(Self {
            handle,
            poe,
            selector,
        });

        this.selector.set_back_cb(clone!(@weak this =>  move || {
            this.handle.pop(None);
        }));

        this.selector
            .set_make_widget(clone!(@weak this =>  @default-panic, move |medium| {
                let row = ActionRowBuilder::new()
                    .activatable(true)
                    .title(&medium.name)
                    .build();

                let medium = medium.to_owned();
                row.connect_activated(clone!(@weak this =>  move |_| {
                    if let Err(err) = this.handle.backend.db().update_medium(medium.clone()) {
                        warn!("Failed to update access time. {err}");
                    }

                    this.handle.pop(Some(medium.clone()));
                }));

                row.upcast()
            }));

        this.selector
            .set_filter(|search, medium| medium.name.to_lowercase().contains(search));

        // Initialize items.
        match this.poe.clone() {
            PersonOrEnsemble::Person(person) => {
                this.selector.set_items(
                    this.handle
                        .backend
                        .db()
                        .get_mediums_for_person(&person.id)
                        .unwrap(),
                );
            }
            PersonOrEnsemble::Ensemble(ensemble) => {
                this.selector.set_items(
                    this.handle
                        .backend
                        .db()
                        .get_mediums_for_ensemble(&ensemble.id)
                        .unwrap(),
                );
            }
        }

        this
    }
}

impl Widget for MediumSelectorMediumScreen {
    fn get_widget(&self) -> gtk::Widget {
        self.selector.widget.clone().upcast()
    }
}
