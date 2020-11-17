use crate::backend::*;
use crate::database::*;
use crate::dialogs::{RecordingEditorDialog, TracksEditor};
use crate::player::*;
use crate::widgets::*;
use gettextrs::gettext;
use gio::prelude::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use libhandy::HeaderBarExt;
use std::cell::RefCell;
use std::rc::Rc;

pub struct RecordingScreen {
    backend: Rc<Backend>,
    window: gtk::Window,
    recording: Recording,
    widget: gtk::Box,
    stack: gtk::Stack,
    tracks: RefCell<Vec<Track>>,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl RecordingScreen {
    pub fn new<W>(backend: Rc<Backend>, window: &W, recording: Recording) -> Rc<Self>
    where
        W: IsA<gtk::Window>,
    {
        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/recording_screen.ui");

        get_widget!(builder, gtk::Box, widget);
        get_widget!(builder, libhandy::HeaderBar, header);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::Stack, stack);
        get_widget!(builder, gtk::Frame, frame);
        get_widget!(builder, gtk::Button, add_to_playlist_button);

        header.set_title(Some(&recording.work.get_title()));
        header.set_subtitle(Some(&recording.get_performers()));

        let edit_action = gio::SimpleAction::new("edit", None);
        let delete_action = gio::SimpleAction::new("delete", None);
        let edit_tracks_action = gio::SimpleAction::new("edit-tracks", None);
        let delete_tracks_action = gio::SimpleAction::new("delete-tracks", None);

        let actions = gio::SimpleActionGroup::new();
        actions.add_action(&edit_action);
        actions.add_action(&delete_action);
        actions.add_action(&edit_tracks_action);
        actions.add_action(&delete_tracks_action);

        widget.insert_action_group("widget", Some(&actions));

        let list = List::new(&gettext("No tracks found."));
        frame.add(&list.widget);

        let result = Rc::new(Self {
            backend,
            window: window.clone().upcast(),
            recording,
            widget,
            stack,
            tracks: RefCell::new(Vec::new()),
            navigator: RefCell::new(None),
        });

        list.set_make_widget(clone!(@strong result => move |track: &Track| {
            let mut title_parts = Vec::<String>::new();
            for part in &track.work_parts {
                title_parts.push(result.recording.work.parts[*part].title.clone());
            }

            let title = if title_parts.is_empty() {
                gettext("Unknown")
            } else {
                title_parts.join(", ")
            };

            let title_label = gtk::Label::new(Some(&title));
            title_label.set_ellipsize(pango::EllipsizeMode::End);
            title_label.set_halign(gtk::Align::Start);

            let file_name_label = gtk::Label::new(Some(&track.file_name));
            file_name_label.set_ellipsize(pango::EllipsizeMode::End);
            file_name_label.set_opacity(0.5);
            file_name_label.set_halign(gtk::Align::Start);

            let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
            vbox.set_border_width(6);
            vbox.add(&title_label);
            vbox.add(&file_name_label);

            vbox.upcast()
        }));

        back_button.connect_clicked(clone!(@strong result => move |_| {
            let navigator = result.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                navigator.clone().pop();
            }
        }));

        add_to_playlist_button.connect_clicked(clone!(@strong result => move |_| {
            if let Some(player) = result.backend.get_player() {
                player.add_item(PlaylistItem {
                    recording: result.recording.clone(),
                    tracks: result.tracks.borrow().clone(),
                }).unwrap();
            }
        }));

        edit_action.connect_activate(clone!(@strong result => move |_, _| {
            RecordingEditorDialog::new(result.backend.clone(), &result.window, Some(result.recording.clone())).show();
        }));

        delete_action.connect_activate(clone!(@strong result => move |_, _| {
            let context = glib::MainContext::default();
            let clone = result.clone();
            context.spawn_local(async move {
                clone.backend.db().delete_recording(clone.recording.id).await.unwrap();
            });
        }));

        edit_tracks_action.connect_activate(clone!(@strong result => move |_, _| {
            TracksEditor::new(result.backend.clone(), &result.window, Some(result.recording.clone()), result.tracks.borrow().clone()).show();
        }));

        delete_tracks_action.connect_activate(clone!(@strong result => move |_, _| {
            let context = glib::MainContext::default();
            let clone = result.clone();
            context.spawn_local(async move {
                clone.backend.db().delete_tracks(clone.recording.id).await.unwrap();
            });
        }));

        let context = glib::MainContext::default();
        let clone = result.clone();
        context.spawn_local(async move {
            let tracks = clone
                .backend
                .db()
                .get_tracks(clone.recording.id)
                .await
                .unwrap();

            list.show_items(tracks.clone());
            clone.stack.set_visible_child_name("content");
            clone.tracks.replace(tracks);
        });

        result
    }
}

impl NavigatorScreen for RecordingScreen {
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
