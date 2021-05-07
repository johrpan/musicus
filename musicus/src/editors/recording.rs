use super::performance::PerformanceEditor;
use crate::navigator::{NavigationHandle, Screen};
use crate::selectors::WorkSelector;
use crate::widgets::{List, Widget};
use anyhow::Result;
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use libadwaita::prelude::*;
use musicus_backend::db::{generate_id, Performance, Recording, Work};
use std::cell::RefCell;
use std::rc::Rc;

/// A widget for creating or editing a recording.
pub struct RecordingEditor {
    handle: NavigationHandle<Recording>,
    widget: gtk::Stack,
    save_button: gtk::Button,
    info_bar: gtk::InfoBar,
    work_row: libadwaita::ActionRow,
    comment_entry: gtk::Entry,
    upload_switch: gtk::Switch,
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
        get_widget!(builder, libadwaita::ActionRow, work_row);
        get_widget!(builder, gtk::Button, work_button);
        get_widget!(builder, gtk::Entry, comment_entry);
        get_widget!(builder, gtk::Switch, upload_switch);
        get_widget!(builder, gtk::Frame, performance_frame);
        get_widget!(builder, gtk::Button, add_performer_button);

        upload_switch.set_active(handle.backend.use_server());

        let performance_list = List::new();
        performance_frame.set_child(Some(&performance_list.widget));

        let (id, work, performances) = match recording {
            Some(recording) => {
                comment_entry.set_text(&recording.comment);
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
            comment_entry,
            upload_switch,
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
                spawn!(@clone this, async move {
                    this.widget.set_visible_child_name("loading");
                    match this.save().await {
                        Ok(recording) => {
                            this.handle.pop(Some(recording));
                        }
                        Err(_) => {
                            this.info_bar.set_revealed(true);
                            this.widget.set_visible_child_name("content");
                        }
                    }
                });
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

            let delete_button = gtk::Button::from_icon_name(Some("user-trash-symbolic"));
            delete_button.set_valign(gtk::Align::Center);

            delete_button.connect_clicked(clone!(@weak this =>  move |_| {
                let length = {
                    let mut performances = this.performances.borrow_mut();
                    performances.remove(index);
                    performances.len()
                };

                this.performance_list.update(length);
            }));

            let edit_button = gtk::Button::from_icon_name(Some("document-edit-symbolic"));
            edit_button.set_valign(gtk::Align::Center);

            edit_button.connect_clicked(clone!(@weak this =>  move |_| {
                spawn!(@clone this, async move {
                    let performance = &this.performances.borrow()[index];
                    if let Some(performance) = push!(this.handle, PerformanceEditor, Some(performance.to_owned())).await {
                        let length = {
                            let mut performances = this.performances.borrow_mut();
                            performances[index] = performance;
                            performances.len()
                        };

                        this.performance_list.update(length);
                    }
                });
            }));

            let row = libadwaita::ActionRow::new();
            row.set_activatable(true);
            row.set_title(Some(&performance.get_title()));
            row.add_suffix(&delete_button);
            row.add_suffix(&edit_button);
            row.set_activatable_widget(Some(&edit_button));

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
        self.work_row.set_title(Some(&gettext("Work")));
        self.work_row.set_subtitle(Some(&work.get_title()));
        self.save_button.set_sensitive(true);
    }

    /// Save the recording and possibly upload it to the server.
    async fn save(self: &Rc<Self>) -> Result<Recording> {
        let recording = Recording {
            id: self.id.clone(),
            work: self
                .work
                .borrow()
                .clone()
                .expect("Tried to create recording without work!"),
            comment: self.comment_entry.text().to_string(),
            performances: self.performances.borrow().clone(),
        };

        let upload = self.upload_switch.state();
        if upload {
            self.handle.backend.cl().post_recording(&recording).await?;
        }

        self.handle
            .backend
            .db()
            .update_recording(recording.clone())
            .await
            .unwrap();

        self.handle.backend.library_changed();

        Ok(recording)
    }
}

impl Widget for RecordingEditor {
    fn get_widget(&self) -> gtk::Widget {
        self.widget.clone().upcast()
    }
}
