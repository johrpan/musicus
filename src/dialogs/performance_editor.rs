use super::*;
use crate::backend::Backend;
use crate::database::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use std::cell::RefCell;
use std::rc::Rc;

pub struct PerformanceEditor<F>
where
    F: Fn(PerformanceDescription) -> () + 'static,
{
    backend: Rc<Backend>,
    window: gtk::Window,
    callback: F,
    save_button: gtk::Button,
    person_label: gtk::Label,
    ensemble_label: gtk::Label,
    role_label: gtk::Label,
    person: RefCell<Option<Person>>,
    ensemble: RefCell<Option<Ensemble>>,
    role: RefCell<Option<Instrument>>,
}

impl<F> PerformanceEditor<F>
where
    F: Fn(PerformanceDescription) -> () + 'static,
{
    pub fn new<P: IsA<gtk::Window>>(
        backend: Rc<Backend>,
        parent: &P,
        performance: Option<PerformanceDescription>,
        callback: F,
    ) -> Rc<Self> {
        let builder =
            gtk::Builder::from_resource("/de/johrpan/musicus/ui/performance_editor.ui");

        get_widget!(builder, gtk::Window, window);
        get_widget!(builder, gtk::Button, cancel_button);
        get_widget!(builder, gtk::Button, save_button);
        get_widget!(builder, gtk::Button, person_button);
        get_widget!(builder, gtk::Button, ensemble_button);
        get_widget!(builder, gtk::Button, role_button);
        get_widget!(builder, gtk::Button, reset_role_button);
        get_widget!(builder, gtk::Label, person_label);
        get_widget!(builder, gtk::Label, ensemble_label);
        get_widget!(builder, gtk::Label, role_label);

        let (person, ensemble, role) = match performance {
            Some(performance) => {
                match performance.person.clone() {
                    Some(person) => {
                        person_label.set_text(&person.name_fl());
                        save_button.set_sensitive(true);
                    }
                    None => (),
                }

                match performance.ensemble.clone() {
                    Some(ensemble) => {
                        ensemble_label.set_text(&ensemble.name);
                        save_button.set_sensitive(true);
                    }
                    None => (),
                }

                match performance.role.clone() {
                    Some(role) => role_label.set_text(&role.name),
                    None => (),
                }

                (performance.person, performance.ensemble, performance.role)
            }
            None => (None, None, None),
        };

        let result = Rc::new(PerformanceEditor {
            backend: backend,
            window: window,
            callback: callback,
            save_button: save_button,
            person_label: person_label,
            ensemble_label: ensemble_label,
            role_label: role_label,
            person: RefCell::new(person),
            ensemble: RefCell::new(ensemble),
            role: RefCell::new(role),
        });

        cancel_button.connect_clicked(clone!(@strong result => move |_| {
            result.window.close();
        }));

        result
            .save_button
            .connect_clicked(clone!(@strong result => move |_| {
                (result.callback)(PerformanceDescription {
                    person: result.person.borrow().clone(),
                    ensemble: result.ensemble.borrow().clone(),
                    role: result.role.borrow().clone(),
                });
                result.window.close();
            }));

        person_button.connect_clicked(clone!(@strong result => move |_| {
            PersonSelector::new(result.backend.clone(), &result.window, clone!(@strong result => move |person| {
                result.person.replace(Some(person.clone()));
                result.person_label.set_text(&person.name_fl());
                result.ensemble.replace(None);
                result.ensemble_label.set_text("Select …");
                result.save_button.set_sensitive(true);
            })).show();
        }));

        ensemble_button.connect_clicked(clone!(@strong result => move |_| {
            EnsembleSelector::new(result.backend.clone(), &result.window, clone!(@strong result => move |ensemble| {
                result.ensemble.replace(Some(ensemble.clone()));
                result.ensemble_label.set_text(&ensemble.name);
                result.person.replace(None);
                result.person_label.set_text("Select …");
                result.save_button.set_sensitive(true);
            })).show();
        }));

        role_button.connect_clicked(clone!(@strong result => move |_| {
            InstrumentSelector::new(result.backend.clone(), &result.window, clone!(@strong result => move |role| {
                result.role.replace(Some(role.clone()));
                result.role_label.set_text(&role.name);
            })).show();
        }));

        reset_role_button.connect_clicked(clone!(@strong result => move |_| {
            result.role.replace(None);
            result.role_label.set_text("Select …");
        }));

        result.window.set_transient_for(Some(parent));

        result
    }

    pub fn show(&self) {
        self.window.show();
    }
}
