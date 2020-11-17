use crate::database::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use std::cell::RefCell;
use std::convert::TryInto;
use std::rc::Rc;

pub struct TrackEditor {
    window: libhandy::Window,
}

impl TrackEditor {
    pub fn new<W, F>(parent: &W, track: Track, work: Work, callback: F) -> Self
    where
        W: IsA<gtk::Window>,
        F: Fn(Track) -> () + 'static,
    {
        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/track_editor.ui");

        get_widget!(builder, libhandy::Window, window);
        get_widget!(builder, gtk::Button, cancel_button);
        get_widget!(builder, gtk::Button, save_button);
        get_widget!(builder, gtk::ListBox, list);

        window.set_transient_for(Some(parent));

        cancel_button.connect_clicked(clone!(@strong window => move |_| {
            window.close();
        }));

        let work = Rc::new(work);
        let work_parts = Rc::new(RefCell::new(track.work_parts));
        let file_name = track.file_name;

        save_button.connect_clicked(clone!(@strong work_parts, @strong window => move |_| {
            let mut work_parts = work_parts.borrow_mut();
            work_parts.sort();

            callback(Track {
                work_parts: work_parts.clone(),
                file_name: file_name.clone(),
            });

            window.close();
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

        Self { window }
    }

    pub fn show(&self) {
        self.window.show();
    }
}
