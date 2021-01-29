use super::source::Source;
use crate::widgets::{Navigator, NavigatorScreen};
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// A screen for selecting tracks from a source.
pub struct TrackSelector {
    source: Rc<Box<dyn Source>>,
    widget: gtk::Box,
    select_button: gtk::Button,
    selection: RefCell<Vec<usize>>,
    selected_cb: RefCell<Option<Box<dyn Fn(Vec<usize>)>>>,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl TrackSelector {
    /// Create a new track selector.
    pub fn new(source: Rc<Box<dyn Source>>) -> Rc<Self> {
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
            source,
            widget,
            select_button,
            selection: RefCell::new(Vec::new()),
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

        this.select_button.connect_clicked(clone!(@strong this => move |_| {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                navigator.pop();
            }

            if let Some(cb) = &*this.selected_cb.borrow() {
                let selection = this.selection.borrow().clone();
                cb(selection);
            }
        }));

        let tracks = this.source.tracks().unwrap();

        for (index, track) in tracks.iter().enumerate() {
            let check = gtk::CheckButton::new();

            check.connect_toggled(clone!(@strong this => move |check| {
                let mut selection = this.selection.borrow_mut();
                if check.get_active() {
                    selection.push(index);
                } else {
                    if let Some(pos) = selection.iter().position(|part| *part == index) {
                        selection.remove(pos);
                    }
                }

                if selection.is_empty() {
                    this.select_button.set_sensitive(false);
                } else {
                    this.select_button.set_sensitive(true);
                }
            }));

            let title = format!("Track {}", track.number);

            let row = libadwaita::ActionRow::new();
            row.add_prefix(&check);
            row.set_activatable_widget(Some(&check));
            row.set_activatable(true);
            row.set_title(Some(&title));

            track_list.append(&row);
        }

        this
    }

    /// Set the closure to be called when the user has selected tracks. The
    /// closure will be called with the indices of the selected tracks as its
    /// argument.
    pub fn set_selected_cb<F: Fn(Vec<usize>) + 'static>(&self, cb: F) {
        self.selected_cb.replace(Some(Box::new(cb)));
    }
}

impl NavigatorScreen for TrackSelector {
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
