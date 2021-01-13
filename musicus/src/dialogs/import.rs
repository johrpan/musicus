use crate::backend::Backend;
use crate::editors::{TrackSetEditor, TrackSource};
use crate::widgets::{Navigator, NavigatorScreen};
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use std::cell::RefCell;
use std::rc::Rc;

/// A dialog for editing metadata while importing music into the music library.
pub struct ImportDialog {
    backend: Rc<Backend>,
    source: Rc<TrackSource>,
    widget: gtk::Box,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl ImportDialog {
    /// Create a new import dialog.
    pub fn new(backend: Rc<Backend>, source: Rc<TrackSource>) -> Rc<Self> {
        // Create UI

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/import_dialog.ui");

        get_widget!(builder, gtk::Box, widget);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::Button, add_button);

        let this = Rc::new(Self {
            backend,
            source,
            widget,
            navigator: RefCell::new(None),
        });

        // Connect signals and callbacks

        back_button.connect_clicked(clone!(@strong this => move |_| {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                navigator.pop();
            }
        }));

        add_button.connect_clicked(clone!(@strong this => move |_| {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                let editor = TrackSetEditor::new(this.backend.clone(), this.source.clone());
                navigator.push(editor);
            }
        }));

        this
    }
}

impl NavigatorScreen for ImportDialog {
    fn attach_navigator(&self, navigator: Rc<Navigator>) {
        self.navigator.replace(Some(navigator));
    }

    fn get_widget(&self) -> gtk::Widget {
        self.widget.clone().upcast()
    }

    fn detach_navigator(&self) {
        self.navigator.replace(None);
    }
}
