use crate::navigator::{NavigationHandle, Screen};
use crate::widgets::Widget;
use adw::prelude::*;
use glib::clone;
use gtk_macros::get_widget;
use musicus_backend::import::ImportSession;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

/// A screen for selecting tracks from a source.
pub struct TrackSelector {
    handle: NavigationHandle<Vec<usize>>,
    session: Arc<ImportSession>,
    widget: gtk::Box,
    select_button: gtk::Button,
    selection: RefCell<Vec<usize>>,
}

impl Screen<Arc<ImportSession>, Vec<usize>> for TrackSelector {
    /// Create a new track selector.
    fn new(session: Arc<ImportSession>, handle: NavigationHandle<Vec<usize>>) -> Rc<Self> {
        // Create UI

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/track_selector.ui");

        get_widget!(builder, gtk::Box, widget);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::Button, select_button);
        get_widget!(builder, gtk::Frame, tracks_frame);

        let track_list = gtk::ListBox::new();
        track_list.set_selection_mode(gtk::SelectionMode::None);
        track_list.set_vexpand(false);
        track_list.show();
        tracks_frame.set_child(Some(&track_list));

        let this = Rc::new(Self {
            handle,
            session,
            widget,
            select_button,
            selection: RefCell::new(Vec::new()),
        });

        // Connect signals and callbacks

        back_button.connect_clicked(clone!(@weak this =>  move |_| {
            this.handle.pop(None);
        }));

        this.select_button
            .connect_clicked(clone!(@weak this =>  move |_| {
                let selection = this.selection.borrow().clone();
                this.handle.pop(Some(selection));
            }));

        let tracks = this.session.tracks();

        for (index, track) in tracks.iter().enumerate() {
            let check = gtk::CheckButton::new();

            check.connect_toggled(clone!(@weak this =>  move |check| {
                let mut selection = this.selection.borrow_mut();
                if check.is_active() {
                    selection.push(index);
                } else if let Some(pos) = selection.iter().position(|part| *part == index) {
                    selection.remove(pos);
                }

                if selection.is_empty() {
                    this.select_button.set_sensitive(false);
                } else {
                    this.select_button.set_sensitive(true);
                }
            }));

            let row = adw::ActionRowBuilder::new()
                .focusable(false)
                .title(&track.name)
                .activatable_widget(&check)
                .build();

            row.add_prefix(&check);

            track_list.append(&row);
        }

        this
    }
}

impl Widget for TrackSelector {
    fn get_widget(&self) -> gtk::Widget {
        self.widget.clone().upcast()
    }
}
