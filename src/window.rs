use super::backend::Backend;
use super::database::*;
use super::dialogs::*;
use gio::prelude::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::{action, get_widget};
use libhandy::prelude::*;
use libhandy::HeaderBarExt;
use std::cell::RefCell;
use std::convert::TryInto;
use std::rc::Rc;

pub struct Window {
    window: libhandy::ApplicationWindow,
    backend: Rc<Backend>,
    leaflet: libhandy::Leaflet,
    persons: RefCell<Vec<Person>>,
    works: RefCell<Vec<WorkDescription>>,
    recordings: RefCell<Vec<RecordingDescription>>,
    sidebar_box: gtk::Box,
    person_search_entry: gtk::SearchEntry,
    person_list: gtk::ListBox,
    stack: gtk::Stack,
    header: libhandy::HeaderBar,
    header_menu_button: gtk::MenuButton,
    work_box: gtk::Box,
    work_list: gtk::ListBox,
    recording_box: gtk::Box,
    recording_list: gtk::ListBox,
}

impl Window {
    pub fn new(app: &gtk::Application) -> Rc<Self> {
        let builder = gtk::Builder::from_resource("/de/johrpan/musicus_editor/ui/window.ui");

        get_widget!(builder, libhandy::ApplicationWindow, window);
        get_widget!(builder, libhandy::Leaflet, leaflet);
        get_widget!(builder, gtk::Box, sidebar_box);
        get_widget!(builder, gtk::SearchEntry, person_search_entry);
        get_widget!(builder, gtk::ListBox, person_list);
        get_widget!(builder, gtk::Stack, stack);
        get_widget!(builder, libhandy::HeaderBar, header);
        get_widget!(builder, gtk::MenuButton, header_menu_button);
        get_widget!(builder, gtk::Box, work_box);
        get_widget!(builder, gtk::ListBox, work_list);
        get_widget!(builder, gtk::Box, recording_box);
        get_widget!(builder, gtk::ListBox, recording_list);

        let backend = Backend::new("test.sqlite");

        let result = Rc::new(Window {
            window: window,
            backend: Rc::new(backend),
            leaflet: leaflet,
            persons: RefCell::new(Vec::new()),
            works: RefCell::new(Vec::new()),
            recordings: RefCell::new(Vec::new()),
            sidebar_box: sidebar_box,
            person_list: person_list,
            person_search_entry: person_search_entry,
            stack: stack,
            header: header,
            header_menu_button: header_menu_button,
            work_box: work_box,
            work_list: work_list,
            recording_box: recording_box,
            recording_list: recording_list,
        });

        result
            .person_list
            .connect_row_activated(clone!(@strong result => move |_, row| {
                let row = row.get_child().unwrap().downcast::<SelectorRow>().unwrap();
                let index: usize = row.get_index().try_into().unwrap();
                let person = result.persons.borrow()[index].clone();
                result.show_person(person);
            }));

        result
            .person_list
            .set_filter_func(Some(Box::new(clone!(@strong result => move |row| {
                let row = row.get_child().unwrap().downcast::<SelectorRow>().unwrap();
                let index: usize = row.get_index().try_into().unwrap();
                let search = result.person_search_entry.get_text().to_string().to_lowercase();

                search.is_empty() || result.persons.borrow()[index]
                    .name_lf()
                    .to_lowercase()
                    .contains(&search)
            }))));

        result
            .person_search_entry
            .connect_search_changed(clone!(@strong result => move |_| {
                result.person_list.invalidate_filter();
            }));

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
                    result.backend.get_persons(clone!(@strong result => move |persons| {
                        result.persons.replace(persons);
                        result.show_persons();
                    }));
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
                    result.backend.get_persons(clone!(@strong result => move |persons| {
                        result.persons.replace(persons);
                        result.show_persons();
                    }));
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

        action!(result.window, "add-recording", |_, _| {
            println!("TODO: Add recording.");
        });

        action!(
            result.window,
            "edit-person",
            Some(glib::VariantTy::new("x").unwrap()),
            clone!(@strong result => move |_, id| {
                result.backend.get_person(id.unwrap().get().unwrap(), clone!(@strong result => move |person| {
                    let person = person.unwrap();
                    PersonEditor::new(result.backend.clone(), &result.window, Some(person), clone!(@strong result => move |person| {
                        result.backend.get_persons(clone!(@strong result => move |persons| {
                            result.persons.replace(persons);
                            result.show_persons();
                        }));
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
                    result.back();
                    result.backend.get_persons(clone!(@strong result => move |persons| {
                        result.persons.replace(persons);
                        result.show_persons();
                    }));
                }));
            })
        );

        result.window.set_application(Some(app));
        result.show_persons();

        result
    }

    pub fn present(&self) {
        self.window.present();
    }

    fn show_persons(&self) {
        for child in self.person_list.get_children() {
            self.person_list.remove(&child);
        }

        for (index, person) in self.persons.borrow().iter().enumerate() {
            let label = gtk::Label::new(Some(&person.name_lf()));
            label.set_halign(gtk::Align::Start);
            let row = SelectorRow::new(index.try_into().unwrap(), &label);
            row.show_all();
            self.person_list.insert(&row, -1);
        }
    }

    fn show_person(&self, person: Person) {
        self.header.set_title(Some(&person.name_fl()));
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

        self.header_menu_button.set_menu_model(Some(&menu));

        // let result = self.clone();
        // self.backend.get_work_descriptions(person.id, |works| {
        //     result.show_works();
        //     result.show_recordings();
        //     result.stack.set_visible_child_name("person_screen");
        //     result.leaflet.set_visible_child(&result.stack);
        // });
    }

    fn show_works(&self) {
        for child in self.work_list.get_children() {
            self.work_list.remove(&child);
        }

        let works = self.works.borrow();

        if works.is_empty() {
            self.work_box.hide();
        } else {
            self.work_box.show();
        }

        for (index, work) in works.iter().enumerate() {
            let label = gtk::Label::new(Some(&work.title));
            label.set_halign(gtk::Align::Start);
            let row = SelectorRow::new(index.try_into().unwrap(), &label);
            row.show_all();
            self.work_list.insert(&row, -1);
        }
    }

    fn show_recordings(&self) {
        for child in self.recording_list.get_children() {
            self.recording_list.remove(&child);
        }

        let recordings = self.recordings.borrow();

        if recordings.is_empty() {
            self.recording_box.hide();
        } else {
            self.recording_box.show();
        }
    }

    fn back(&self) {
        self.stack.set_visible_child_name("empty_screen");
        self.leaflet.set_visible_child(&self.sidebar_box);
    }
}
