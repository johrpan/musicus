use super::{WorkScreen, RecordingScreen};

use crate::backend::Backend;
use crate::database::{Person, Recording, Work};
use crate::editors::PersonEditor;
use crate::navigator::NavigatorWindow;
use crate::widgets::{List, Navigator, NavigatorScreen, Screen, Section};

use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// A screen for showing works by and recordings with a person.
pub struct PersonScreen {
    backend: Rc<Backend>,
    person: Person,
    widget: Screen,
    work_list: Rc<List>,
    recording_list: Rc<List>,
    works: RefCell<Vec<Work>>,
    recordings: RefCell<Vec<Recording>>,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl PersonScreen {
    /// Create a new person screen for the specified person and load the
    /// contents asynchronously.
    pub fn new(backend: Rc<Backend>, person: Person) -> Rc<Self> {
        let widget = Screen::new();
        widget.set_title(&person.name_fl());

        let work_list = List::new();
        let recording_list = List::new();

        let this = Rc::new(Self {
            backend,
            person,
            widget,
            work_list,
            recording_list,
            works: RefCell::new(Vec::new()),
            recordings: RefCell::new(Vec::new()),
            navigator: RefCell::new(None),
        });

        this.widget.set_back_cb(clone!(@strong this => move || {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                navigator.pop();
            }
        }));


        this.widget.add_action(&gettext("Edit person"), clone!(@strong this => move || {
            spawn!(@clone this, async move {
                let window = NavigatorWindow::new(this.backend.clone());
                replace!(window.navigator, PersonEditor, None).await;
            });
        }));

        this.widget.add_action(&gettext("Delete person"), clone!(@strong this => move || {
            let context = glib::MainContext::default();
            let clone = this.clone();
            context.spawn_local(async move {
                clone.backend.db().delete_person(&clone.person.id).await.unwrap();
                clone.backend.library_changed();
            });
        }));

        this.widget.set_search_cb(clone!(@strong this => move || {
            this.work_list.invalidate_filter();
            this.recording_list.invalidate_filter();
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
            let search = this.widget.get_search();
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
            let search = this.widget.get_search();
            let text = recording.work.get_title() + &recording.get_performers();
            search.is_empty() || text.to_lowercase().contains(&search)
        }));

        // Load the content asynchronously.

        let context = glib::MainContext::default();
        let clone = Rc::clone(&this);

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

            if !works.is_empty() {
                let length = works.len();
                clone.works.replace(works);
                clone.work_list.update(length);

                let section = Section::new("Works", &clone.work_list.widget);
                clone.widget.add_content(&section.widget);
            }

            if !recordings.is_empty() {
                let length = recordings.len();
                clone.recordings.replace(recordings);
                clone.recording_list.update(length);

                let section = Section::new("Recordings", &clone.recording_list.widget);
                clone.widget.add_content(&section.widget);
            }

            clone.widget.ready();
        });

        this
    }
}

impl NavigatorScreen for PersonScreen {
    fn attach_navigator(&self, navigator: Rc<Navigator>) {
        self.navigator.replace(Some(navigator));
    }

    fn get_widget(&self) -> gtk::Widget {
        self.widget.widget.clone().upcast()
    }

    fn detach_navigator(&self) {
        self.navigator.replace(None);
    }
}
