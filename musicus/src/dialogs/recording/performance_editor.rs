use crate::backend::*;
use crate::database::*;
use crate::dialogs::*;
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use std::cell::RefCell;
use std::rc::Rc;

/// A dialog for editing a performance within a recording.
pub struct PerformanceEditor {
    backend: Rc<Backend>,
    window: libhandy::Window,
    save_button: gtk::Button,
    person_label: gtk::Label,
    ensemble_label: gtk::Label,
    role_label: gtk::Label,
    reset_role_button: gtk::Button,
    person: RefCell<Option<Person>>,
    ensemble: RefCell<Option<Ensemble>>,
    role: RefCell<Option<Instrument>>,
    selected_cb: RefCell<Option<Box<dyn Fn(Performance) -> ()>>>,
}

impl PerformanceEditor {
    /// Create a new performance editor.
    pub fn new<P: IsA<gtk::Window>>(
        backend: Rc<Backend>,
        parent: &P,
        performance: Option<Performance>,
    ) -> Rc<Self> {
        // Create UI

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/performance_editor.ui");

        get_widget!(builder, libhandy::Window, window);
        get_widget!(builder, gtk::Button, cancel_button);
        get_widget!(builder, gtk::Button, save_button);
        get_widget!(builder, gtk::Button, person_button);
        get_widget!(builder, gtk::Button, ensemble_button);
        get_widget!(builder, gtk::Button, role_button);
        get_widget!(builder, gtk::Button, reset_role_button);
        get_widget!(builder, gtk::Label, person_label);
        get_widget!(builder, gtk::Label, ensemble_label);
        get_widget!(builder, gtk::Label, role_label);

        window.set_transient_for(Some(parent));

        let this = Rc::new(PerformanceEditor {
            backend,
            window,
            save_button,
            person_label,
            ensemble_label,
            role_label,
            reset_role_button,
            person: RefCell::new(None),
            ensemble: RefCell::new(None),
            role: RefCell::new(None),
            selected_cb: RefCell::new(None),
        });

        // Connect signals and callbacks

        cancel_button.connect_clicked(clone!(@strong this => move |_| {
            this.window.close();
        }));

        this.save_button
            .connect_clicked(clone!(@strong this => move |_| {
                if let Some(cb) = &*this.selected_cb.borrow() {
                    cb(Performance {
                        person: this.person.borrow().clone(),
                        ensemble: this.ensemble.borrow().clone(),
                        role: this.role.borrow().clone(),
                    });

                    this.window.close();
                }
            }));

        person_button.connect_clicked(clone!(@strong this => move |_| {
            let dialog = PersonSelector::new(this.backend.clone(), &this.window);

            dialog.set_selected_cb(clone!(@strong this => move |person| {
                this.show_person(Some(&person));
                this.person.replace(Some(person));
                this.show_ensemble(None);
                this.ensemble.replace(None);
            }));

            dialog.show();
        }));

        ensemble_button.connect_clicked(clone!(@strong this => move |_| {
            let dialog = EnsembleSelector::new(this.backend.clone(), &this.window);

            dialog.set_selected_cb(clone!(@strong this => move |ensemble| {
                this.show_person(None);
                this.person.replace(None);
                this.show_ensemble(Some(&ensemble));
                this.ensemble.replace(Some(ensemble));
            }));

            dialog.show();
        }));

        role_button.connect_clicked(clone!(@strong this => move |_| {
            InstrumentSelector::new(this.backend.clone(), &this.window, clone!(@strong this => move |role| {
                this.show_role(Some(&role));
                this.role.replace(Some(role));
            })).show();
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

    /// Show the performance editor.
    pub fn show(&self) {
        self.window.show();
    }

    /// Update the UI according to person.
    fn show_person(&self, person: Option<&Person>) {
        if let Some(person) = person {
            self.person_label.set_text(&person.name_fl());
            self.save_button.set_sensitive(true);
        } else {
            self.person_label.set_text(&gettext("Select …"));
        }
    }

    /// Update the UI according to ensemble.
    fn show_ensemble(&self, ensemble: Option<&Ensemble>) {
        if let Some(ensemble) = ensemble {
            self.ensemble_label.set_text(&ensemble.name);
            self.save_button.set_sensitive(true);
        } else {
            self.ensemble_label.set_text(&gettext("Select …"));
        }
    }

    /// Update the UI according to role.
    fn show_role(&self, role: Option<&Instrument>) {
        if let Some(role) = role {
            self.role_label.set_text(&role.name);
            self.reset_role_button.show();
        } else {
            self.role_label.set_text(&gettext("Select …"));
            self.reset_role_button.hide();
        }
    }
}
