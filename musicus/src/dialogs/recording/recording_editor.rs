use super::performance_editor::*;
use crate::backend::*;
use crate::database::*;
use crate::dialogs::*;
use crate::widgets::*;
use anyhow::Result;
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use std::cell::RefCell;
use std::rc::Rc;

/// A widget for creating or editing a recording.
// TODO: Disable buttons if no performance is selected.
pub struct RecordingEditor {
    pub widget: gtk::Stack,
    backend: Rc<Backend>,
    parent: gtk::Window,
    save_button: gtk::Button,
    info_bar: gtk::InfoBar,
    work_label: gtk::Label,
    comment_entry: gtk::Entry,
    upload_switch: gtk::Switch,
    performance_list: Rc<List<Performance>>,
    id: String,
    work: RefCell<Option<Work>>,
    performances: RefCell<Vec<Performance>>,
    selected_cb: RefCell<Option<Box<dyn Fn(Recording) -> ()>>>,
    back_cb: RefCell<Option<Box<dyn Fn() -> ()>>>,
}

impl RecordingEditor {
    /// Create a new recording editor widget and optionally initialize it. The parent window is
    /// used as the parent for newly created dialogs.
    pub fn new<W: IsA<gtk::Window>>(
        backend: Rc<Backend>,
        parent: &W,
        recording: Option<Recording>,
    ) -> Rc<Self> {
        // Create UI

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/recording_editor.ui");

        get_widget!(builder, gtk::Stack, widget);
        get_widget!(builder, gtk::Button, cancel_button);
        get_widget!(builder, gtk::Button, save_button);
        get_widget!(builder, gtk::InfoBar, info_bar);
        get_widget!(builder, gtk::Button, work_button);
        get_widget!(builder, gtk::Label, work_label);
        get_widget!(builder, gtk::Entry, comment_entry);
        get_widget!(builder, gtk::Switch, upload_switch);
        get_widget!(builder, gtk::ScrolledWindow, scroll);
        get_widget!(builder, gtk::Button, add_performer_button);
        get_widget!(builder, gtk::Button, edit_performer_button);
        get_widget!(builder, gtk::Button, remove_performer_button);

        let performance_list = List::new(&gettext("No performers added."));
        scroll.add(&performance_list.widget);

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
            parent: parent.clone().upcast(),
            save_button,
            info_bar,
            work_label,
            comment_entry,
            upload_switch,
            performance_list,
            id,
            work: RefCell::new(work),
            performances: RefCell::new(performances),
            selected_cb: RefCell::new(None),
            back_cb: RefCell::new(None),
        });

        // Connect signals and callbacks

        cancel_button.connect_clicked(clone!(@strong this => move |_| {
            if let Some(cb) = &*this.back_cb.borrow() {
                cb();
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
                            // We already called the callback.
                        }
                        Err(_) => {
                            clone.info_bar.set_revealed(true);
                            clone.widget.set_visible_child_name("content");
                        }
                    }

                });
            }));

        work_button.connect_clicked(clone!(@strong this => move |_| {
            let dialog = WorkDialog::new(this.backend.clone(), &this.parent);

            dialog.set_selected_cb(clone!(@strong this => move |work| {
                this.work_selected(&work);
                this.work.replace(Some(work));
            }));

            dialog.show();
        }));

        this.performance_list.set_make_widget(|performance| {
            let label = gtk::Label::new(Some(&performance.get_title()));
            label.set_ellipsize(pango::EllipsizeMode::End);
            label.set_halign(gtk::Align::Start);
            label.set_margin_start(6);
            label.set_margin_end(6);
            label.set_margin_top(6);
            label.set_margin_bottom(6);
            label.upcast()
        });

        add_performer_button.connect_clicked(clone!(@strong this => move |_| {
            let editor = PerformanceEditor::new(this.backend.clone(), &this.parent, None);

            editor.set_selected_cb(clone!(@strong this => move |performance| {
                let mut performances = this.performances.borrow_mut();

                let index = match this.performance_list.get_selected_index() {
                    Some(index) => index + 1,
                    None => performances.len(),
                };

                performances.insert(index, performance);
                this.performance_list.show_items(performances.clone());
                this.performance_list.select_index(index);
            }));

            editor.show();
        }));

        edit_performer_button.connect_clicked(clone!(@strong this => move |_| {
            if let Some(index) = this.performance_list.get_selected_index() {
                let performance = &this.performances.borrow()[index];

                let editor = PerformanceEditor::new(
                    this.backend.clone(),
                    &this.parent,
                    Some(performance.clone()),
                );

                editor.set_selected_cb(clone!(@strong this => move |performance| {
                    let mut performances = this.performances.borrow_mut();
                    performances[index] = performance;
                    this.performance_list.show_items(performances.clone());
                    this.performance_list.select_index(index);
                }));

                editor.show();
            }
        }));

        remove_performer_button.connect_clicked(clone!(@strong this => move |_| {
            if let Some(index) = this.performance_list.get_selected_index() {
                let mut performances = this.performances.borrow_mut();
                performances.remove(index);
                this.performance_list.show_items(performances.clone());
                this.performance_list.select_index(index);
            }
        }));

        // Initialize

        if let Some(work) = &*this.work.borrow() {
            this.work_selected(work);
        }

        this.performance_list
            .show_items(this.performances.borrow().clone());

        this
    }

    /// Set the closure to be called if the editor is canceled.
    pub fn set_back_cb<F: Fn() -> () + 'static>(&self, cb: F) {
        self.back_cb.replace(Some(Box::new(cb)));
    }

    /// Set the closure to be called if the recording was created.
    pub fn set_selected_cb<F: Fn(Recording) -> () + 'static>(&self, cb: F) {
        self.selected_cb.replace(Some(Box::new(cb)));
    }

    /// Update the UI according to work.    
    fn work_selected(&self, work: &Work) {
        self.work_label
            .set_text(&format!("{}: {}", work.composer.name_fl(), work.title));
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
            comment: self.comment_entry.get_text().to_string(),
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

        Ok(())
    }
}
