use super::work_editor::*;
use super::work_selector::*;
use crate::backend::*;
use crate::database::*;
use glib::clone;
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// A dialog for selecting and creating a work.
pub struct WorkDialog {
    pub window: libhandy::Window,
    stack: gtk::Stack,
    selector: Rc<WorkSelector>,
    editor: Rc<WorkEditor>,
    selected_cb: RefCell<Option<Box<dyn Fn(WorkDescription) -> ()>>>,
}

impl WorkDialog {
    /// Create a new work dialog.
    pub fn new<W: IsA<gtk::Window>>(backend: Rc<Backend>, parent: &W) -> Rc<Self> {
        // Create UI

        let window = libhandy::Window::new();
        window.set_type_hint(gdk::WindowTypeHint::Dialog);
        window.set_modal(true);
        window.set_transient_for(Some(parent));
        window.set_default_size(600, 424);

        let selector = WorkSelector::new(backend.clone());
        let editor = WorkEditor::new(backend.clone(), &window, None);

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
            .set_selected_cb(clone!(@strong this => move |work| {
                if let Some(cb) = &*this.selected_cb.borrow() {
                    cb(work);
                    this.window.close();
                }
            }));

        this.editor.set_cancel_cb(clone!(@strong this => move || {
            this.stack.set_visible_child(&this.selector.widget);
        }));

        this.editor
            .set_saved_cb(clone!(@strong this => move |work| {
                if let Some(cb) = &*this.selected_cb.borrow() {
                    cb(work);
                    this.window.close();
                }
            }));

        this
    }

    /// Set the closure to be called when the user has selected or created a work.
    pub fn set_selected_cb<F: Fn(WorkDescription) -> () + 'static>(&self, cb: F) {
        self.selected_cb.replace(Some(Box::new(cb)));
    }

    /// Show the work dialog.
    pub fn show(&self) {
        self.window.show();
    }
}
