use super::backend::Backend;
use super::database::*;
use super::dialogs::*;
use gio::prelude::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::{action, get_widget};
use libhandy::prelude::*;
use libhandy::HeaderBarExt;
use std::cell::Cell;
use std::convert::TryInto;
use std::rc::Rc;

enum WindowState {
    Loading,
    Persons(Vec<Person>),
    PersonLoading(Person),
    Person(Vec<WorkDescription>, Vec<RecordingDescription>),
}

pub struct Window {
    window: libhandy::ApplicationWindow,
    backend: Rc<Backend>,
    leaflet: libhandy::Leaflet,
    sidebar_stack: gtk::Stack,
    person_search_entry: gtk::SearchEntry,
    person_list: gtk::ListBox,
    stack: gtk::Stack,
    header: libhandy::HeaderBar,
    header_menu_button: gtk::MenuButton,
    content_stack: gtk::Stack,
    work_box: gtk::Box,
    work_list: gtk::ListBox,
    recording_box: gtk::Box,
    recording_list: gtk::ListBox,
    actions_revealer: gtk::Revealer,
    edit_button: gtk::Button,
    delete_button: gtk::Button,
    person_list_row_activated_handler_id: Cell<Option<glib::SignalHandlerId>>,
    work_list_row_activated_handler_id: Cell<Option<glib::SignalHandlerId>>,
    recording_list_row_activated_handler_id: Cell<Option<glib::SignalHandlerId>>,
    edit_button_clicked_handler_id: Cell<Option<glib::SignalHandlerId>>,
    delete_button_clicked_handler_id: Cell<Option<glib::SignalHandlerId>>,
}

impl Window {
    pub fn new(app: &gtk::Application) -> Rc<Self> {
        use WindowState::*;

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus_editor/ui/window.ui");

        get_widget!(builder, libhandy::ApplicationWindow, window);
        get_widget!(builder, libhandy::Leaflet, leaflet);
        get_widget!(builder, gtk::SearchEntry, person_search_entry);
        get_widget!(builder, gtk::Stack, sidebar_stack);
        get_widget!(builder, gtk::ListBox, person_list);
        get_widget!(builder, gtk::Stack, stack);
        get_widget!(builder, libhandy::HeaderBar, header);
        get_widget!(builder, gtk::MenuButton, header_menu_button);
        get_widget!(builder, gtk::Stack, content_stack);
        get_widget!(builder, gtk::Box, work_box);
        get_widget!(builder, gtk::ListBox, work_list);
        get_widget!(builder, gtk::Box, recording_box);
        get_widget!(builder, gtk::ListBox, recording_list);
        get_widget!(builder, gtk::Revealer, actions_revealer);
        get_widget!(builder, gtk::Button, edit_button);
        get_widget!(builder, gtk::Button, delete_button);

        let backend = Backend::new("test.sqlite");

        let result = Rc::new(Window {
            window: window,
            backend: Rc::new(backend),
            leaflet: leaflet,
            sidebar_stack: sidebar_stack,
            person_list: person_list,
            person_search_entry: person_search_entry,
            stack: stack,
            header: header,
            header_menu_button: header_menu_button,
            content_stack: content_stack,
            work_box: work_box,
            work_list: work_list,
            recording_box: recording_box,
            recording_list: recording_list,
            actions_revealer: actions_revealer,
            edit_button: edit_button,
            delete_button: delete_button,
            person_list_row_activated_handler_id: Cell::new(None),
            work_list_row_activated_handler_id: Cell::new(None),
            recording_list_row_activated_handler_id: Cell::new(None),
            edit_button_clicked_handler_id: Cell::new(None),
            delete_button_clicked_handler_id: Cell::new(None),
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

        result
            .person_search_entry
            .connect_search_changed(clone!(@strong result => move |_| {
                result.person_list.invalidate_filter();
            }));

        result.window.set_application(Some(app));
        result.clone().set_state(Loading);

        result
    }

    pub fn present(&self) {
        self.window.present();
    }

    fn set_state(self: Rc<Self>, state: WindowState) {
        use WindowState::*;

        match state {
            Loading => {
                self.backend
                    .get_persons(clone!(@strong self as self_ => move |persons| {
                        self_.clone().set_state(Persons(persons));
                    }));
                
                self.actions_revealer.set_reveal_child(false);
                self.sidebar_stack.set_visible_child_name("loading");
                self.stack.set_visible_child_name("empty_screen");
                self.leaflet.set_visible_child_name("sidebar");
            }
            Persons(persons) => {
                for child in self.person_list.get_children() {
                    self.person_list.remove(&child);
                }

                for (index, person) in persons.iter().enumerate() {
                    let label = gtk::Label::new(Some(&person.name_lf()));
                    label.set_ellipsize(pango::EllipsizeMode::End);
                    label.set_halign(gtk::Align::Start);
                    let row = SelectorRow::new(index.try_into().unwrap(), &label);
                    row.show_all();
                    self.person_list.insert(&row, -1);
                }

                match self.person_list_row_activated_handler_id.take() {
                    Some(id) => self.person_list.disconnect(id),
                    None => (),
                }

                let handler_id = self.person_list.connect_row_activated(
                    clone!(@strong self as self_, @strong persons => move |_, row| {
                        let row = row.get_child().unwrap().downcast::<SelectorRow>().unwrap();
                        let index: usize = row.get_index().try_into().unwrap();
                        let person = persons[index].clone();
                        self_.clone().set_state(PersonLoading(person));
                    }),
                );

                self.person_list_row_activated_handler_id
                    .set(Some(handler_id));

                self.person_list.set_filter_func(Some(Box::new(
                    clone!(@strong self as self_, @strong persons => move |row| {
                        let row = row.get_child().unwrap().downcast::<SelectorRow>().unwrap();
                        let index: usize = row.get_index().try_into().unwrap();
                        let search = self_.person_search_entry.get_text().to_string().to_lowercase();

                        search.is_empty() || persons[index]
                            .name_lf()
                            .to_lowercase()
                            .contains(&search)
                    }),
                )));


                self.actions_revealer.set_reveal_child(false);
                self.sidebar_stack.set_visible_child_name("persons_list");
                self.stack.set_visible_child_name("empty_screen");
                self.leaflet.set_visible_child_name("sidebar");
            }
            PersonLoading(person) => {
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

                self.backend.get_work_descriptions(
                    person.id,
                    clone!(@strong self as self_ => move |works| {
                        self_.backend.get_recordings_for_person(
                            person.id,
                            clone!(@strong self_ => move |recordings| {
                                self_.clone().set_state(Person(works.clone(), recordings));
                            }),
                        );
                    }),
                );

                self.actions_revealer.set_reveal_child(false);
                self.content_stack.set_visible_child_name("loading");
                self.stack.set_visible_child_name("person_screen");
                self.leaflet.set_visible_child_name("content");
            }
            Person(works, recordings) => {
                for child in self.work_list.get_children() {
                    self.work_list.remove(&child);
                }

                for child in self.recording_list.get_children() {
                    self.recording_list.remove(&child);
                }

                if works.is_empty() {
                    self.work_box.hide();
                } else {
                    self.work_box.show();
                }

                for (index, work) in works.iter().enumerate() {
                    let label = gtk::Label::new(Some(&work.title));
                    label.set_ellipsize(pango::EllipsizeMode::End);
                    label.set_halign(gtk::Align::Start);
                    let row = SelectorRow::new(index.try_into().unwrap(), &label);
                    row.show_all();
                    self.work_list.insert(&row, -1);
                }

                match self.work_list_row_activated_handler_id.take() {
                    Some(id) => self.work_list.disconnect(id),
                    None => (),
                }

                let handler_id = self.work_list.connect_row_activated(
                    clone!(@strong self as self_, @strong works => move |_, row| {
                        self_.recording_list.unselect_all();

                        let row = row.get_child().unwrap().downcast::<SelectorRow>().unwrap();
                        let index: usize = row.get_index().try_into().unwrap();
                        let work = works[index].clone();

                        match self_.edit_button_clicked_handler_id.take() {
                            Some(id) => self_.edit_button.disconnect(id),
                            None => (),
                        }
        
                        let handler_id = self_.edit_button.connect_clicked(
                            clone!(@strong self_ => move |_| {
                                WorkEditor::new(self_.backend.clone(), &self_.window, Some(work.clone()), clone!(@strong self_ => move |_| {
                                        self_.clone().set_state(Loading);
                                    })).show();
                            }),
                        );
        
                        self_.edit_button_clicked_handler_id
                            .set(Some(handler_id));

                        match self_.delete_button_clicked_handler_id.take() {
                            Some(id) => self_.delete_button.disconnect(id),
                            None => (),
                        }
        
                        let handler_id = self_.delete_button.connect_clicked(
                            clone!(@strong self_ => move |_| {
                               // self_.backend.delete_work(work.id);
                            }),
                        );
        
                        self_.delete_button_clicked_handler_id
                            .set(Some(handler_id));

                        self_.actions_revealer.set_reveal_child(true);
                    }),
                );

                self.work_list_row_activated_handler_id
                    .set(Some(handler_id));

                if recordings.is_empty() {
                    self.recording_box.hide();
                } else {
                    self.recording_box.show();
                }

                for (index, recording) in recordings.iter().enumerate() {
                    let work_label = gtk::Label::new(Some(&format!(
                        "{}: {}",
                        recording.work.composer.name_fl(),
                        recording.work.title
                    )));

                    work_label.set_ellipsize(pango::EllipsizeMode::End);
                    work_label.set_halign(gtk::Align::Start);

                    let performers_label = gtk::Label::new(Some(&recording.get_performers()));
                    performers_label.set_ellipsize(pango::EllipsizeMode::End);
                    performers_label.set_opacity(0.5);
                    performers_label.set_halign(gtk::Align::Start);

                    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
                    vbox.add(&work_label);
                    vbox.add(&performers_label);

                    let row = SelectorRow::new(index.try_into().unwrap(), &vbox);
                    row.show_all();
                    self.recording_list.insert(&row, -1);
                }

                match self.recording_list_row_activated_handler_id.take() {
                    Some(id) => self.recording_list.disconnect(id),
                    None => (),
                }

                let handler_id = self.recording_list.connect_row_activated(
                    clone!(@strong self as self_, @strong recordings => move |_, row| {
                        self_.work_list.unselect_all();

                        let row = row.get_child().unwrap().downcast::<SelectorRow>().unwrap();
                        let index: usize = row.get_index().try_into().unwrap();
                        let recording = recordings[index].clone();

                        match self_.edit_button_clicked_handler_id.take() {
                            Some(id) => self_.edit_button.disconnect(id),
                            None => (),
                        }
        
                        let handler_id = self_.edit_button.connect_clicked(
                            clone!(@strong self_ => move |_| {
                                RecordingEditor::new(self_.backend.clone(), &self_.window, Some(recording.clone()), clone!(@strong self_ => move |_| {
                                        self_.clone().set_state(Loading);
                                    })).show();
                            }),
                        );
        
                        self_.edit_button_clicked_handler_id
                            .set(Some(handler_id));

                        match self_.delete_button_clicked_handler_id.take() {
                            Some(id) => self_.delete_button.disconnect(id),
                            None => (),
                        }
        
                        let handler_id = self_.delete_button.connect_clicked(
                            clone!(@strong self_ => move |_| {
                               // self_.backend.delete_recording(recording.id);
                            }),
                        );
        
                        self_.delete_button_clicked_handler_id
                            .set(Some(handler_id));

                        self_.actions_revealer.set_reveal_child(true);
                    }),
                );

                self.recording_list_row_activated_handler_id
                    .set(Some(handler_id));

                self.content_stack.set_visible_child_name("content");
                self.stack.set_visible_child_name("person_screen");
                self.leaflet.set_visible_child_name("content");
            }
        }
    }

    fn back(&self) {
        self.actions_revealer.set_reveal_child(false);
        self.stack.set_visible_child_name("empty_screen");
        self.leaflet.set_visible_child_name("sidebar");
    }
}
