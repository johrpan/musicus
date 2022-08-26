use crate::navigator::{NavigationHandle, Screen};
use crate::selectors::{EnsembleSelector, InstrumentSelector, PersonSelector};
use crate::widgets::{ButtonRow, Editor, Section, Widget};
use adw::prelude::*;
use gettextrs::gettext;
use gtk::builders::ButtonBuilder;
use gtk::{builders::ListBoxBuilder, glib::clone};
use log::error;
use musicus_backend::db::{Ensemble, Instrument, Performance, Person, PersonOrEnsemble};
use std::cell::RefCell;
use std::rc::Rc;

/// A dialog for editing a performance within a recording.
pub struct PerformanceEditor {
    handle: NavigationHandle<Performance>,
    editor: Editor,
    person_row: ButtonRow,
    ensemble_row: ButtonRow,
    role_row: ButtonRow,
    reset_role_button: gtk::Button,
    person: RefCell<Option<Person>>,
    ensemble: RefCell<Option<Ensemble>>,
    role: RefCell<Option<Instrument>>,
}

impl Screen<Option<Performance>, Performance> for PerformanceEditor {
    /// Create a new performance editor.
    fn new(performance: Option<Performance>, handle: NavigationHandle<Performance>) -> Rc<Self> {
        let editor = Editor::new();
        editor.set_title("Performance");
        editor.set_may_save(false);

        let performer_list = ListBoxBuilder::new()
            .selection_mode(gtk::SelectionMode::None)
            .css_classes(vec![String::from("boxed-list")])
            .build();

        let person_row = ButtonRow::new("Person", "Select");
        let ensemble_row = ButtonRow::new("Ensemble", "Select");

        performer_list.append(&person_row.get_widget());
        performer_list.append(&ensemble_row.get_widget());

        let performer_section = Section::new(&gettext("Performer"), &performer_list);
        performer_section.set_subtitle(&gettext(
            "Select either a person or an ensemble as a performer.",
        ));

        let role_list = ListBoxBuilder::new()
            .selection_mode(gtk::SelectionMode::None)
            .css_classes(vec![String::from("boxed-list")])
            .build();

        let reset_role_button = ButtonBuilder::new()
            .icon_name("user-trash-symbolic")
            .valign(gtk::Align::Center)
            .visible(false)
            .build();

        let role_row = ButtonRow::new("Role", "Select");
        role_row.widget.add_suffix(&reset_role_button);

        role_list.append(&role_row.get_widget());

        let role_section = Section::new(&gettext("Role"), &role_list);
        role_section.set_subtitle(&gettext(
            "Optionally, choose a role to specify what the performer does.",
        ));

        editor.add_content(&performer_section);
        editor.add_content(&role_section);

        let this = Rc::new(PerformanceEditor {
            handle,
            editor,
            person_row,
            ensemble_row,
            role_row,
            reset_role_button,
            person: RefCell::new(None),
            ensemble: RefCell::new(None),
            role: RefCell::new(None),
        });

        this.editor.set_back_cb(clone!(@weak this => move || {
            this.handle.pop(None);
        }));

        this.editor.set_save_cb(clone!(@weak this => move || {
            let performance = Performance {
                performer: if let Some(person) = this.person.borrow().clone() {
                    PersonOrEnsemble::Person(person)
                } else if let Some(ensemble) = this.ensemble.borrow().clone() {
                    PersonOrEnsemble::Ensemble(ensemble)
                } else {
                    error!("Tried to save performance without performer");
                    return;
                },
                role: this.role.borrow().clone(),
            };

            this.handle.pop(Some(performance));
        }));

        this.person_row.set_cb(clone!(@weak this => move || {
            spawn!(@clone this, async move {
                if let Some(person) = push!(this.handle, PersonSelector).await {
                    this.show_person(Some(&person));
                    this.person.replace(Some(person));
                    this.show_ensemble(None);
                    this.ensemble.replace(None);
                }
            });
        }));

        this.ensemble_row.set_cb(clone!(@weak this =>  move || {
            spawn!(@clone this, async move {
                if let Some(ensemble) = push!(this.handle, EnsembleSelector).await {
                    this.show_person(None);
                    this.person.replace(None);
                    this.show_ensemble(Some(&ensemble));
                    this.ensemble.replace(Some(ensemble));
                }
            });
        }));

        this.role_row.set_cb(clone!(@weak this =>  move || {
            spawn!(@clone this, async move {
                if let Some(role) = push!(this.handle, InstrumentSelector).await {
                    this.show_role(Some(&role));
                    this.role.replace(Some(role));
                }
            });
        }));

        this.reset_role_button
            .connect_clicked(clone!(@weak this =>  move |_| {
                this.show_role(None);
                this.role.replace(None);
            }));

        // Initialize

        if let Some(performance) = performance {
            match performance.performer {
                PersonOrEnsemble::Person(person) => {
                    this.show_person(Some(&person));
                    this.person.replace(Some(person));
                }
                PersonOrEnsemble::Ensemble(ensemble) => {
                    this.show_ensemble(Some(&ensemble));
                    this.ensemble.replace(Some(ensemble));
                }
            };

            if let Some(role) = performance.role {
                this.show_role(Some(&role));
                this.role.replace(Some(role));
            }
        }

        this
    }
}

impl PerformanceEditor {
    /// Update the UI according to person.
    fn show_person(&self, person: Option<&Person>) {
        if let Some(person) = person {
            self.person_row.set_subtitle(&person.name_fl());
            self.editor.set_may_save(true);
        } else {
            self.person_row.set_subtitle("");
        }
    }

    /// Update the UI according to ensemble.
    fn show_ensemble(&self, ensemble: Option<&Ensemble>) {
        if let Some(ensemble) = ensemble {
            self.ensemble_row.set_subtitle(&ensemble.name);
            self.editor.set_may_save(true);
        } else {
            self.ensemble_row.set_subtitle("");
        }
    }

    /// Update the UI according to role.
    fn show_role(&self, role: Option<&Instrument>) {
        if let Some(role) = role {
            self.role_row.set_subtitle(&role.name);
            self.reset_role_button.show();
        } else {
            self.role_row.set_subtitle("");
            self.reset_role_button.hide();
        }
    }
}

impl Widget for PerformanceEditor {
    fn get_widget(&self) -> gtk::Widget {
        self.editor.widget.clone().upcast()
    }
}
