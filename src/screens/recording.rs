use crate::backend::Backend;
use crate::database::Recording;
use crate::editors::RecordingEditor;
use crate::navigator::{NavigatorWindow, NavigationHandle, Screen};
use crate::widgets;
use crate::widgets::{List, Section, Widget};
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// A screen for showing a recording.
pub struct RecordingScreen {
    handle: NavigationHandle<()>,
    recording: Recording,
    widget: widgets::Screen,
    track_list: Rc<List>,
    recordings: RefCell<Vec<Recording>>,
}

impl Screen<Recording, ()> for RecordingScreen {
    /// Create a new recording screen for the specified recording and load the
    /// contents asynchronously.
    fn new(recording: Recording, handle: NavigationHandle<()>) -> Rc<Self> {
        let widget = widgets::Screen::new();
        widget.set_title(&recording.work.get_title());
        widget.set_subtitle(&recording.get_performers());

        let track_list = List::new();

        let this = Rc::new(Self {
            handle,
            recording,
            widget,
            track_list,
            recordings: RefCell::new(Vec::new()),
        });

        this.widget.set_back_cb(clone!(@weak this => move || {
            this.handle.pop(None);
        }));


        this.widget.add_action(&gettext("Edit recording"), clone!(@weak this => move || {
            spawn!(@clone this, async move {
                let window = NavigatorWindow::new(this.handle.backend.clone());
                replace!(window.navigator, RecordingEditor, Some(this.recording.clone())).await;
            });
        }));

        this.widget.add_action(&gettext("Delete recording"), clone!(@weak this => move || {
            spawn!(@clone this, async move {
                this.handle.backend.db().delete_recording(&this.recording.id).await.unwrap();
                this.handle.backend.library_changed();
            });
        }));

        this.widget.set_search_cb(clone!(@weak this => move || {
            this.track_list.invalidate_filter();
        }));

        // TODO: Implement.
        // this.track_list.set_make_widget_cb(clone!(@strong this => move |index| {
        // }));

        this.track_list.set_filter_cb(clone!(@weak this => move |index| {
            // TODO: Implement.
            // search.is_empty() || text.to_lowercase().contains(&search)
            true
        }));

        // Load the content asynchronously.

        spawn!(@clone this, async move {
            // TODO: Implement.

            this.widget.ready();
        });

        this
    }
}

impl Widget for RecordingScreen {
    fn get_widget(&self) -> gtk::Widget {
        self.widget.widget.clone().upcast()
    }
}
