use crate::navigator::{NavigationHandle, Screen};
use crate::widgets::Widget;
use adw::builders::ActionRowBuilder;
use adw::prelude::*;
use glib::clone;
use gtk::builders::ListBoxBuilder;
use gtk_macros::get_widget;
use musicus_backend::db::Recording;
use std::cell::RefCell;
use std::rc::Rc;

/// A screen for editing a single track.
pub struct TrackEditor {
    handle: NavigationHandle<Vec<usize>>,
    widget: gtk::Box,
    selection: RefCell<Vec<usize>>,
}

impl Screen<(Recording, Vec<usize>), Vec<usize>> for TrackEditor {
    /// Create a new track editor.
    fn new(
        (recording, selection): (Recording, Vec<usize>),
        handle: NavigationHandle<Vec<usize>>,
    ) -> Rc<Self> {
        // Create UI

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/track_editor.ui");

        get_widget!(builder, gtk::Box, widget);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::Button, select_button);
        get_widget!(builder, adw::Clamp, clamp);

        let parts_list = ListBoxBuilder::new()
            .selection_mode(gtk::SelectionMode::None)
            .css_classes(vec![String::from("boxed-list")])
            .build();

        clamp.set_child(Some(&parts_list));

        let this = Rc::new(Self {
            handle,
            widget,
            selection: RefCell::new(selection),
        });

        // Connect signals and callbacks

        back_button.connect_clicked(clone!(@weak this =>  move |_| {
            this.handle.pop(None);
        }));

        select_button.connect_clicked(clone!(@weak this =>  move |_| {
            let selection = this.selection.borrow().clone();
            this.handle.pop(Some(selection));
        }));

        for (index, part) in recording.work.parts.iter().enumerate() {
            let check = gtk::CheckButton::new();
            check.set_active(this.selection.borrow().contains(&index));

            check.connect_toggled(clone!(@weak this =>  move |check| {
                let mut selection = this.selection.borrow_mut();
                if check.is_active() {
                    selection.push(index);
                } else if let Some(pos) = selection.iter().position(|part| *part == index) {
                    selection.remove(pos);
                }
            }));

            let row = ActionRowBuilder::new()
                .focusable(false)
                .title(&part.title)
                .activatable_widget(&check)
                .build();

            row.add_prefix(&check);
            parts_list.append(&row);
        }

        this
    }
}

impl Widget for TrackEditor {
    fn get_widget(&self) -> gtk::Widget {
        self.widget.clone().upcast()
    }
}
