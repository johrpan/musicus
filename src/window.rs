use crate::backend::*;
use crate::dialogs::*;
use crate::screens::*;
use crate::widgets::*;
use gio::prelude::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::{action, get_widget};
use libhandy::prelude::*;
use std::rc::Rc;

pub struct Window {
    backend: Rc<Backend>,
    window: libhandy::ApplicationWindow,
    leaflet: libhandy::Leaflet,
    sidebar_box: gtk::Box,
    poe_list: Rc<PoeList>,
    stack: Stack,
}

impl Window {
    pub fn new(app: &gtk::Application) -> Rc<Self> {
        let builder = gtk::Builder::from_resource("/de/johrpan/musicus_editor/ui/window.ui");

        get_widget!(builder, libhandy::ApplicationWindow, window);
        get_widget!(builder, libhandy::Leaflet, leaflet);
        get_widget!(builder, gtk::Box, sidebar_box);
        get_widget!(builder, gtk::Box, empty_screen);

        let backend = Rc::new(Backend::new("test.sqlite"));
        let poe_list = PoeList::new(backend.clone());
        let stack = Stack::new(&empty_screen);

        let result = Rc::new(Self {
            backend,
            window,
            leaflet,
            sidebar_box,
            poe_list,
            stack,
        });

        result
            .poe_list
            .set_selected(clone!(@strong result => move |poe| {
                result.leaflet.set_visible_child(&result.stack.widget);
                match poe {
                    PersonOrEnsemble::Person(person) => {
                        let person_screen = Rc::new(PersonScreen::new(result.backend.clone(), person.clone()));

                        person_screen.set_back(clone!(@strong result => move || {
                            result.leaflet.set_visible_child(&result.sidebar_box);
                            result.stack.reset_child();
                        }));

                        person_screen.set_work_selected(clone!(@strong result, @strong person_screen => move |work| {
                            let work_screen = Rc::new(WorkScreen::new(result.backend.clone(), work.clone()));

                            work_screen.set_back(clone!(@strong result, @strong person_screen => move || {
                                result.stack.set_child(person_screen.widget.clone());
                            }));

                            work_screen.set_recording_selected(clone!(@strong result, @strong work_screen => move |recording| {
                                let recording_screen = RecordingScreen::new(result.backend.clone(), recording.clone());

                                recording_screen.set_back(clone!(@strong result, @strong work_screen => move || {
                                    result.stack.set_child(work_screen.widget.clone());
                                }));

                                result.stack.set_child(recording_screen.widget.clone());
                            }));

                            result.stack.set_child(work_screen.widget.clone());
                        }));

                        person_screen.set_recording_selected(clone!(@strong result, @strong person_screen => move |recording| {
                            let recording_screen = Rc::new(RecordingScreen::new(result.backend.clone(), recording.clone()));

                            recording_screen.set_back(clone!(@strong result, @strong person_screen => move || {
                                result.stack.set_child(person_screen.widget.clone());
                            }));

                            result.stack.set_child(recording_screen.widget.clone());
                        }));

                        result.stack.set_child(person_screen.widget.clone());
                    }
                    PersonOrEnsemble::Ensemble(ensemble) => {
                        let ensemble_screen = EnsembleScreen::new(result.backend.clone(), ensemble.clone());

                        ensemble_screen.set_back(clone!(@strong result => move || {
                            result.leaflet.set_visible_child(&result.sidebar_box);
                            result.stack.reset_child();
                        }));

                        ensemble_screen.set_recording_selected(clone!(@strong result, @strong ensemble_screen => move |recording| {
                            let recording_screen = Rc::new(RecordingScreen::new(result.backend.clone(), recording.clone()));

                            recording_screen.set_back(clone!(@strong result, @strong ensemble_screen => move || {
                                result.stack.set_child(ensemble_screen.widget.clone());
                            }));

                            result.stack.set_child(recording_screen.widget.clone());
                        }));

                        result.stack.set_child(ensemble_screen.widget.clone());
                    }
                }
            }));

        result.leaflet.add(&result.stack.widget);
        result
            .sidebar_box
            .pack_start(&result.poe_list.widget, true, true, 0);
        result.window.set_application(Some(app));

        action!(
            result.window,
            "add-person",
            clone!(@strong result => move |_, _| {
                PersonEditor::new(result.backend.clone(), &result.window, None, clone!(@strong result => move |_| {
                   result.reload();
                })).show();
            })
        );

        action!(
            result.window,
            "add-instrument",
            clone!(@strong result => move |_, _| {
                InstrumentEditor::new(result.backend.clone(), &result.window, None, |instrument| {
                    println!("{:?}", instrument);
                }).show();
            })
        );

        action!(
            result.window,
            "add-work",
            clone!(@strong result => move |_, _| {
                WorkEditor::new(result.backend.clone(), &result.window, None, clone!(@strong result => move |_| {
                    result.reload();
                })).show();
            })
        );

        action!(
            result.window,
            "add-ensemble",
            clone!(@strong result => move |_, _| {
                EnsembleEditor::new(result.backend.clone(), &result.window, None, clone!(@strong result => move |_| {
                    result.reload();
                })).show();
            })
        );

        action!(
            result.window,
            "add-recording",
            clone!(@strong result => move |_, _| {
                RecordingEditor::new(result.backend.clone(), &result.window, None, clone!(@strong result => move |_| {
                    result.reload();
                })).show();
            })
        );

        action!(
            result.window,
            "edit-person",
            Some(glib::VariantTy::new("x").unwrap()),
            clone!(@strong result => move |_, id| {
                let id = id.unwrap().get().unwrap();
                let result = result.clone();
                let c = glib::MainContext::default();
                c.spawn_local(async move {
                    let person = result.backend.get_person(id).await.unwrap();
                    PersonEditor::new(result.backend.clone(), &result.window, Some(person), clone!(@strong result => move |_| {
                        result.reload();
                    })).show();
                });
            })
        );

        action!(
            result.window,
            "delete-person",
            Some(glib::VariantTy::new("x").unwrap()),
            clone!(@strong result => move |_, id| {
                let id = id.unwrap().get().unwrap();
                let result = result.clone();
                let c = glib::MainContext::default();
                c.spawn_local(async move {
                    result.backend.delete_person(id).await.unwrap();
                    result.reload();
                });
            })
        );

        action!(
            result.window,
            "edit-ensemble",
            Some(glib::VariantTy::new("x").unwrap()),
            clone!(@strong result => move |_, id| {
                let id = id.unwrap().get().unwrap();
                let result = result.clone();
                let c = glib::MainContext::default();
                c.spawn_local(async move {
                    let ensemble = result.backend.get_ensemble(id).await.unwrap();
                    EnsembleEditor::new(result.backend.clone(), &result.window, Some(ensemble), clone!(@strong result => move |_| {
                        result.reload();
                    })).show();
                });
            })
        );

        action!(
            result.window,
            "delete-ensemble",
            Some(glib::VariantTy::new("x").unwrap()),
            clone!(@strong result => move |_, id| {
                let id = id.unwrap().get().unwrap();
                let result = result.clone();
                let c = glib::MainContext::default();
                c.spawn_local(async move {
                    result.backend.delete_ensemble(id).await.unwrap();
                    result.reload();
                });
            })
        );

        result
    }

    pub fn present(&self) {
        self.window.present();
    }

    fn reload(&self) {
        self.poe_list.clone().reload();
        self.stack.reset_child();
        self.leaflet.set_visible_child(&self.sidebar_box);
    }
}
