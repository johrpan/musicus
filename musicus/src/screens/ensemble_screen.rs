use super::*;
use crate::backend::*;
use crate::database::*;
use crate::dialogs::EnsembleEditor;
use crate::widgets::*;
use gettextrs::gettext;
use gio::prelude::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use libhandy::HeaderBarExt;
use std::cell::RefCell;
use std::rc::Rc;

pub struct EnsembleScreen {
    backend: Rc<Backend>,
    window: gtk::Window,
    ensemble: Ensemble,
    widget: gtk::Box,
    stack: gtk::Stack,
    recording_list: Rc<List<Recording>>,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl EnsembleScreen {
    pub fn new<W>(backend: Rc<Backend>, window: &W, ensemble: Ensemble) -> Rc<Self>
    where
        W: IsA<gtk::Window>,
    {
        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/ensemble_screen.ui");

        get_widget!(builder, gtk::Box, widget);
        get_widget!(builder, libhandy::HeaderBar, header);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::SearchEntry, search_entry);
        get_widget!(builder, gtk::Stack, stack);
        get_widget!(builder, gtk::Frame, recording_frame);

        header.set_title(Some(&ensemble.name));

        let edit_action = gio::SimpleAction::new("edit", None);
        let delete_action = gio::SimpleAction::new("delete", None);

        let actions = gio::SimpleActionGroup::new();
        actions.add_action(&edit_action);
        actions.add_action(&delete_action);

        widget.insert_action_group("widget", Some(&actions));

        let recording_list = List::new(&gettext("No recordings found."));

        recording_list.set_make_widget(|recording: &Recording| {
            let work_label = gtk::Label::new(Some(&recording.work.get_title()));

            work_label.set_ellipsize(pango::EllipsizeMode::End);
            work_label.set_halign(gtk::Align::Start);

            let performers_label = gtk::Label::new(Some(&recording.get_performers()));
            performers_label.set_ellipsize(pango::EllipsizeMode::End);
            performers_label.set_opacity(0.5);
            performers_label.set_halign(gtk::Align::Start);

            let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
            vbox.set_border_width(6);
            vbox.add(&work_label);
            vbox.add(&performers_label);

            vbox.upcast()
        });

        recording_list.set_filter(
            clone!(@strong search_entry => move |recording: &Recording| {
                let search = search_entry.get_text().to_string().to_lowercase();
                let text = recording.work.get_title() + &recording.get_performers();
                search.is_empty() || text.contains(&search)
            }),
        );

        recording_frame.add(&recording_list.widget.clone());

        let result = Rc::new(Self {
            backend,
            window: window.clone().upcast(),
            ensemble,
            widget,
            stack,
            recording_list,
            navigator: RefCell::new(None),
        });

        search_entry.connect_search_changed(clone!(@strong result => move |_| {
            result.recording_list.invalidate_filter();
        }));

        back_button.connect_clicked(clone!(@strong result => move |_| {
            let navigator = result.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                navigator.pop();
            }
        }));

        result
            .recording_list
            .set_selected(clone!(@strong result => move |recording| {
                let navigator = result.navigator.borrow().clone();
                if let Some(navigator) = navigator {
                    navigator.push(RecordingScreen::new(result.backend.clone(), &result.window, recording.clone()));
                }
            }));

        edit_action.connect_activate(clone!(@strong result => move |_, _| {
            EnsembleEditor::new(result.backend.clone(), &result.window, Some(result.ensemble.clone()), |_| {}).show();
        }));

        delete_action.connect_activate(clone!(@strong result => move |_, _| {
            let context = glib::MainContext::default();
            let clone = result.clone();
            context.spawn_local(async move {
                clone.backend.db().delete_ensemble(&clone.ensemble.id).await.unwrap();
            });
        }));

        let context = glib::MainContext::default();
        let clone = result.clone();
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
                clone.recording_list.show_items(recordings);
                clone.stack.set_visible_child_name("content");
            }
        });

        result
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
