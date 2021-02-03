use super::RecordingScreen;

use crate::backend::Backend;
use crate::database::{Work, Recording};
use crate::editors::WorkEditor;
use crate::navigator::NavigatorWindow;
use crate::widgets::{List, Navigator, NavigatorScreen, Screen, Section};

use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// A screen for showing recordings of a work.
pub struct WorkScreen {
    backend: Rc<Backend>,
    work: Work,
    widget: Screen,
    recording_list: Rc<List>,
    recordings: RefCell<Vec<Recording>>,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl WorkScreen {
    /// Create a new work screen for the specified work and load the
    /// contents asynchronously.
    pub fn new(backend: Rc<Backend>, work: Work) -> Rc<Self> {
        let widget = Screen::new();
        widget.set_title(&work.title);
        widget.set_subtitle(&work.composer.name_fl());

        let recording_list = List::new();

        let this = Rc::new(Self {
            backend,
            work,
            widget,
            recording_list,
            recordings: RefCell::new(Vec::new()),
            navigator: RefCell::new(None),
        });

        this.widget.set_back_cb(clone!(@strong this => move || {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                navigator.pop();
            }
        }));


        this.widget.add_action(&gettext("Edit work"), clone!(@strong this => move || {
            spawn!(@clone this, async move {
                let window = NavigatorWindow::new(this.backend.clone());
                replace!(window.navigator, WorkEditor, None).await;
            });
        }));

        this.widget.add_action(&gettext("Delete work"), clone!(@strong this => move || {
            let context = glib::MainContext::default();
            let clone = this.clone();
            context.spawn_local(async move {
                clone.backend.db().delete_work(&clone.work.id).await.unwrap();
                clone.backend.library_changed();
            });
        }));

        this.widget.set_search_cb(clone!(@strong this => move || {
            this.recording_list.invalidate_filter();
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
            let recordings = clone
                .backend
                .db()
                .get_recordings_for_work(&clone.work.id)
                .await
                .unwrap();

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

impl NavigatorScreen for WorkScreen {
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
