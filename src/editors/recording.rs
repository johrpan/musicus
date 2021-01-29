use super::performance::PerformanceEditor;
use crate::backend::Backend;
use crate::database::*;
use crate::selectors::{PersonSelector, WorkSelector};
use crate::widgets::{List, Navigator, NavigatorScreen};
use anyhow::Result;
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// A widget for creating or editing a recording.
pub struct RecordingEditor {
    pub widget: gtk::Stack,
    backend: Rc<Backend>,
    save_button: gtk::Button,
    info_bar: gtk::InfoBar,
    work_row: libadwaita::ActionRow,
    comment_entry: gtk::Entry,
    upload_switch: gtk::Switch,
    performance_list: Rc<List>,
    id: String,
    work: RefCell<Option<Work>>,
    performances: RefCell<Vec<Performance>>,
    selected_cb: RefCell<Option<Box<dyn Fn(Recording) -> ()>>>,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl RecordingEditor {
    /// Create a new recording editor widget and optionally initialize it.
    pub fn new(backend: Rc<Backend>, recording: Option<Recording>) -> Rc<Self> {
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
            widget,
            backend,
            save_button,
            info_bar,
            work_row,
            comment_entry,
            upload_switch,
            performance_list,
            id,
            work: RefCell::new(work),
            performances: RefCell::new(performances),
            selected_cb: RefCell::new(None),
            navigator: RefCell::new(None),
        });

        // Connect signals and callbacks

        back_button.connect_clicked(clone!(@strong this => move |_| {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                navigator.clone().pop();
            }
        }));

        this.save_button
            .connect_clicked(clone!(@strong this => move |_| {
                let context = glib::MainContext::default();
                let clone = this.clone();
                context.spawn_local(async move {
                    clone.widget.set_visible_child_name("loading");
                    match clone.clone().save().await {
                        Ok(_) => {
                            let navigator = clone.navigator.borrow().clone();
                            if let Some(navigator) = navigator {
                                navigator.clone().pop();
                            }
                        }
                        Err(_) => {
                            clone.info_bar.set_revealed(true);
                            clone.widget.set_visible_child_name("content");
                        }
                    }

                });
            }));

        work_button.connect_clicked(clone!(@strong this => move |_| {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                let person_selector = PersonSelector::new(this.backend.clone());

                person_selector.set_selected_cb(clone!(@strong this, @strong navigator => move |person| {
                    let work_selector = WorkSelector::new(this.backend.clone(), person.clone());
                    
                    work_selector.set_selected_cb(clone!(@strong this, @strong navigator => move |work| {
                        this.work_selected(&work);
                        this.work.replace(Some(work.clone()));

                        navigator.clone().pop();
                        navigator.clone().pop();
                    }));

                    navigator.clone().push(work_selector);
                }));

                navigator.push(person_selector);
            }
        }));

        this.performance_list.set_make_widget_cb(clone!(@strong this => move |index| {
            let performance = &this.performances.borrow()[index];

            let delete_button = gtk::Button::from_icon_name(Some("user-trash-symbolic"));
            delete_button.set_valign(gtk::Align::Center);

            delete_button.connect_clicked(clone!(@strong this => move |_| {
                    let length = {
                        let mut performances = this.performances.borrow_mut();
                        performances.remove(index);
                        performances.len()
                    };

                    this.performance_list.update(length);
            }));

            let edit_button = gtk::Button::from_icon_name(Some("document-edit-symbolic"));
            edit_button.set_valign(gtk::Align::Center);

            edit_button.connect_clicked(clone!(@strong this => move |_| {
                    let navigator = this.navigator.borrow().clone();
                    if let Some(navigator) = navigator {
                        let performance = &this.performances.borrow()[index];

                        let editor = PerformanceEditor::new(
                            this.backend.clone(),
                            Some(performance.clone()),
                        );

                        editor.set_selected_cb(clone!(@strong this, @strong navigator => move |performance| {
                            let length = {
                                let mut performances = this.performances.borrow_mut();
                                performances[index] = performance;
                                performances.len()
                            };

                            this.performance_list.update(length);

                            navigator.clone().pop();
                        }));

                        navigator.push(editor);
                    }
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
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                let editor = PerformanceEditor::new(this.backend.clone(), None);

                editor.set_selected_cb(clone!(@strong this, @strong navigator => move |performance| {
                    let length = {
                        let mut performances = this.performances.borrow_mut();
                        performances.push(performance);
                        performances.len()
                    };

                    this.performance_list.update(length);

                    navigator.clone().pop();
                }));

                navigator.push(editor);
            }
        }));

        // Initialize

        if let Some(work) = &*this.work.borrow() {
            this.work_selected(work);
        }

        let length = this.performances.borrow().len();
        this.performance_list.update(length);

        this
    }

    /// Set the closure to be called if the recording was created.
    pub fn set_selected_cb<F: Fn(Recording) -> () + 'static>(&self, cb: F) {
        self.selected_cb.replace(Some(Box::new(cb)));
    }

    /// Update the UI according to work.    
    fn work_selected(&self, work: &Work) {
        self.work_row.set_title(Some(&gettext("Work")));
        self.work_row.set_subtitle(Some(&work.get_title()));
        self.save_button.set_sensitive(true);
    }

    /// Save the recording and possibly upload it to the server.
    async fn save(self: Rc<Self>) -> Result<()> {
        let recording = Recording {
            id: self.id.clone(),
            work: self
                .work
                .borrow()
                .clone()
                .expect("Tried to create recording without work!"),
            comment: self.comment_entry.get_text().unwrap().to_string(),
            performances: self.performances.borrow().clone(),
        };

        let upload = self.upload_switch.get_active();
        if upload {
            self.backend.post_recording(&recording).await?;
        }

        self.backend
            .db()
            .update_recording(recording.clone().into())
            .await
            .unwrap();

        self.backend.library_changed();

        if let Some(cb) = &*self.selected_cb.borrow() {
            cb(recording.clone());
        }

        let navigator = self.navigator.borrow().clone();
        if let Some(navigator) = navigator {
            navigator.clone().pop();
        }

        Ok(())
    }
}

impl NavigatorScreen for RecordingEditor {
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
