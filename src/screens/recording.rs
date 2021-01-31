use crate::backend::Backend;
use crate::database::Recording;
use crate::editors::RecordingEditor;
use crate::widgets::{List, Navigator, NavigatorScreen, NavigatorWindow, Screen, Section};

use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// A screen for showing a recording.
pub struct RecordingScreen {
    backend: Rc<Backend>,
    recording: Recording,
    widget: Screen,
    track_list: Rc<List>,
    recordings: RefCell<Vec<Recording>>,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl RecordingScreen {
    /// Create a new recording screen for the specified recording and load the
    /// contents asynchronously.
    pub fn new(backend: Rc<Backend>, recording: Recording) -> Rc<Self> {
        let widget = Screen::new();
        widget.set_title(&recording.work.get_title());
        widget.set_subtitle(&recording.get_performers());

        let track_list = List::new();

        let this = Rc::new(Self {
            backend,
            recording,
            widget,
            track_list,
            recordings: RefCell::new(Vec::new()),
            navigator: RefCell::new(None),
        });

        this.widget.set_back_cb(clone!(@strong this => move || {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                navigator.pop();
            }
        }));


        this.widget.add_action(&gettext("Edit recording"), clone!(@strong this => move || {
            let editor = RecordingEditor::new(this.backend.clone(), Some(this.recording.clone()));
            let window = NavigatorWindow::new(editor);
            window.show();
        }));

        this.widget.add_action(&gettext("Delete recording"), clone!(@strong this => move || {
            let context = glib::MainContext::default();
            let clone = this.clone();
            context.spawn_local(async move {
                clone.backend.db().delete_recording(&clone.recording.id).await.unwrap();
                clone.backend.library_changed();
            });
        }));

        this.widget.set_search_cb(clone!(@strong this => move || {
            this.track_list.invalidate_filter();
        }));

        // TODO: Implement.
        // this.track_list.set_make_widget_cb(clone!(@strong this => move |index| {
        // }));

        this.track_list.set_filter_cb(clone!(@strong this => move |index| {
            // TODO: Implement.
            // search.is_empty() || text.to_lowercase().contains(&search)
            true
        }));

        // Load the content asynchronously.

        let context = glib::MainContext::default();
        let clone = Rc::clone(&this);

        context.spawn_local(async move {
            // TODO: Implement.

            clone.widget.ready();
        });

        this
    }
}

impl NavigatorScreen for RecordingScreen {
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
