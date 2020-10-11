use super::backend::Backend;
use super::database::*;
use super::dialogs::*;
use gio::prelude::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::{action, get_widget};
use libhandy::prelude::*;
use libhandy::HeaderBarExt;
use std::cell::{Cell, RefCell};
use std::convert::TryInto;
use std::rc::Rc;

#[derive(Clone)]
enum PersonOrEnsemble {
    Person(Person),
    Ensemble(Ensemble),
}

impl PersonOrEnsemble {
    pub fn get_title(&self) -> String {
        match self {
            PersonOrEnsemble::Person(person) => person.name_lf(),
            PersonOrEnsemble::Ensemble(ensemble) => ensemble.name.clone(),
        }
    }
}

#[derive(Clone)]
enum WindowState {
    Loading,
    Selection(Vec<PersonOrEnsemble>),
    OverviewScreenLoading(PersonOrEnsemble),
    OverviewScreen(
        PersonOrEnsemble,
        Vec<WorkDescription>,
        Vec<RecordingDescription>,
        String,
    ),
    WorkScreenLoading(PersonOrEnsemble, WorkDescription),
    WorkScreen(
        PersonOrEnsemble,
        WorkDescription,
        Vec<RecordingDescription>,
        String,
    ),
    RecordingScreenLoading(PersonOrEnsemble, RecordingDescription),
}

pub struct Window {
    window: libhandy::ApplicationWindow,
    state: RefCell<WindowState>,
    backend: Rc<Backend>,
    leaflet: libhandy::Leaflet,
    sidebar_stack: gtk::Stack,
    person_search_entry: gtk::SearchEntry,
    sidebar_list: gtk::ListBox,
    main_stack: gtk::Stack,
    overview_header: libhandy::HeaderBar,
    overview_header_menu_button: gtk::MenuButton,
    overview_search_entry: gtk::SearchEntry,
    overview_stack: gtk::Stack,
    overview_work_box: gtk::Box,
    overview_work_list: gtk::ListBox,
    overview_recording_box: gtk::Box,
    overview_recording_list: gtk::ListBox,
    work_details_header: libhandy::HeaderBar,
    work_details_stack: gtk::Stack,
    work_details_recording_list: gtk::ListBox,
    recording_details_header: libhandy::HeaderBar,
    recording_details_stack: gtk::Stack,
    sidebar_list_row_activated_handler_id: Cell<Option<glib::SignalHandlerId>>,
    overview_work_list_row_activated_handler_id: Cell<Option<glib::SignalHandlerId>>,
    overview_recording_list_row_activated_handler_id: Cell<Option<glib::SignalHandlerId>>,
    work_details_recording_list_row_activated_handler_id: Cell<Option<glib::SignalHandlerId>>,
}

impl Window {
    pub fn new(app: &gtk::Application) -> Rc<Self> {
        use WindowState::*;

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus_editor/ui/window.ui");

        get_widget!(builder, libhandy::ApplicationWindow, window);
        get_widget!(builder, libhandy::Leaflet, leaflet);
        get_widget!(builder, gtk::SearchEntry, person_search_entry);
        get_widget!(builder, gtk::Stack, sidebar_stack);
        get_widget!(builder, gtk::ListBox, sidebar_list);
        get_widget!(builder, gtk::Stack, main_stack);
        get_widget!(builder, libhandy::HeaderBar, overview_header);
        get_widget!(builder, gtk::MenuButton, overview_header_menu_button);
        get_widget!(builder, gtk::SearchEntry, overview_search_entry);
        get_widget!(builder, gtk::Stack, overview_stack);
        get_widget!(builder, gtk::Box, overview_work_box);
        get_widget!(builder, gtk::ListBox, overview_work_list);
        get_widget!(builder, gtk::Box, overview_recording_box);
        get_widget!(builder, gtk::ListBox, overview_recording_list);
        get_widget!(builder, libhandy::HeaderBar, work_details_header);
        get_widget!(builder, gtk::Button, work_details_back_button);
        get_widget!(builder, gtk::Stack, work_details_stack);
        get_widget!(builder, gtk::ListBox, work_details_recording_list);
        get_widget!(builder, libhandy::HeaderBar, recording_details_header);
        get_widget!(builder, gtk::Button, recording_details_back_button);
        get_widget!(builder, gtk::Stack, recording_details_stack);

        let backend = Backend::new("test.sqlite");

        let result = Rc::new(Window {
            window: window,
            state: RefCell::new(Loading),
            backend: Rc::new(backend),
            leaflet: leaflet,
            sidebar_stack: sidebar_stack,
            sidebar_list: sidebar_list,
            person_search_entry: person_search_entry,
            main_stack: main_stack,
            overview_header: overview_header,
            overview_header_menu_button: overview_header_menu_button,
            overview_search_entry: overview_search_entry,
            overview_stack: overview_stack,
            overview_work_box: overview_work_box,
            overview_work_list: overview_work_list,
            overview_recording_box: overview_recording_box,
            overview_recording_list: overview_recording_list,
            work_details_header: work_details_header,
            work_details_stack: work_details_stack,
            work_details_recording_list: work_details_recording_list,
            recording_details_header: recording_details_header,
            recording_details_stack: recording_details_stack,
            sidebar_list_row_activated_handler_id: Cell::new(None),
            overview_work_list_row_activated_handler_id: Cell::new(None),
            overview_recording_list_row_activated_handler_id: Cell::new(None),
            work_details_recording_list_row_activated_handler_id: Cell::new(None),
        });

        action!(
            result.window,
            "back",
            clone!(@strong result => move |_, _| {
                result.back();
            })
        );

        action!(
            result.window,
            "add-person",
            clone!(@strong result => move |_, _| {
                PersonEditor::new(result.backend.clone(), &result.window, None, clone!(@strong result => move |_| {
                   result.clone().set_state(Loading);
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
                    result.clone().set_state(Loading);
                })).show();
            })
        );

        action!(
            result.window,
            "add-ensemble",
            clone!(@strong result => move |_, _| {
                EnsembleEditor::new(result.backend.clone(), &result.window, None, |ensemble| {
                    println!("{:?}", ensemble);
                }).show();
            })
        );

        action!(
            result.window,
            "add-recording",
            clone!(@strong result => move |_, _| {
                RecordingEditor::new(result.backend.clone(), &result.window, None, clone!(@strong result => move |_| {
                    result.clone().set_state(Loading);
                })).show();
            })
        );

        action!(
            result.window,
            "edit-person",
            Some(glib::VariantTy::new("x").unwrap()),
            clone!(@strong result => move |_, id| {
                result.backend.get_person(id.unwrap().get().unwrap(), clone!(@strong result => move |person| {
                    let person = person.unwrap();
                    PersonEditor::new(result.backend.clone(), &result.window, Some(person), clone!(@strong result => move |_| {
                        result.clone().set_state(Loading);
                    })).show();
                }));
            })
        );

        action!(
            result.window,
            "delete-person",
            Some(glib::VariantTy::new("x").unwrap()),
            clone!(@strong result => move |_, id| {
                result.backend.delete_person(id.unwrap().get().unwrap(), clone!(@strong result => move |_| {
                    result.clone().set_state(Loading);
                }));
            })
        );

        action!(
            result.window,
            "edit-ensemble",
            Some(glib::VariantTy::new("x").unwrap()),
            clone!(@strong result => move |_, id| {
                result.backend.get_ensemble(id.unwrap().get().unwrap(), clone!(@strong result => move |ensemble| {
                    let ensemble = ensemble.unwrap();
                    EnsembleEditor::new(result.backend.clone(), &result.window, Some(ensemble), clone!(@strong result => move |_| {
                        result.clone().set_state(Loading);
                    })).show();
                }));
            })
        );

        action!(
            result.window,
            "delete-ensemble",
            Some(glib::VariantTy::new("x").unwrap()),
            clone!(@strong result => move |_, id| {
                result.backend.delete_ensemble(id.unwrap().get().unwrap(), clone!(@strong result => move |_| {
                    result.clone().set_state(Loading);
                }));
            })
        );

        result
            .person_search_entry
            .connect_search_changed(clone!(@strong result => move |_| {
                result.sidebar_list.invalidate_filter();
            }));

        result.overview_search_entry.connect_search_changed(clone!(@strong result => move |_| {
            match result.get_state() {
                OverviewScreen(poe, works, recordings, _) => {
                    result.clone().set_state(OverviewScreen(poe, works.clone(), recordings.clone(), result.overview_search_entry.get_text().to_string()));
                },
                _ => (),
            }
        }));

        work_details_back_button.connect_clicked(clone!(@strong result => move |_| {
            match result.get_state() {
                WorkScreenLoading(poe, _) => {
                    result.clone().set_state(OverviewScreenLoading(poe));
                },
                WorkScreen(poe, _, _, _) => {
                    result.clone().set_state(OverviewScreenLoading(poe));
                },
                _ => (),
            }
        }));

        recording_details_back_button.connect_clicked(clone!(@strong result => move |_| {
            match result.get_state() {
                RecordingScreenLoading(poe, _) => {
                    result.clone().set_state(OverviewScreenLoading(poe));
                },
                _ => (),
            }
        }));

        result.window.set_application(Some(app));
        result.clone().set_state(Loading);

        result
    }

    pub fn present(&self) {
        self.window.present();
    }

    fn get_state(&self) -> WindowState {
        self.state.borrow().clone()
    }

    fn set_state(self: Rc<Self>, state: WindowState) {
        use WindowState::*;

        self.state.replace(state.clone());

        match state {
            Loading => {
                self.backend
                    .get_persons(clone!(@strong self as self_ => move |persons| {
                        self_.backend.get_ensembles(clone!(@strong self_ => move |ensembles| {
                            let mut poes: Vec<PersonOrEnsemble> = Vec::new();

                            for person in &persons {
                                poes.push(PersonOrEnsemble::Person(person.clone()));
                            }

                            for ensemble in &ensembles {
                                poes.push(PersonOrEnsemble::Ensemble(ensemble.clone()));
                            }

                            self_.clone().set_state(Selection(poes));
                        }));
                    }));

                self.sidebar_stack.set_visible_child_name("loading");
                self.main_stack.set_visible_child_name("empty_screen");
                self.leaflet.set_visible_child_name("sidebar");
            }
            Selection(poes) => {
                for child in self.sidebar_list.get_children() {
                    self.sidebar_list.remove(&child);
                }

                for (index, poe) in poes.iter().enumerate() {
                    let label = gtk::Label::new(Some(&poe.get_title()));
                    label.set_ellipsize(pango::EllipsizeMode::End);
                    label.set_halign(gtk::Align::Start);
                    let row = SelectorRow::new(index.try_into().unwrap(), &label);
                    row.show_all();
                    self.sidebar_list.insert(&row, -1);
                }

                match self.sidebar_list_row_activated_handler_id.take() {
                    Some(id) => self.sidebar_list.disconnect(id),
                    None => (),
                }

                let handler_id = self.sidebar_list.connect_row_activated(
                    clone!(@strong self as self_, @strong poes => move |_, row| {
                        let row = row.get_child().unwrap().downcast::<SelectorRow>().unwrap();
                        let index: usize = row.get_index().try_into().unwrap();
                        let poe = poes[index].clone();
                        self_.clone().set_state(OverviewScreenLoading(poe));
                    }),
                );

                self.sidebar_list_row_activated_handler_id
                    .set(Some(handler_id));

                self.sidebar_list.set_filter_func(Some(Box::new(
                    clone!(@strong self as self_, @strong poes => move |row| {
                        let row = row.get_child().unwrap().downcast::<SelectorRow>().unwrap();
                        let index: usize = row.get_index().try_into().unwrap();
                        let search = self_.person_search_entry.get_text().to_string().to_lowercase();

                        search.is_empty() || poes[index]
                            .get_title()
                            .to_lowercase()
                            .contains(&search)
                    }),
                )));

                self.sidebar_stack.set_visible_child_name("content");
                self.main_stack.set_visible_child_name("empty_screen");
                self.leaflet.set_visible_child_name("sidebar");
            }
            OverviewScreenLoading(poe) => {
                match poe.clone() {
                    PersonOrEnsemble::Person(person) => {
                        self.overview_header.set_title(Some(&person.name_fl()));

                        let edit_menu_item = gio::MenuItem::new(Some("Edit person"), None);
                        edit_menu_item.set_action_and_target_value(
                            Some("win.edit-person"),
                            Some(&glib::Variant::from(person.id)),
                        );

                        let delete_menu_item = gio::MenuItem::new(Some("Delete person"), None);
                        delete_menu_item.set_action_and_target_value(
                            Some("win.delete-person"),
                            Some(&glib::Variant::from(person.id)),
                        );

                        let menu = gio::Menu::new();
                        menu.append_item(&edit_menu_item);
                        menu.append_item(&delete_menu_item);

                        self.overview_header_menu_button.set_menu_model(Some(&menu));

                        self.backend.get_work_descriptions(
                            person.id,
                            clone!(@strong self as self_, @strong poe => move |works| {
                                self_.backend.get_recordings_for_person(
                                    person.id,
                                    clone!(@strong self_, @strong poe => move |recordings| {
                                        self_.clone().set_state(OverviewScreen(poe.clone(), works.clone(), recordings, String::from("")));
                                    }),
                                );
                            }),
                        );
                    }
                    PersonOrEnsemble::Ensemble(ensemble) => {
                        self.overview_header.set_title(Some(&ensemble.name));

                        let edit_menu_item = gio::MenuItem::new(Some("Edit ensemble"), None);
                        edit_menu_item.set_action_and_target_value(
                            Some("win.edit-ensemble"),
                            Some(&glib::Variant::from(ensemble.id)),
                        );

                        let delete_menu_item = gio::MenuItem::new(Some("Delete ensemble"), None);
                        delete_menu_item.set_action_and_target_value(
                            Some("win.delete-ensemble"),
                            Some(&glib::Variant::from(ensemble.id)),
                        );

                        let menu = gio::Menu::new();
                        menu.append_item(&edit_menu_item);
                        menu.append_item(&delete_menu_item);

                        self.overview_header_menu_button.set_menu_model(Some(&menu));

                        self.backend.get_recordings_for_ensemble(
                            ensemble.id,
                            clone!(@strong self as self_ => move |recordings| {
                                self_.clone().set_state(OverviewScreen(poe.clone(), Vec::new(), recordings, String::from("")));
                            }),
                        );
                    }
                }

                self.overview_search_entry.set_text("");

                self.overview_stack.set_visible_child_name("loading");
                self.main_stack.set_visible_child_name("overview_screen");
                self.leaflet.set_visible_child_name("content");
            }
            OverviewScreen(poe, works, recordings, search) => {
                for child in self.overview_work_list.get_children() {
                    self.overview_work_list.remove(&child);
                }

                for child in self.overview_recording_list.get_children() {
                    self.overview_recording_list.remove(&child);
                }

                if works.is_empty() {
                    self.overview_work_box.hide();
                } else {
                    self.overview_work_box.show();
                }

                for (index, work) in works.iter().enumerate() {
                    if search.is_empty() || work.title.to_lowercase().contains(&search) {
                        let label = gtk::Label::new(Some(&work.title));
                        label.set_ellipsize(pango::EllipsizeMode::End);
                        label.set_halign(gtk::Align::Start);
                        let row = SelectorRow::new(index.try_into().unwrap(), &label);
                        row.show_all();
                        self.overview_work_list.insert(&row, -1);
                    }
                }

                match self.overview_work_list_row_activated_handler_id.take() {
                    Some(id) => self.overview_work_list.disconnect(id),
                    None => (),
                }

                let handler_id = self.overview_work_list.connect_row_activated(
                    clone!(@strong self as self_, @strong works, @strong poe => move |_, row| {
                        self_.overview_recording_list.unselect_all();

                        let row = row.get_child().unwrap().downcast::<SelectorRow>().unwrap();
                        let index: usize = row.get_index().try_into().unwrap();
                        let work = works[index].clone();

                        self_.clone().set_state(WorkScreenLoading(poe.clone(), work));
                    }),
                );

                self.overview_work_list_row_activated_handler_id
                    .set(Some(handler_id));

                if recordings.is_empty() {
                    self.overview_recording_box.hide();
                } else {
                    self.overview_recording_box.show();
                }

                for (index, recording) in recordings.iter().enumerate() {
                    let work_text = recording.work.get_title();
                    let performers_text = recording.get_performers();

                    if search.is_empty()
                        || (work_text.to_lowercase().contains(&search)
                            || performers_text.to_lowercase().contains(&search))
                    {
                        let work_label = gtk::Label::new(Some(&work_text));

                        work_label.set_ellipsize(pango::EllipsizeMode::End);
                        work_label.set_halign(gtk::Align::Start);

                        let performers_label = gtk::Label::new(Some(&performers_text));
                        performers_label.set_ellipsize(pango::EllipsizeMode::End);
                        performers_label.set_opacity(0.5);
                        performers_label.set_halign(gtk::Align::Start);

                        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
                        vbox.add(&work_label);
                        vbox.add(&performers_label);

                        let row = SelectorRow::new(index.try_into().unwrap(), &vbox);
                        row.show_all();
                        self.overview_recording_list.insert(&row, -1);
                    }
                }

                match self.overview_recording_list_row_activated_handler_id.take() {
                    Some(id) => self.overview_recording_list.disconnect(id),
                    None => (),
                }

                let handler_id = self.overview_recording_list.connect_row_activated(
                    clone!(@strong self as self_, @strong recordings, @strong poe => move |_, row| {
                        self_.overview_work_list.unselect_all();

                        let row = row.get_child().unwrap().downcast::<SelectorRow>().unwrap();
                        let index: usize = row.get_index().try_into().unwrap();
                        let recording = recordings[index].clone();

                        self_.clone().set_state(RecordingScreenLoading(poe.clone(), recording));
                    }),
                );

                self.overview_recording_list_row_activated_handler_id
                    .set(Some(handler_id));

                self.overview_stack.set_visible_child_name("content");
                self.main_stack.set_visible_child_name("overview_screen");
                self.leaflet.set_visible_child_name("content");
            }
            WorkScreenLoading(poe, work) => {
                self.work_details_header
                    .set_title(Some(&work.composer.name_fl()));
                self.work_details_header.set_subtitle(Some(&work.title));

                self.backend.get_recordings_for_work(
                    work.id,
                    clone!(@strong self as self_ => move |recordings| {
                        self_.clone().set_state(WorkScreen(poe.clone(), work.clone(), recordings, String::new()));
                    }),
                );

                self.work_details_stack.set_visible_child_name("loading");
                self.main_stack
                    .set_visible_child_name("work_details_screen");
                self.leaflet.set_visible_child_name("content");
            }
            WorkScreen(poe, work, recordings, search) => {
                for child in self.work_details_recording_list.get_children() {
                    self.work_details_recording_list.remove(&child);
                }

                for (index, recording) in recordings.iter().enumerate() {
                    let work_text = recording.work.get_title();
                    let performers_text = recording.get_performers();

                    if search.is_empty()
                        || (work_text.to_lowercase().contains(&search)
                            || performers_text.to_lowercase().contains(&search))
                    {
                        let work_label = gtk::Label::new(Some(&work_text));

                        work_label.set_ellipsize(pango::EllipsizeMode::End);
                        work_label.set_halign(gtk::Align::Start);

                        let performers_label = gtk::Label::new(Some(&performers_text));
                        performers_label.set_ellipsize(pango::EllipsizeMode::End);
                        performers_label.set_opacity(0.5);
                        performers_label.set_halign(gtk::Align::Start);

                        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
                        vbox.add(&work_label);
                        vbox.add(&performers_label);

                        let row = SelectorRow::new(index.try_into().unwrap(), &vbox);
                        row.show_all();
                        self.work_details_recording_list.insert(&row, -1);
                    }
                }

                match self.work_details_recording_list_row_activated_handler_id.take() {
                    Some(id) => self.work_details_recording_list.disconnect(id),
                    None => (),
                }

                let handler_id = self.work_details_recording_list.connect_row_activated(
                    clone!(@strong self as self_, @strong recordings, @strong poe => move |_, row| {
                        self_.overview_work_list.unselect_all();

                        let row = row.get_child().unwrap().downcast::<SelectorRow>().unwrap();
                        let index: usize = row.get_index().try_into().unwrap();
                        let recording = recordings[index].clone();

                        self_.clone().set_state(RecordingScreenLoading(poe.clone(), recording));
                    }),
                );

                self.work_details_recording_list_row_activated_handler_id
                    .set(Some(handler_id));

                self.work_details_stack.set_visible_child_name("content");
                self.main_stack
                    .set_visible_child_name("work_details_screen");
                self.leaflet.set_visible_child_name("content");
            }
            RecordingScreenLoading(poe, recording) => {
                self.recording_details_header
                    .set_title(Some(&recording.work.get_title()));
                self.recording_details_header
                    .set_subtitle(Some(&recording.get_performers()));

                self.recording_details_stack
                    .set_visible_child_name("loading");
                self.main_stack
                    .set_visible_child_name("recording_details_screen");
                self.leaflet.set_visible_child_name("content");
            }
        }
    }

    fn back(&self) {
        self.main_stack.set_visible_child_name("empty_screen");
        self.leaflet.set_visible_child_name("sidebar");
    }
}
