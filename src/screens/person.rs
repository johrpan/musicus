use super::{WorkScreen, RecordingScreen};
use crate::backend::{Backend, Person, Recording, Work};
use crate::editors::PersonEditor;
use crate::navigator::{NavigatorWindow, NavigationHandle, Screen};
use crate::widgets;
use crate::widgets::{List, Section, Widget};
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// A screen for showing works by and recordings with a person.
pub struct PersonScreen {
    handle: NavigationHandle<()>,
    person: Person,
    widget: widgets::Screen,
    work_list: Rc<List>,
    recording_list: Rc<List>,
    works: RefCell<Vec<Work>>,
    recordings: RefCell<Vec<Recording>>,
}

impl Screen<Person, ()> for PersonScreen {
    /// Create a new person screen for the specified person and load the
    /// contents asynchronously.
    fn new(person: Person, handle: NavigationHandle<()>) -> Rc<Self> {
        let widget = widgets::Screen::new();
        widget.set_title(&person.name_fl());

        let work_list = List::new();
        let recording_list = List::new();

        let this = Rc::new(Self {
            handle,
            person,
            widget,
            work_list,
            recording_list,
            works: RefCell::new(Vec::new()),
            recordings: RefCell::new(Vec::new()),
        });

        this.widget.set_back_cb(clone!(@weak this => move || {
            this.handle.pop(None);
        }));


        this.widget.add_action(&gettext("Edit person"), clone!(@weak this => move || {
            spawn!(@clone this, async move {
                let window = NavigatorWindow::new(this.handle.backend.clone());
                replace!(window.navigator, PersonEditor, Some(this.person.clone())).await;
            });
        }));

        this.widget.add_action(&gettext("Delete person"), clone!(@weak this => move || {
            spawn!(@clone this, async move {
                this.handle.backend.db().delete_person(&this.person.id).await.unwrap();
                this.handle.backend.library_changed();
            });
        }));

        this.widget.set_search_cb(clone!(@weak this => move || {
            this.work_list.invalidate_filter();
            this.recording_list.invalidate_filter();
        }));

        this.work_list.set_make_widget_cb(clone!(@weak this => move |index| {
            let work = &this.works.borrow()[index];

            let row = libadwaita::ActionRow::new();
            row.set_activatable(true);
            row.set_title(Some(&work.title));

            let work = work.to_owned();
            row.connect_activated(clone!(@weak this => move |_| {
                let work = work.clone();
                spawn!(@clone this, async move {
                    push!(this.handle, WorkScreen, work.clone()).await;
                });
            }));

            row.upcast()
        }));

        this.work_list.set_filter_cb(clone!(@weak this => move |index| {
            let work = &this.works.borrow()[index];
            let search = this.widget.get_search();
            let title = work.title.to_lowercase();
            search.is_empty() || title.contains(&search)
        }));

        this.recording_list.set_make_widget_cb(clone!(@weak this => move |index| {
            let recording = &this.recordings.borrow()[index];

            let row = libadwaita::ActionRow::new();
            row.set_activatable(true);
            row.set_title(Some(&recording.work.get_title()));
            row.set_subtitle(Some(&recording.get_performers()));

            let recording = recording.to_owned();
            row.connect_activated(clone!(@weak this => move |_| {
                let recording = recording.clone();
                spawn!(@clone this, async move {
                    push!(this.handle, RecordingScreen, recording.clone()).await;
                });
            }));

            row.upcast()
        }));

        this.recording_list.set_filter_cb(clone!(@weak this => move |index| {
            let recording = &this.recordings.borrow()[index];
            let search = this.widget.get_search();
            let text = recording.work.get_title() + &recording.get_performers();
            search.is_empty() || text.to_lowercase().contains(&search)
        }));

        // Load the content asynchronously.

        spawn!(@clone this, async move {
            let works = this.handle
                .backend
                .db()
                .get_works(&this.person.id)
                .await
                .unwrap();

            let recordings = this.handle
                .backend
                .db()
                .get_recordings_for_person(&this.person.id)
                .await
                .unwrap();

            if !works.is_empty() {
                let length = works.len();
                this.works.replace(works);
                this.work_list.update(length);

                let section = Section::new("Works", &this.work_list.widget);
                this.widget.add_content(&section.widget);
            }

            if !recordings.is_empty() {
                let length = recordings.len();
                this.recordings.replace(recordings);
                this.recording_list.update(length);

                let section = Section::new("Recordings", &this.recording_list.widget);
                this.widget.add_content(&section.widget);
            }

            this.widget.ready();
        });

        this
    }
}

impl Widget for PersonScreen {
    fn get_widget(&self) -> gtk::Widget {
        self.widget.widget.clone().upcast()
    }
}
