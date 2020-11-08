use crate::backend::*;
use crate::database::*;
use crate::player::*;
use crate::widgets::*;
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use libhandy::HeaderBarExt;
use std::cell::RefCell;
use std::rc::Rc;

pub struct RecordingScreen {
    backend: Rc<Backend>,
    widget: gtk::Box,
    stack: gtk::Stack,
    tracks: RefCell<Vec<TrackDescription>>,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl RecordingScreen {
    pub fn new(backend: Rc<Backend>, recording: RecordingDescription) -> Rc<Self> {
        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/recording_screen.ui");

        get_widget!(builder, gtk::Box, widget);
        get_widget!(builder, libhandy::HeaderBar, header);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::MenuButton, menu_button);
        get_widget!(builder, gtk::Stack, stack);
        get_widget!(builder, gtk::Frame, frame);
        get_widget!(builder, gtk::Button, add_to_playlist_button);

        header.set_title(Some(&recording.work.get_title()));
        header.set_subtitle(Some(&recording.get_performers()));

        let edit_menu_item = gio::MenuItem::new(Some(&gettext("Edit recording")), None);
        edit_menu_item.set_action_and_target_value(
            Some("win.edit-recording"),
            Some(&glib::Variant::from(recording.id)),
        );

        let delete_menu_item = gio::MenuItem::new(Some(&gettext("Delete recording")), None);
        delete_menu_item.set_action_and_target_value(
            Some("win.delete-recording"),
            Some(&glib::Variant::from(recording.id)),
        );

        let edit_tracks_menu_item = gio::MenuItem::new(Some(&gettext("Edit tracks")), None);
        edit_tracks_menu_item.set_action_and_target_value(
            Some("win.edit-tracks"),
            Some(&glib::Variant::from(recording.id)),
        );

        let delete_tracks_menu_item = gio::MenuItem::new(Some(&gettext("Delete tracks")), None);
        delete_tracks_menu_item.set_action_and_target_value(
            Some("win.delete-tracks"),
            Some(&glib::Variant::from(recording.id)),
        );

        let menu = gio::Menu::new();
        menu.append_item(&edit_menu_item);
        menu.append_item(&delete_menu_item);
        menu.append_item(&edit_tracks_menu_item);
        menu.append_item(&delete_tracks_menu_item);

        menu_button.set_menu_model(Some(&menu));

        let recording = Rc::new(recording);
        let list = List::new(&gettext("No tracks found."));

        list.set_make_widget(
            clone!(@strong recording => move |track: &TrackDescription| {
                let mut title_parts = Vec::<String>::new();
                for part in &track.work_parts {
                    title_parts.push(recording.work.parts[*part].title.clone());
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
            }),
        );

        frame.add(&list.widget);

        let result = Rc::new(Self {
            backend,
            widget,
            stack,
            tracks: RefCell::new(Vec::new()),
            navigator: RefCell::new(None),
        });

        back_button.connect_clicked(clone!(@strong result => move |_| {
            let navigator = result.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                navigator.clone().pop();
            }
        }));

        add_to_playlist_button.connect_clicked(
            clone!(@strong result, @strong recording => move |_| {
                if let Some(player) = result.backend.get_player() {
                    player.add_item(PlaylistItem {
                        recording: (*recording).clone(),
                        tracks: result.tracks.borrow().clone(),
                    }).unwrap();
                }
            }),
        );

        let context = glib::MainContext::default();
        let clone = result.clone();
        let id = recording.id;
        context.spawn_local(async move {
            let tracks = clone.backend.get_tracks(id).await.unwrap();
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
