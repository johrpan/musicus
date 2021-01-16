use super::selector::Selector;
use crate::backend::Backend;
use crate::database::{Recording, Work};
use crate::editors::RecordingEditor;
use crate::widgets::{Navigator, NavigatorScreen};
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// A screen for selecting a recording.
pub struct RecordingSelector {
    backend: Rc<Backend>,
    work: Work,
    selector: Rc<Selector<Recording>>,
    selected_cb: RefCell<Option<Box<dyn Fn(&Recording) -> ()>>>,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl RecordingSelector {
    /// Create a new recording selector for recordings of a specific work.
    pub fn new(backend: Rc<Backend>, work: Work) -> Rc<Self> {
        // Create UI

        let selector = Selector::<Recording>::new();
        selector.set_title(&gettext("Select recording"));
        selector.set_subtitle(&work.get_title());

        let this = Rc::new(Self {
            backend,
            work,
            selector,
            selected_cb: RefCell::new(None),
            navigator: RefCell::new(None),
        });

        // Connect signals and callbacks

        this.selector.set_back_cb(clone!(@strong this => move || {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                navigator.pop();
            }
        }));

        this.selector.set_add_cb(clone!(@strong this => move || {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                let recording = Recording::new(this.work.clone());

                let editor = RecordingEditor::new(this.backend.clone(), Some(recording));
                
                editor
                    .set_selected_cb(clone!(@strong this, @strong navigator => move |recording| {
                        navigator.clone().pop();
                        this.select(&recording);
                    }));
                
                navigator.push(editor);
            }
        }));

        this.selector
            .set_load_online(clone!(@strong this => move || {
                let clone = this.clone();
                async move { clone.backend.get_recordings_for_work(&clone.work.id).await }
            }));

        this.selector
            .set_load_local(clone!(@strong this => move || {
                let clone = this.clone();
                async move { clone.backend.db().get_recordings_for_work(&clone.work.id).await.unwrap() }
            }));

        this.selector.set_make_widget(|recording| {
            let label = gtk::Label::new(Some(&recording.get_performers()));
            label.set_halign(gtk::Align::Start);
            label.set_margin_start(6);
            label.set_margin_end(6);
            label.set_margin_top(6);
            label.set_margin_bottom(6);
            label.upcast()
        });

        this.selector.set_filter(|search, recording| {
            recording.get_performers().to_lowercase().contains(search)
        });

        this.selector
            .set_selected_cb(clone!(@strong this => move |recording| this.select(recording)));

        this
    }

    /// Set the closure to be called when an item is selected.
    pub fn set_selected_cb<F: Fn(&Recording) -> () + 'static>(&self, cb: F) {
        self.selected_cb.replace(Some(Box::new(cb)));
    }

    /// Select a recording.
    fn select(&self, recording: &Recording) {
        if let Some(cb) = &*self.selected_cb.borrow() {
            cb(&recording);
        }
    }
}

impl NavigatorScreen for RecordingSelector {
    fn attach_navigator(&self, navigator: Rc<Navigator>) {
        self.navigator.replace(Some(navigator));
    }

    fn get_widget(&self) -> gtk::Widget {
        self.selector.widget.clone().upcast()
    }

    fn detach_navigator(&self) {
        self.navigator.replace(None);
    }
}
