use super::disc_source::DiscSource;
use super::track_set_editor::TrackSetEditor;
use crate::backend::Backend;
use crate::widgets::{Navigator, NavigatorScreen};
use crate::widgets::new_list::List;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use std::cell::RefCell;
use std::rc::Rc;

/// A dialog for editing metadata while importing music into the music library.
pub struct MediumEditor {
    backend: Rc<Backend>,
    source: Rc<DiscSource>,
    widget: gtk::Box,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl MediumEditor {
    /// Create a new medium editor.
    pub fn new(backend: Rc<Backend>, source: DiscSource) -> Rc<Self> {
        // Create UI

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/medium_editor.ui");

        get_widget!(builder, gtk::Box, widget);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::Button, add_button);
        get_widget!(builder, gtk::Frame, frame);

        let list = List::new("No recordings added.");
        frame.add(&list.widget);

        let this = Rc::new(Self {
            backend,
            source: Rc::new(source),
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
                let editor = TrackSetEditor::new(this.backend.clone(), Rc::clone(&this.source));
                navigator.push(editor);
            }
        }));

        this
    }
}

impl NavigatorScreen for MediumEditor {
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
