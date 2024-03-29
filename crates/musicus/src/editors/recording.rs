use super::performance::PerformanceEditor;
use crate::navigator::{NavigationHandle, Screen};
use crate::selectors::WorkSelector;
use crate::widgets::{List, Widget};

use adw::prelude::*;
use anyhow::Result;
use gettextrs::gettext;
use glib::clone;
use gtk_macros::get_widget;
use musicus_backend::db::{self, generate_id, Performance, Recording, Work};
use std::cell::RefCell;
use std::rc::Rc;

/// A widget for creating or editing a recording.
pub struct RecordingEditor {
    handle: NavigationHandle<Recording>,
    widget: gtk::Stack,
    save_button: gtk::Button,
    info_bar: gtk::InfoBar,
    work_row: adw::ActionRow,
    comment_row: adw::EntryRow,
    performance_list: Rc<List>,
    id: String,
    work: RefCell<Option<Work>>,
    performances: RefCell<Vec<Performance>>,
}

impl Screen<Option<Recording>, Recording> for RecordingEditor {
    /// Create a new recording editor widget and optionally initialize it.
    fn new(recording: Option<Recording>, handle: NavigationHandle<Recording>) -> Rc<Self> {
        // Create UI

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/recording_editor.ui");

        get_widget!(builder, gtk::Stack, widget);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::Button, save_button);
        get_widget!(builder, gtk::InfoBar, info_bar);
        get_widget!(builder, adw::ActionRow, work_row);
        get_widget!(builder, gtk::Button, work_button);
        get_widget!(builder, adw::EntryRow, comment_row);
        get_widget!(builder, gtk::Frame, performance_frame);
        get_widget!(builder, gtk::Button, add_performer_button);

        let performance_list = List::new();
        performance_frame.set_child(Some(&performance_list.widget));

        let (id, work, performances) = match recording {
            Some(recording) => {
                comment_row.set_text(&recording.comment);
                (recording.id, Some(recording.work), recording.performances)
            }
            None => (generate_id(), None, Vec::new()),
        };

        let this = Rc::new(RecordingEditor {
            handle,
            widget,
            save_button,
            info_bar,
            work_row,
            comment_row,
            performance_list,
            id,
            work: RefCell::new(work),
            performances: RefCell::new(performances),
        });

        // Connect signals and callbacks

        back_button.connect_clicked(clone!(@weak this =>  move |_| {
            this.handle.pop(None);
        }));

        this.save_button
            .connect_clicked(clone!(@weak this =>  move |_| {
                match this.save() {
                    Ok(recording) => {
                        this.handle.pop(Some(recording));
                    }
                    Err(_) => {
                        this.info_bar.set_revealed(true);
                        this.widget.set_visible_child_name("content");
                    }
                }
            }));

        work_button.connect_clicked(clone!(@weak this =>  move |_| {
            spawn!(@clone this, async move {
                if let Some(work) = push!(this.handle, WorkSelector).await {
                    this.work_selected(&work);
                    this.work.replace(Some(work));
                }
            });
        }));

        this.performance_list.set_make_widget_cb(clone!(@weak this => @default-panic, move |index| {
            let performance = &this.performances.borrow()[index];

            let delete_button = gtk::Button::from_icon_name("user-trash-symbolic");
            delete_button.set_valign(gtk::Align::Center);

            delete_button.connect_clicked(clone!(@weak this =>  move |_| {
                let length = {
                    let mut performances = this.performances.borrow_mut();
                    performances.remove(index);
                    performances.len()
                };

                this.performance_list.update(length);
            }));

            let edit_button = gtk::Button::from_icon_name("document-edit-symbolic");
            edit_button.set_valign(gtk::Align::Center);

            edit_button.connect_clicked(clone!(@weak this =>  move |_| {
                spawn!(@clone this, async move {
                    let performance = this.performances.borrow()[index].clone();
                    if let Some(performance) = push!(this.handle, PerformanceEditor, Some(performance)).await {
                        let length = {
                            let mut performances = this.performances.borrow_mut();
                            performances[index] = performance;
                            performances.len()
                        };

                        this.performance_list.update(length);
                    }
                });
            }));

            let row = adw::ActionRow::builder()
                .focusable(false)
                .activatable_widget(&edit_button)
                .title(performance.get_title())
                .build();

            row.add_suffix(&delete_button);
            row.add_suffix(&edit_button);

            row.upcast()
        }));

        add_performer_button.connect_clicked(clone!(@strong this => move |_| {
            spawn!(@clone this, async move {
                if let Some(performance) = push!(this.handle, PerformanceEditor, None).await {
                    let length = {
                        let mut performances = this.performances.borrow_mut();
                        performances.push(performance);
                        performances.len()
                    };

                    this.performance_list.update(length);
                }
            });
        }));

        // Initialize

        if let Some(work) = &*this.work.borrow() {
            this.work_selected(work);
        }

        let length = this.performances.borrow().len();
        this.performance_list.update(length);

        this
    }
}

impl RecordingEditor {
    /// Update the UI according to work.
    fn work_selected(&self, work: &Work) {
        self.work_row.set_title(&gettext("Work"));
        self.work_row.set_subtitle(&work.get_title());
        self.save_button.set_sensitive(true);
    }

    /// Save the recording.
    fn save(self: &Rc<Self>) -> Result<Recording> {
        let recording = Recording::new(
            self.id.clone(),
            self.work
                .borrow()
                .clone()
                .expect("Tried to create recording without work!"),
            self.comment_row.text().to_string(),
            self.performances.borrow().clone(),
        );

        db::update_recording(
            &mut self.handle.backend.db().lock().unwrap(),
            recording.clone(),
        )?;

        self.handle.backend.library_changed();

        Ok(recording)
    }
}

impl Widget for RecordingEditor {
    fn get_widget(&self) -> gtk::Widget {
        self.widget.clone().upcast()
    }
}
