use crate::backend::Backend;
use crate::database::*;
use crate::selectors::{EnsembleSelector, InstrumentSelector, PersonSelector};
use crate::widgets::{Navigator, NavigatorScreen};
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// A dialog for editing a performance within a recording.
pub struct PerformanceEditor {
    backend: Rc<Backend>,
    widget: gtk::Box,
    save_button: gtk::Button,
    person_row: libadwaita::ActionRow,
    ensemble_row: libadwaita::ActionRow,
    role_row: libadwaita::ActionRow,
    reset_role_button: gtk::Button,
    person: RefCell<Option<Person>>,
    ensemble: RefCell<Option<Ensemble>>,
    role: RefCell<Option<Instrument>>,
    selected_cb: RefCell<Option<Box<dyn Fn(Performance) -> ()>>>,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl PerformanceEditor {
    /// Create a new performance editor.
    pub fn new(backend: Rc<Backend>, performance: Option<Performance>) -> Rc<Self> {
        // Create UI

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/performance_editor.ui");

        get_widget!(builder, gtk::Box, widget);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::Button, save_button);
        get_widget!(builder, gtk::Button, person_button);
        get_widget!(builder, gtk::Button, ensemble_button);
        get_widget!(builder, gtk::Button, role_button);
        get_widget!(builder, gtk::Button, reset_role_button);
        get_widget!(builder, libadwaita::ActionRow, person_row);
        get_widget!(builder, libadwaita::ActionRow, ensemble_row);
        get_widget!(builder, libadwaita::ActionRow, role_row);

        let this = Rc::new(PerformanceEditor {
            backend,
            widget,
            save_button,
            person_row,
            ensemble_row,
            role_row,
            reset_role_button,
            person: RefCell::new(None),
            ensemble: RefCell::new(None),
            role: RefCell::new(None),
            selected_cb: RefCell::new(None),
            navigator: RefCell::new(None),
        });

        // Connect signals and callbacks

        back_button.connect_clicked(clone!(@strong this => move |_| {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                navigator.pop();
            }
        }));

        this.save_button
            .connect_clicked(clone!(@strong this => move |_| {
                if let Some(cb) = &*this.selected_cb.borrow() {
                    cb(Performance {
                        person: this.person.borrow().clone(),
                        ensemble: this.ensemble.borrow().clone(),
                        role: this.role.borrow().clone(),
                    });
                }

                let navigator = this.navigator.borrow().clone();
                if let Some(navigator) = navigator {
                    navigator.pop();
                }
            }));

        person_button.connect_clicked(clone!(@strong this => move |_| {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                let selector = PersonSelector::new(this.backend.clone());

                selector.set_selected_cb(clone!(@strong this, @strong navigator => move |person| {
                    this.show_person(Some(&person));
                    this.person.replace(Some(person.clone()));
                    this.show_ensemble(None);
                    this.ensemble.replace(None);
                    navigator.clone().pop();
                }));

                navigator.push(selector);
            }
        }));

        ensemble_button.connect_clicked(clone!(@strong this => move |_| {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                let selector = EnsembleSelector::new(this.backend.clone());

                selector.set_selected_cb(clone!(@strong this, @strong navigator => move |ensemble| {
                    this.show_person(None);
                    this.person.replace(None);
                    this.show_ensemble(Some(&ensemble));
                    this.ensemble.replace(Some(ensemble.clone()));
                    navigator.clone().pop();
                }));

                navigator.push(selector);
            }
        }));

        role_button.connect_clicked(clone!(@strong this => move |_| {
            let navigator = this.navigator.borrow().clone();
                if let Some(navigator) = navigator {
                let selector = InstrumentSelector::new(this.backend.clone());

                selector.set_selected_cb(clone!(@strong this, @strong navigator => move |role| {
                    this.show_role(Some(&role));
                    this.role.replace(Some(role.clone()));
                    navigator.clone().pop();
                }));

                navigator.push(selector);
            }
        }));

        this.reset_role_button
            .connect_clicked(clone!(@strong this => move |_| {
                this.show_role(None);
                this.role.replace(None);
            }));

        // Initialize

        if let Some(performance) = performance {
            if let Some(person) = performance.person {
                this.show_person(Some(&person));
                this.person.replace(Some(person));
            } else if let Some(ensemble) = performance.ensemble {
                this.show_ensemble(Some(&ensemble));
                this.ensemble.replace(Some(ensemble));
            }

            if let Some(role) = performance.role {
                this.show_role(Some(&role));
                this.role.replace(Some(role));
            }
        }

        this
    }

    /// Set a closure to be called when the user has chosen to save the performance.
    pub fn set_selected_cb<F: Fn(Performance) -> () + 'static>(&self, cb: F) {
        self.selected_cb.replace(Some(Box::new(cb)));
    }

    /// Update the UI according to person.
    fn show_person(&self, person: Option<&Person>) {
        if let Some(person) = person {
            self.person_row.set_title(Some(&gettext("Person")));
            self.person_row.set_subtitle(Some(&person.name_fl()));
            self.save_button.set_sensitive(true);
        } else {
            self.person_row.set_title(Some(&gettext("Select a person")));
            self.person_row.set_subtitle(None);
        }
    }

    /// Update the UI according to ensemble.
    fn show_ensemble(&self, ensemble: Option<&Ensemble>) {
        if let Some(ensemble) = ensemble {
            self.ensemble_row.set_title(Some(&gettext("Ensemble")));
            self.ensemble_row.set_subtitle(Some(&ensemble.name));
            self.save_button.set_sensitive(true);
        } else {
            self.ensemble_row.set_title(Some(&gettext("Select an ensemble")));
            self.ensemble_row.set_subtitle(None);
        }
    }

    /// Update the UI according to role.
    fn show_role(&self, role: Option<&Instrument>) {
        if let Some(role) = role {
            self.role_row.set_title(Some(&gettext("Role")));
            self.role_row.set_subtitle(Some(&role.name));
            self.reset_role_button.show();
        } else {
            self.role_row.set_title(Some(&gettext("Select a role")));
            self.role_row.set_subtitle(None);
            self.reset_role_button.hide();
        }
    }
}

impl NavigatorScreen for PerformanceEditor {
    fn attach_navigator(&self, navigator: Rc<Navigator>) {
        self.navigator.replace(Some(navigator));
    }

    fn get_widget(&self) -> gtk::Widget {
        self.widget.clone().upcast()
    }

    fn detach_navigator(&self) {
        self.navigator.replace(None);
    }
}
