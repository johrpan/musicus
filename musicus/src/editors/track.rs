use crate::database::*;
use crate::widgets::{Navigator, NavigatorScreen};
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use std::cell::RefCell;
use std::convert::TryInto;
use std::rc::Rc;

/// A screen for editing a single track.
// TODO: Refactor.
pub struct TrackEditor {
    widget: gtk::Box,
    ready_cb: RefCell<Option<Box<dyn Fn(Track) -> ()>>>,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl TrackEditor {
    /// Create a new track editor.
    pub fn new(track: Track, work: Work) -> Rc<Self> {
        // Create UI

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/track_editor.ui");

        get_widget!(builder, gtk::Box, widget);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::Button, save_button);
        get_widget!(builder, gtk::ListBox, list);

        let this = Rc::new(Self {
            widget,
            ready_cb: RefCell::new(None),
            navigator: RefCell::new(None),
        });

        back_button.connect_clicked(clone!(@strong this => move |_| {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                navigator.pop();
            }
        }));

        let work = Rc::new(work);
        let work_parts = Rc::new(RefCell::new(track.work_parts));
        let file_name = track.file_name;

        save_button.connect_clicked(clone!(@strong this, @strong work_parts => move |_| {
            let mut work_parts = work_parts.borrow_mut();
            work_parts.sort();

            if let Some(cb) = &*this.ready_cb.borrow() {
                cb(Track {
                    work_parts: work_parts.clone(),
                    file_name: file_name.clone(),
                });
            }

            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                navigator.pop();
            }
        }));

        for (index, part) in work.parts.iter().enumerate() {
            let check = gtk::CheckButton::new();
            check.set_active(work_parts.borrow().contains(&index));
            check.connect_toggled(clone!(@strong check, @strong work_parts => move |_| {
                if check.get_active() {
                    let mut work_parts = work_parts.borrow_mut();
                    work_parts.push(index);
                } else {
                    let mut work_parts = work_parts.borrow_mut();
                    if let Some(pos) = work_parts.iter().position(|part| *part == index) {
                        work_parts.remove(pos);
                    }
                }
            }));

            let label = gtk::Label::new(Some(&part.title));
            label.set_halign(gtk::Align::Start);
            label.set_ellipsize(pango::EllipsizeMode::End);

            let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            hbox.set_border_width(6);
            hbox.add(&check);
            hbox.add(&label);

            let row = gtk::ListBoxRow::new();
            row.add(&hbox);
            row.show_all();

            list.add(&row);
            list.connect_row_activated(
                clone!(@strong row, @strong check => move |_, activated_row| {
                    if *activated_row == row {
                        check.activate();
                    }
                }),
            );
        }

        let mut section_count = 0;
        for section in &work.sections {
            let attributes = pango::AttrList::new();
            attributes.insert(pango::Attribute::new_weight(pango::Weight::Bold).unwrap());

            let label = gtk::Label::new(Some(&section.title));
            label.set_halign(gtk::Align::Start);
            label.set_ellipsize(pango::EllipsizeMode::End);
            label.set_attributes(Some(&attributes));
            let wrap = gtk::Box::new(gtk::Orientation::Vertical, 0);
            wrap.set_border_width(6);
            wrap.add(&label);

            let row = gtk::ListBoxRow::new();
            row.set_activatable(false);
            row.add(&wrap);
            row.show_all();

            list.insert(
                &row,
                (section.before_index + section_count).try_into().unwrap(),
            );
            section_count += 1;
        }

        this
    }

    /// Set the closure to be called when the track was edited.
    pub fn set_ready_cb<F: Fn(Track) -> () + 'static>(&self, cb: F) {
        self.ready_cb.replace(Some(Box::new(cb)));
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
