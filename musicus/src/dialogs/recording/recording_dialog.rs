use super::recording_editor::*;
use super::recording_selector::*;
use crate::backend::*;
use crate::database::*;
use glib::clone;
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// A dialog for selecting and creating a recording.
pub struct RecordingDialog {
    pub window: libhandy::Window,
    stack: gtk::Stack,
    selector: Rc<RecordingSelector>,
    editor: Rc<RecordingEditor>,
    selected_cb: RefCell<Option<Box<dyn Fn(Recording) -> ()>>>,
}

impl RecordingDialog {
    /// Create a new recording dialog.
    pub fn new<W: IsA<gtk::Window>>(backend: Rc<Backend>, parent: &W) -> Rc<Self> {
        // Create UI

        let window = libhandy::Window::new();
        window.set_type_hint(gdk::WindowTypeHint::Dialog);
        window.set_modal(true);
        window.set_transient_for(Some(parent));
        window.set_default_size(600, 424);

        let selector = RecordingSelector::new(backend.clone());
        let editor = RecordingEditor::new(backend.clone(), &window, None);

        let stack = gtk::Stack::new();
        stack.set_transition_type(gtk::StackTransitionType::Crossfade);
        stack.add(&selector.widget);
        stack.add(&editor.widget);
        window.add(&stack);
        window.show_all();

        let this = Rc::new(Self {
            window,
            stack,
            selector,
            editor,
            selected_cb: RefCell::new(None),
        });

        // Connect signals and callbacks

        this.selector.set_add_cb(clone!(@strong this => move || {
            this.stack.set_visible_child(&this.editor.widget);
        }));

        this.selector
            .set_selected_cb(clone!(@strong this => move |recording| {
                if let Some(cb) = &*this.selected_cb.borrow() {
                    cb(recording);
                    this.window.close();
                }
            }));

        this.editor.set_back_cb(clone!(@strong this => move || {
            this.stack.set_visible_child(&this.selector.widget);
        }));

        this.editor
            .set_selected_cb(clone!(@strong this => move |recording| {
                if let Some(cb) = &*this.selected_cb.borrow() {
                    cb(recording);
                    this.window.close();
                }
            }));

        this
    }

    /// Set the closure to be called when the user has selected or created a recording.
    pub fn set_selected_cb<F: Fn(Recording) -> () + 'static>(&self, cb: F) {
        self.selected_cb.replace(Some(Box::new(cb)));
    }

    /// Show the recording dialog.
    pub fn show(&self) {
        self.window.show();
    }
}
