use super::*;
use crate::backend::*;
use crate::database::*;
use crate::editors::PersonEditor;
use crate::widgets::{List, Navigator, NavigatorScreen, NavigatorWindow};
use gio::prelude::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

pub struct PersonScreen {
    backend: Rc<Backend>,
    person: Person,
    widget: gtk::Box,
    stack: gtk::Stack,
    search_entry: gtk::SearchEntry,
    work_box: gtk::Box,
    work_list: Rc<List>,
    recording_box: gtk::Box,
    recording_list: Rc<List>,
    works: RefCell<Vec<Work>>,
    recordings: RefCell<Vec<Recording>>,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl PersonScreen {
    pub fn new(backend: Rc<Backend>, person: Person) -> Rc<Self> {
        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/person_screen.ui");

        get_widget!(builder, gtk::Box, widget);
        get_widget!(builder, gtk::Label, title_label);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::SearchEntry, search_entry);
        get_widget!(builder, gtk::Stack, stack);
        get_widget!(builder, gtk::Box, work_box);
        get_widget!(builder, gtk::Frame, work_frame);
        get_widget!(builder, gtk::Box, recording_box);
        get_widget!(builder, gtk::Frame, recording_frame);

        title_label.set_label(&person.name_fl());

        let edit_action = gio::SimpleAction::new("edit", None);
        let delete_action = gio::SimpleAction::new("delete", None);

        let actions = gio::SimpleActionGroup::new();
        actions.add_action(&edit_action);
        actions.add_action(&delete_action);

        widget.insert_action_group("widget", Some(&actions));

        let work_list = List::new();
        let recording_list = List::new();
        work_frame.set_child(Some(&work_list.widget));
        recording_frame.set_child(Some(&recording_list.widget));

        let this = Rc::new(Self {
            backend,
            person,
            widget,
            stack,
            search_entry,
            work_box,
            work_list,
            recording_box,
            recording_list,
            works: RefCell::new(Vec::new()),
            recordings: RefCell::new(Vec::new()),
            navigator: RefCell::new(None),
        });

        this.search_entry.connect_search_changed(clone!(@strong this => move |_| {
            this.work_list.invalidate_filter();
            this.recording_list.invalidate_filter();
        }));

        back_button.connect_clicked(clone!(@strong this => move |_| {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                navigator.clone().pop();
            }
        }));

        this.work_list.set_make_widget_cb(clone!(@strong this => move |index| {
            let work = &this.works.borrow()[index];

            let row = libadwaita::ActionRow::new();
            row.set_activatable(true);
            row.set_title(Some(&work.title));

            let work = work.to_owned();
            row.connect_activated(clone!(@strong this => move |_| {
                let navigator = this.navigator.borrow().clone();
                if let Some(navigator) = navigator {
                    navigator.push(WorkScreen::new(this.backend.clone(), work.clone()));
                }
            }));

            row.upcast()
        }));

        this.work_list.set_filter_cb(clone!(@strong this => move |index| {
            let work = &this.works.borrow()[index];
            let search = this.search_entry.get_text().unwrap().to_string().to_lowercase();
            let title = work.title.to_lowercase();
            search.is_empty() || title.contains(&search)
        }));

        this.recording_list.set_make_widget_cb(clone!(@strong this => move |index| {
            let recording = &this.recordings.borrow()[index];

            let row = libadwaita::ActionRow::new();
            row.set_activatable(true);
            row.set_title(Some(&recording.work.get_title()));
            row.set_subtitle(Some(&recording.get_performers()));

            let recording = recording.to_owned();
            row.connect_activated(clone!(@strong this => move |_| {
                let navigator = this.navigator.borrow().clone();
                if let Some(navigator) = navigator {
                    navigator.push(RecordingScreen::new(this.backend.clone(), recording.clone()));
                }
            }));

            row.upcast()
        }));

        this.recording_list.set_filter_cb(clone!(@strong this => move |index| {
            let recording = &this.recordings.borrow()[index];
            let search = this.search_entry.get_text().unwrap().to_string().to_lowercase();
            let text = recording.work.get_title() + &recording.get_performers();
            search.is_empty() || text.contains(&search)
        }));

        edit_action.connect_activate(clone!(@strong this => move |_, _| {
            let editor = PersonEditor::new(this.backend.clone(), Some(this.person.clone()));
            let window = NavigatorWindow::new(editor);
            window.show();
        }));

        delete_action.connect_activate(clone!(@strong this => move |_, _| {
            let context = glib::MainContext::default();
            let clone = this.clone();
            context.spawn_local(async move {
                clone.backend.db().delete_person(&clone.person.id).await.unwrap();
                clone.backend.library_changed();
            });
        }));

        let context = glib::MainContext::default();
        let clone = this.clone();
        context.spawn_local(async move {
            let works = clone
                .backend
                .db()
                .get_works(&clone.person.id)
                .await
                .unwrap();

            let recordings = clone
                .backend
                .db()
                .get_recordings_for_person(&clone.person.id)
                .await
                .unwrap();

            if works.is_empty() && recordings.is_empty() {
                clone.stack.set_visible_child_name("nothing");
            } else {
                if works.is_empty() {
                    clone.work_box.hide();
                } else {
                    let length = works.len();
                    clone.works.replace(works);
                    clone.work_list.update(length);
                }

                if recordings.is_empty() {
                    clone.recording_box.hide();
                } else {
                    let length = recordings.len();
                    clone.recordings.replace(recordings);
                    clone.recording_list.update(length);
                }

                clone.stack.set_visible_child_name("content");
            }
        });

        this
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
