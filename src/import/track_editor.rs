use crate::database::Recording;
use crate::widgets::{Navigator, NavigatorScreen};
use crate::widgets::new_list::List;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use libhandy::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// A screen for editing a single track.
pub struct TrackEditor {
    widget: gtk::Box,
    selection: RefCell<Vec<usize>>,
    selected_cb: RefCell<Option<Box<dyn Fn(Vec<usize>)>>>,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl TrackEditor {
    /// Create a new track editor.
    pub fn new(recording: Recording, selection: Vec<usize>) -> Rc<Self> {
        // Create UI

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/track_editor.ui");

        get_widget!(builder, gtk::Box, widget);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::Button, select_button);
        get_widget!(builder, gtk::Frame, parts_frame);

        let parts_list = gtk::ListBox::new();
        parts_list.set_selection_mode(gtk::SelectionMode::None);
        parts_list.set_vexpand(false);
        parts_list.show();
        parts_frame.add(&parts_list);

        let this = Rc::new(Self {
            widget,
            selection: RefCell::new(selection),
            selected_cb: RefCell::new(None),
            navigator: RefCell::new(None),
        });

        // Connect signals and callbacks

        back_button.connect_clicked(clone!(@strong this => move |_| {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                navigator.pop();
            }
        }));

        select_button.connect_clicked(clone!(@strong this => move |_| {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                navigator.pop();
            }

            if let Some(cb) = &*this.selected_cb.borrow() {
                let selection = this.selection.borrow().clone();
                cb(selection);
            }
        }));

        for (index, part) in recording.work.parts.iter().enumerate() {
            let check = gtk::CheckButton::new();
            check.set_active(this.selection.borrow().contains(&index));

            check.connect_toggled(clone!(@strong this => move |check| {
                let mut selection = this.selection.borrow_mut();
                if check.get_active() {
                    selection.push(index);
                } else {
                    if let Some(pos) = selection.iter().position(|part| *part == index) {
                        selection.remove(pos);
                    }
                }
            }));

            let row = libhandy::ActionRow::new();
            row.add_prefix(&check);
            row.set_activatable_widget(Some(&check));
            row.set_title(Some(&part.title));
            row.show_all();

            parts_list.add(&row);
        }

        this
    }

    /// Set the closure to be called when the user has edited the track.
    pub fn set_selected_cb<F: Fn(Vec<usize>) + 'static>(&self, cb: F) {
        self.selected_cb.replace(Some(Box::new(cb)));
    }
}

impl NavigatorScreen for TrackEditor {
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
