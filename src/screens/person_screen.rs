use super::*;
use crate::backend::*;
use crate::database::*;
use crate::widgets::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use libhandy::HeaderBarExt;
use std::cell::RefCell;
use std::rc::Rc;

pub struct PersonScreen {
    backend: Rc<Backend>,
    widget: gtk::Box,
    stack: gtk::Stack,
    work_list: Rc<List<WorkDescription>>,
    recording_list: Rc<List<RecordingDescription>>,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl PersonScreen {
    pub fn new(backend: Rc<Backend>, person: Person) -> Rc<Self> {
        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/person_screen.ui");

        get_widget!(builder, gtk::Box, widget);
        get_widget!(builder, libhandy::HeaderBar, header);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::MenuButton, menu_button);
        get_widget!(builder, gtk::SearchEntry, search_entry);
        get_widget!(builder, gtk::Stack, stack);
        get_widget!(builder, gtk::Box, work_box);
        get_widget!(builder, gtk::Frame, work_frame);
        get_widget!(builder, gtk::Box, recording_box);
        get_widget!(builder, gtk::Frame, recording_frame);

        header.set_title(Some(&person.name_fl()));

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

        menu_button.set_menu_model(Some(&menu));

        let work_list = List::new(
            |work: &WorkDescription| {
                let label = gtk::Label::new(Some(&work.title));
                label.set_halign(gtk::Align::Start);
                label.upcast()
            },
            clone!(@strong search_entry => move |work: &WorkDescription| {
                let search = search_entry.get_text().to_string().to_lowercase();
                let title = work.title.to_lowercase();
                search.is_empty() || title.contains(&search)
            }),
            "No works found.",
        );

        let recording_list = List::new(
            |recording: &RecordingDescription| {
                let work_label = gtk::Label::new(Some(&recording.work.get_title()));

                work_label.set_ellipsize(pango::EllipsizeMode::End);
                work_label.set_halign(gtk::Align::Start);

                let performers_label = gtk::Label::new(Some(&recording.get_performers()));
                performers_label.set_ellipsize(pango::EllipsizeMode::End);
                performers_label.set_opacity(0.5);
                performers_label.set_halign(gtk::Align::Start);

                let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
                vbox.add(&work_label);
                vbox.add(&performers_label);

                vbox.upcast()
            },
            clone!(@strong search_entry => move |recording: &RecordingDescription| {
                let search = search_entry.get_text().to_string().to_lowercase();
                let text = recording.work.get_title() + &recording.get_performers();
                search.is_empty() || text.contains(&search)
            }),
            "No recordings found.",
        );

        work_frame.add(&work_list.widget);
        recording_frame.add(&recording_list.widget);

        let result = Rc::new(Self {
            backend,
            widget,
            stack,
            work_list,
            recording_list,
            navigator: RefCell::new(None),
        });

        search_entry.connect_search_changed(clone!(@strong result => move |_| {
            result.work_list.invalidate_filter();
            result.recording_list.invalidate_filter();
        }));

        back_button.connect_clicked(clone!(@strong result => move |_| {
            let navigator = result.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                navigator.clone().pop();
            }
        }));

        result
            .work_list
            .set_selected(clone!(@strong result => move |work| {
                result.recording_list.clear_selection();
                let navigator = result.navigator.borrow().clone();
                if let Some(navigator) = navigator {
                    navigator.push(WorkScreen::new(result.backend.clone(), work.clone()));
                }
            }));

        result
            .recording_list
            .set_selected(clone!(@strong result => move |recording| {
                result.work_list.clear_selection();
                let navigator = result.navigator.borrow().clone();
                if let Some(navigator) = navigator {
                    navigator.push(RecordingScreen::new(result.backend.clone(), recording.clone()));
                }
            }));

        let context = glib::MainContext::default();
        let clone = result.clone();
        context.spawn_local(async move {
            let works = clone
                .backend
                .get_work_descriptions(person.id)
                .await
                .unwrap();
            let recordings = clone
                .backend
                .get_recordings_for_person(person.id)
                .await
                .unwrap();

            if works.is_empty() && recordings.is_empty() {
                clone.stack.set_visible_child_name("nothing");
            } else {
                if works.is_empty() {
                    work_box.hide();
                } else {
                    clone.work_list.show_items(works);
                }

                if recordings.is_empty() {
                    recording_box.hide();
                } else {
                    clone.recording_list.show_items(recordings);
                }

                clone.stack.set_visible_child_name("content");
            }
        });

        result
    }
}

impl NavigatorScreen for PersonScreen {
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
