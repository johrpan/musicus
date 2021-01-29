use super::*;
use crate::backend::*;
use crate::database::*;
use crate::editors::EnsembleEditor;
use crate::widgets::{List, Navigator, NavigatorScreen, NavigatorWindow};
use gio::prelude::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

pub struct EnsembleScreen {
    backend: Rc<Backend>,
    ensemble: Ensemble,
    widget: gtk::Box,
    search_entry: gtk::SearchEntry,
    stack: gtk::Stack,
    recording_list: Rc<List>,
    recordings: RefCell<Vec<Recording>>,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl EnsembleScreen {
    pub fn new(backend: Rc<Backend>, ensemble: Ensemble) -> Rc<Self> {
        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/ensemble_screen.ui");

        get_widget!(builder, gtk::Box, widget);
        get_widget!(builder, gtk::Label, title_label);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::SearchEntry, search_entry);
        get_widget!(builder, gtk::Stack, stack);
        get_widget!(builder, gtk::Frame, recording_frame);

        title_label.set_label(&ensemble.name);

        let edit_action = gio::SimpleAction::new("edit", None);
        let delete_action = gio::SimpleAction::new("delete", None);

        let actions = gio::SimpleActionGroup::new();
        actions.add_action(&edit_action);
        actions.add_action(&delete_action);

        widget.insert_action_group("widget", Some(&actions));

        let recording_list = List::new();
        recording_frame.set_child(Some(&recording_list.widget));

        let this = Rc::new(Self {
            backend,
            ensemble,
            widget,
            search_entry,
            stack,
            recording_list,
            recordings: RefCell::new(Vec::new()),
            navigator: RefCell::new(None),
        });

        this.search_entry.connect_search_changed(clone!(@strong this => move |_| {
            this.recording_list.invalidate_filter();
        }));

        back_button.connect_clicked(clone!(@strong this => move |_| {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                navigator.pop();
            }
        }));

        this.recording_list.set_make_widget_cb(clone!(@strong this => move |index| {
            let recording = &this.recordings.borrow()[index];

            let row = libadwaita::ActionRow::new();
            row.set_activatable(true);
            row.set_title(Some(&recording.work.get_title()));
            row.set_subtitle(Some(&recording.get_performers()));

            let recording = recording.to_owned();
            row.connect_activated(clone!(@strong this => move |_| {
                let navigator = this.navigator.borrow().clone();
                if let Some(navigator) = navigator {
                    navigator.push(RecordingScreen::new(this.backend.clone(), recording.clone()));
                }
            }));

            row.upcast()
        }));

        this.recording_list.set_filter_cb(clone!(@strong this => move |index| {
            let recording = &this.recordings.borrow()[index];
            let search = this.search_entry.get_text().unwrap().to_string().to_lowercase();
            let text = recording.work.get_title() + &recording.get_performers();
            search.is_empty() || text.to_lowercase().contains(&search)
        }));

        edit_action.connect_activate(clone!(@strong this => move |_, _| {
            let editor = EnsembleEditor::new(this.backend.clone(), Some(this.ensemble.clone()));
            let window = NavigatorWindow::new(editor);
            window.show();
        }));

        delete_action.connect_activate(clone!(@strong this => move |_, _| {
            let context = glib::MainContext::default();
            let clone = this.clone();
            context.spawn_local(async move {
                clone.backend.db().delete_ensemble(&clone.ensemble.id).await.unwrap();
                clone.backend.library_changed();
            });
        }));

        let context = glib::MainContext::default();
        let clone = this.clone();
        context.spawn_local(async move {
            let recordings = clone
                .backend
                .db()
                .get_recordings_for_ensemble(&clone.ensemble.id)
                .await
                .unwrap();

            if recordings.is_empty() {
                clone.stack.set_visible_child_name("nothing");
            } else {
                let length = recordings.len();
                clone.recordings.replace(recordings);
                clone.recording_list.update(length);
                clone.stack.set_visible_child_name("content");
            }
        });

        this
    }
}

impl NavigatorScreen for EnsembleScreen {
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
