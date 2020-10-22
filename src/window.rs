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
    navigator: Rc<Navigator>,
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
        let navigator = Navigator::new(&empty_screen);

        let result = Rc::new(Self {
            backend,
            window,
            leaflet,
            sidebar_box,
            poe_list,
            navigator,
        });

        result
            .poe_list
            .set_selected(clone!(@strong result => move |poe| {
                result.leaflet.set_visible_child(&result.navigator.widget);
                match poe {
                    PersonOrEnsemble::Person(person) => {
                        result.navigator.clone().replace(PersonScreen::new(result.backend.clone(), person.clone()));
                    }
                    PersonOrEnsemble::Ensemble(ensemble) => {
                        result.navigator.clone().replace(EnsembleScreen::new(result.backend.clone(), ensemble.clone()));
                    }
                }
            }));

        result.leaflet.add(&result.navigator.widget);
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

        action!(
            result.window,
            "edit-work",
            Some(glib::VariantTy::new("x").unwrap()),
            clone!(@strong result => move |_, id| {
                let id = id.unwrap().get().unwrap();
                let result = result.clone();
                let c = glib::MainContext::default();
                c.spawn_local(async move {
                    let work = result.backend.get_work_description(id).await.unwrap();
                    WorkEditor::new(result.backend.clone(), &result.window, Some(work), clone!(@strong result => move |_| {
                        result.reload();
                    })).show();
                });
            })
        );

        action!(
            result.window,
            "delete-work",
            Some(glib::VariantTy::new("x").unwrap()),
            clone!(@strong result => move |_, id| {
                let id = id.unwrap().get().unwrap();
                let result = result.clone();
                let c = glib::MainContext::default();
                c.spawn_local(async move {
                    result.backend.delete_work(id).await.unwrap();
                    result.reload();
                });
            })
        );

        action!(
            result.window,
            "edit-recording",
            Some(glib::VariantTy::new("x").unwrap()),
            clone!(@strong result => move |_, id| {
                let id = id.unwrap().get().unwrap();
                let result = result.clone();
                let c = glib::MainContext::default();
                c.spawn_local(async move {
                    let recording = result.backend.get_recording_description(id).await.unwrap();
                    RecordingEditor::new(result.backend.clone(), &result.window, Some(recording), clone!(@strong result => move |_| {
                        result.reload();
                    })).show();
                });
            })
        );

        action!(
            result.window,
            "delete-recording",
            Some(glib::VariantTy::new("x").unwrap()),
            clone!(@strong result => move |_, id| {
                let id = id.unwrap().get().unwrap();
                let result = result.clone();
                let c = glib::MainContext::default();
                c.spawn_local(async move {
                    result.backend.delete_recording(id).await.unwrap();
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
        self.navigator.reset();
        self.leaflet.set_visible_child(&self.sidebar_box);
    }
}
