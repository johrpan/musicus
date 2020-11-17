use super::work_editor::*;
use crate::backend::*;
use crate::database::*;
use glib::clone;
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// A dialog for creating or editing a work.
pub struct WorkEditorDialog {
    pub window: libhandy::Window,
    saved_cb: RefCell<Option<Box<dyn Fn(Work) -> ()>>>,
}

impl WorkEditorDialog {
    /// Create a new work editor dialog and optionally initialize it.
    pub fn new<W: IsA<gtk::Window>>(
        backend: Rc<Backend>,
        parent: &W,
        work: Option<Work>,
    ) -> Rc<Self> {
        // Create UI

        let window = libhandy::Window::new();
        window.set_type_hint(gdk::WindowTypeHint::Dialog);
        window.set_modal(true);
        window.set_transient_for(Some(parent));

        let editor = WorkEditor::new(backend.clone(), &window, work);
        window.add(&editor.widget);
        window.show_all();

        let this = Rc::new(Self {
            window,
            saved_cb: RefCell::new(None),
        });

        // Connect signals and callbacks

        editor.set_cancel_cb(clone!(@strong this => move || {
            this.window.close();
        }));

        editor.set_saved_cb(clone!(@strong this => move |work| {
            if let Some(cb) = &*this.saved_cb.borrow() {
                cb(work);
            }

            this.window.close();
        }));

        this
    }

    /// Set the closure to be called when the user edited or created a work.
    pub fn set_saved_cb<F: Fn(Work) -> () + 'static>(&self, cb: F) {
        self.saved_cb.replace(Some(Box::new(cb)));
    }

    /// Show the work editor dialog.
    pub fn show(&self) {
        self.window.show();
    }
}
