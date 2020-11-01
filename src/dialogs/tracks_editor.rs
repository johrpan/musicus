use super::*;
use crate::backend::*;
use crate::database::*;
use crate::widgets::*;
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use std::cell::RefCell;
use std::rc::Rc;

pub struct TracksEditor {
    window: gtk::Window,
}

impl TracksEditor {
    pub fn new<F: Fn() -> () + 'static, P: IsA<gtk::Window>>(
        backend: Rc<Backend>,
        parent: &P,
        callback: F,
    ) -> Self {
        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/tracks_editor.ui");

        get_widget!(builder, gtk::Window, window);
        get_widget!(builder, gtk::Button, cancel_button);
        get_widget!(builder, gtk::Button, save_button);
        get_widget!(builder, gtk::Button, recording_button);
        get_widget!(builder, gtk::Stack, recording_stack);
        get_widget!(builder, gtk::Label, work_label);
        get_widget!(builder, gtk::Label, performers_label);
        get_widget!(builder, gtk::ScrolledWindow, scroll);
        get_widget!(builder, gtk::Button, add_track_button);
        get_widget!(builder, gtk::Button, edit_track_button);
        get_widget!(builder, gtk::Button, remove_track_button);
        get_widget!(builder, gtk::Button, move_track_up_button);
        get_widget!(builder, gtk::Button, move_track_down_button);

        window.set_transient_for(Some(parent));

        cancel_button.connect_clicked(clone!(@strong window => move |_| {
            window.close();
        }));

        let recording = Rc::new(RefCell::new(None::<RecordingDescription>));
        let tracks = Rc::new(RefCell::new(Vec::<TrackDescription>::new()));

        let track_list = List::new(
            clone!(@strong recording => move |track: &TrackDescription| {
                let mut title_parts = Vec::<String>::new();
                for part in &track.work_parts {
                    if let Some(recording) = &*recording.borrow() {
                        title_parts.push(recording.work.parts[*part].title.clone());
                    }
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
                vbox.add(&title_label);
                vbox.add(&file_name_label);

                vbox.upcast()
            }),
            |_| true,
            &gettext("Add some tracks."),
        );

        let autofill_parts = Rc::new(clone!(@strong recording, @strong tracks, @strong track_list => move || {
            if let Some(recording) = &*recording.borrow() {
                let mut tracks = tracks.borrow_mut();
                for (index, _) in recording.work.parts.iter().enumerate() {
                    if let Some(mut track) = tracks.get_mut(index) {
                        track.work_parts = vec!(index);
                    } else {
                        break;
                    }
                }

                track_list.show_items(tracks.clone());
            }
        }));

        recording_button.connect_clicked(clone!(
            @strong backend,
            @strong window,
            @strong save_button,
            @strong work_label,
            @strong performers_label,
            @strong recording_stack,
            @strong recording,
            @strong autofill_parts => move |_| {
                RecordingSelector::new(
                    backend.clone(),
                    &window,
                    clone!(
                        @strong save_button,
                        @strong work_label,
                        @strong performers_label,
                        @strong recording_stack,
                        @strong recording,
                        @strong autofill_parts => move |r| {
                            work_label.set_text(&r.work.get_title());
                            performers_label.set_text(&r.get_performers());
                            recording_stack.set_visible_child_name("selected");
                            recording.replace(Some(r));
                            save_button.set_sensitive(true);
                            autofill_parts();
                        }
                    )).show();
            }
        ));

        let callback = Rc::new(callback);
        save_button.connect_clicked(clone!(@strong window, @strong backend, @strong recording, @strong tracks, @strong callback => move |_| {
            let context = glib::MainContext::default();
            let window = window.clone();
            let backend = backend.clone();
            let recording = recording.clone();
            let tracks = tracks.clone();
            let callback = callback.clone();
            context.spawn_local(async move {
                backend.add_tracks(recording.borrow().as_ref().unwrap().id, tracks.borrow().clone()).await.unwrap();
                callback();
                window.close();
            });

        }));

        add_track_button.connect_clicked(clone!(@strong window, @strong tracks, @strong track_list, @strong autofill_parts => move |_| {
            let music_library_path = backend.get_music_library_path().unwrap();

            let dialog = gtk::FileChooserNative::new(Some(&gettext("Select audio files")), Some(&window), gtk::FileChooserAction::Open, None, None);
            dialog.set_select_multiple(true);
            dialog.set_current_folder(&music_library_path);

            if let gtk::ResponseType::Accept = dialog.run() {
                let mut index = match track_list.get_selected_index() {
                    Some(index) => index + 1,
                    None => tracks.borrow().len(),
                };

                {
                    let mut tracks = tracks.borrow_mut();
                    for file_name in dialog.get_filenames() {
                        let file_name = file_name.strip_prefix(&music_library_path).unwrap();
                        tracks.insert(index, TrackDescription {
                            work_parts: Vec::new(),
                            file_name: String::from(file_name.to_str().unwrap()),
                        });
                        index += 1;
                    }
                }
                
                track_list.show_items(tracks.borrow().clone());
                autofill_parts();
                track_list.select_index(index);
            }
        }));

        remove_track_button.connect_clicked(
            clone!(@strong tracks, @strong track_list => move |_| {
                match track_list.get_selected_index() {
                    Some(index) => {
                        tracks.borrow_mut().remove(index);
                        track_list.show_items(tracks.borrow().clone());
                        track_list.select_index(index);
                    }
                    None => (),
                }
            }),
        );

        move_track_up_button.connect_clicked(
            clone!(@strong tracks, @strong track_list => move |_| {
                match track_list.get_selected_index() {
                    Some(index) => {
                        if index > 0 {
                            tracks.borrow_mut().swap(index - 1, index);
                            track_list.show_items(tracks.borrow().clone());
                            track_list.select_index(index - 1);
                        }
                    }
                    None => (),
                }
            }),
        );

        move_track_down_button.connect_clicked(
            clone!(@strong tracks, @strong track_list => move |_| {
                match track_list.get_selected_index() {
                    Some(index) => {
                        if index < tracks.borrow().len() - 1 {
                            tracks.borrow_mut().swap(index, index + 1);
                            track_list.show_items(tracks.borrow().clone());
                            track_list.select_index(index + 1);
                        }
                    }
                    None => (),
                }
            }),
        );

        edit_track_button.connect_clicked(clone!(@strong window, @strong tracks, @strong track_list, @strong recording => move |_| {
            if let Some(index) = track_list.get_selected_index() {
                if let Some(recording) = &*recording.borrow() {
                    TrackEditor::new(&window, tracks.borrow()[index].clone(), recording.work.clone(), clone!(@strong tracks, @strong track_list => move |track| {
                        let mut tracks = tracks.borrow_mut();
                        tracks[index] = track;
                        track_list.show_items(tracks.clone());
                        track_list.select_index(index);
                    })).show();
                }
            }
        }));

        scroll.add(&track_list.widget);

        Self { window }
    }

    pub fn show(&self) {
        self.window.show();
    }
}
