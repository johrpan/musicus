use super::disc_source::DiscSource;
use super::track_set_editor::{TrackSetData, TrackSetEditor};
use crate::backend::Backend;
use crate::widgets::{Navigator, NavigatorScreen};
use crate::widgets::new_list::List;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use libhandy::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// A dialog for editing metadata while importing music into the music library.
pub struct MediumEditor {
    backend: Rc<Backend>,
    source: Rc<DiscSource>,
    widget: gtk::Box,
    track_set_list: List,
    track_sets: RefCell<Vec<TrackSetData>>,
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
            track_set_list: list,
            track_sets: RefCell::new(Vec::new()),
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

                editor.set_done_cb(clone!(@strong this => move |track_set| {
                    let length = {
                        let mut track_sets = this.track_sets.borrow_mut();
                        track_sets.push(track_set);
                        track_sets.len()
                    };

                    this.track_set_list.update(length);
                }));

                navigator.push(editor);
            }
        }));

        this.track_set_list.set_make_widget(clone!(@strong this => move |index| {
            let track_set = &this.track_sets.borrow()[index];

            let title = track_set.recording.work.get_title();
            let subtitle = track_set.recording.get_performers();

            let edit_image = gtk::Image::from_icon_name(Some("document-edit-symbolic"), gtk::IconSize::Button);
            let edit_button = gtk::Button::new();
            edit_button.set_relief(gtk::ReliefStyle::None);
            edit_button.set_valign(gtk::Align::Center);
            edit_button.add(&edit_image);

            let row = libhandy::ActionRow::new();
            row.set_activatable(true);
            row.set_title(Some(&title));
            row.set_subtitle(Some(&subtitle));
            row.add(&edit_button);
            row.set_activatable_widget(Some(&edit_button));
            row.show_all();

            edit_button.connect_clicked(clone!(@strong this => move |_| {

            }));

            row.upcast()
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
