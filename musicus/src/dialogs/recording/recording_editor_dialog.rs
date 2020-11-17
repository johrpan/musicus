use super::recording_editor::*;
use crate::backend::*;
use crate::database::*;
use glib::clone;
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// A dialog for creating or editing a recording.
pub struct RecordingEditorDialog {
    pub window: libhandy::Window,
    selected_cb: RefCell<Option<Box<dyn Fn(Recording) -> ()>>>,
}

impl RecordingEditorDialog {
    /// Create a new recording editor dialog and optionally initialize it.
    pub fn new<W: IsA<gtk::Window>>(
        backend: Rc<Backend>,
        parent: &W,
        recording: Option<Recording>,
    ) -> Rc<Self> {
        // Create UI

        let window = libhandy::Window::new();
        window.set_type_hint(gdk::WindowTypeHint::Dialog);
        window.set_modal(true);
        window.set_transient_for(Some(parent));

        let editor = RecordingEditor::new(backend.clone(), &window, recording);
        window.add(&editor.widget);
        window.show_all();

        let this = Rc::new(Self {
            window,
            selected_cb: RefCell::new(None),
        });

        // Connect signals and callbacks

        editor.set_back_cb(clone!(@strong this => move || {
            this.window.close();
        }));

        editor.set_selected_cb(clone!(@strong this => move |recording| {
            if let Some(cb) = &*this.selected_cb.borrow() {
                cb(recording);
            }
            
            this.window.close();
        }));

        this
    }

    /// Set the closure to be called when the user edited or created a recording.
    pub fn set_selected_cb<F: Fn(Recording) -> () + 'static>(&self, cb: F) {
        self.selected_cb.replace(Some(Box::new(cb)));
    }

    /// Show the recording editor dialog.
    pub fn show(&self) {
        self.window.show();
    }
}
