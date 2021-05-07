use super::selector::Selector;
use crate::navigator::{NavigationHandle, Screen};
use crate::widgets::Widget;
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use libadwaita::prelude::*;
use musicus_backend::db::{Person, Ensemble, Medium};
use std::rc::Rc;

/// Either a person or an ensemble to be shown in the list.
#[derive(Clone, Debug)]
pub enum PersonOrEnsemble {
    Person(Person),
    Ensemble(Ensemble),
}

impl PersonOrEnsemble {
    /// Get a short textual representation of the item.
    pub fn get_title(&self) -> String {
        match self {
            PersonOrEnsemble::Person(person) => person.name_lf(),
            PersonOrEnsemble::Ensemble(ensemble) => ensemble.name.clone(),
        }
    }
}

/// A screen for selecting a medium.
pub struct MediumSelector {
    handle: NavigationHandle<Medium>,
    selector: Rc<Selector<PersonOrEnsemble>>,
}

impl Screen<(), Medium> for MediumSelector {
    fn new(_: (), handle: NavigationHandle<Medium>) -> Rc<Self> {
        // Create UI

        let selector = Selector::<PersonOrEnsemble>::new(Rc::clone(&handle.backend));
        selector.set_title(&gettext("Select performer"));

        let this = Rc::new(Self {
            handle,
            selector,
        });

        // Connect signals and callbacks

        this.selector.set_back_cb(clone!(@weak this => move || {
            this.handle.pop(None);
        }));

        this.selector.set_load_online(clone!(@weak this => move || {
            async move {
                let mut poes = Vec::new();

                let persons = this.handle.backend.cl().get_persons().await?;
                let ensembles = this.handle.backend.cl().get_ensembles().await?;

                for person in persons {
                    poes.push(PersonOrEnsemble::Person(person));
                }

                for ensemble in ensembles {
                    poes.push(PersonOrEnsemble::Ensemble(ensemble));
                }

                Ok(poes)
            }
        }));

        this.selector.set_load_local(clone!(@weak this => move || {
            async move {
                let mut poes = Vec::new();

                let persons = this.handle.backend.db().get_persons().await.unwrap();
                let ensembles = this.handle.backend.db().get_ensembles().await.unwrap();

                for person in persons {
                    poes.push(PersonOrEnsemble::Person(person));
                }

                for ensemble in ensembles {
                    poes.push(PersonOrEnsemble::Ensemble(ensemble));
                }

                poes
            }
        }));

        this.selector.set_make_widget(clone!(@weak this => move |poe| {
            let row = libadwaita::ActionRow::new();
            row.set_activatable(true);
            row.set_title(Some(&poe.get_title()));

            let poe = poe.to_owned();
            row.connect_activated(clone!(@weak this => move |_| {
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
        let selector = Selector::<Medium>::new(Rc::clone(&handle.backend));
        selector.set_title(&gettext("Select medium"));
        selector.set_subtitle(&poe.get_title());

        let this = Rc::new(Self {
            handle,
            poe,
            selector,
        });

        this.selector.set_back_cb(clone!(@weak this => move || {
            this.handle.pop(None);
        }));

        match this.poe.clone() {
            PersonOrEnsemble::Person(person) => {
                // this.selector.set_load_online(clone!(@weak this => move || {
                //     async move { this.handle.backend.cl().get_mediums_for_person(&person.id).await }
                // }));

                this.selector.set_load_local(clone!(@weak this => move || {
                    let person = person.clone();
                    async move { this.handle.backend.db().get_mediums_for_person(&person.id).await.unwrap() }
                }));
            }
            PersonOrEnsemble::Ensemble(ensemble) => {
                this.selector.set_load_local(clone!(@weak this => move || {
                    let ensemble = ensemble.clone();
                    async move { this.handle.backend.db().get_mediums_for_ensemble(&ensemble.id).await.unwrap() }
                }));
            }
        }

        this.selector.set_make_widget(clone!(@weak this => move |medium| {
            let row = libadwaita::ActionRow::new();
            row.set_activatable(true);
            row.set_title(Some(&medium.name));

            let medium = medium.to_owned();
            row.connect_activated(clone!(@weak this => move |_| {
                this.handle.pop(Some(medium.clone()));
            }));

            row.upcast()
        }));

        this.selector.set_filter(|search, medium| medium.name.to_lowercase().contains(search));

        this
    }
}

impl Widget for MediumSelectorMediumScreen {
    fn get_widget(&self) -> gtk::Widget {
        self.selector.widget.clone().upcast()
    }
}
