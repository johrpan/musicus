use crate::backend::Backend;
use crate::database::*;
use crate::selectors::{EnsembleSelector, InstrumentSelector, PersonSelector};
use crate::widgets::{Editor, Navigator, NavigatorScreen, Section, ButtonRow, Widget};
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// A dialog for editing a performance within a recording.
pub struct PerformanceEditor {
    backend: Rc<Backend>,
    editor: Editor,
    person_row: ButtonRow,
    ensemble_row: ButtonRow,
    role_row: ButtonRow,
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
        let editor = Editor::new();
        editor.set_title("Performance");
        editor.set_may_save(false);

        let performer_list = gtk::ListBoxBuilder::new()
            .selection_mode(gtk::SelectionMode::None)
            .build();

        let person_row = ButtonRow::new("Person", "Select");
        let ensemble_row = ButtonRow::new("Ensemble", "Select");

        performer_list.append(&person_row.get_widget());
        performer_list.append(&ensemble_row.get_widget());

        let performer_section = Section::new(&gettext("Performer"), &performer_list);
        performer_section.set_subtitle(
            &gettext("Select either a person or an ensemble as a performer."));

        let role_list = gtk::ListBoxBuilder::new()
            .selection_mode(gtk::SelectionMode::None)
            .build();

        let reset_role_button = gtk::ButtonBuilder::new()
            .icon_name("user-trash-symbolic")
            .valign(gtk::Align::Center)
            .visible(false)
            .build();

        let role_row = ButtonRow::new("Role", "Select");
        role_row.widget.add_suffix(&reset_role_button);

        role_list.append(&role_row.get_widget());

        let role_section = Section::new(&gettext("Role"), &role_list);
        role_section.set_subtitle(
            &gettext("Optionally, choose a role to specify what the performer does."));

        editor.add_content(&performer_section);
        editor.add_content(&role_section);

        let this = Rc::new(PerformanceEditor {
            backend,
            editor,
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

        this.editor.set_back_cb(clone!(@strong this => move || {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                navigator.pop();
            }
        }));

        this.editor.set_save_cb(clone!(@weak this => move || {
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

        this.person_row.set_cb(clone!(@weak this => move || {
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

        this.ensemble_row.set_cb(clone!(@weak this => move || {
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

        this.role_row.set_cb(clone!(@weak this => move || {
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

        this.reset_role_button.connect_clicked(clone!(@weak this => move |_| {
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
            self.person_row.set_subtitle(Some(&person.name_fl()));
            self.editor.set_may_save(true);
        } else {
            self.person_row.set_subtitle(None);
        }
    }

    /// Update the UI according to ensemble.
    fn show_ensemble(&self, ensemble: Option<&Ensemble>) {
        if let Some(ensemble) = ensemble {
            self.ensemble_row.set_subtitle(Some(&ensemble.name));
            self.editor.set_may_save(true);
        } else {
            self.ensemble_row.set_subtitle(None);
        }
    }

    /// Update the UI according to role.
    fn show_role(&self, role: Option<&Instrument>) {
        if let Some(role) = role {
            self.role_row.set_subtitle(Some(&role.name));
            self.reset_role_button.show();
        } else {
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
        self.editor.widget.clone().upcast()
    }

    fn detach_navigator(&self) {
        self.navigator.replace(None);
    }
}
