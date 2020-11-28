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

/// A dialog for editing a set of tracks.
// TODO: Disable buttons if no track is selected.
pub struct TracksEditor {
    backend: Rc<Backend>,
    window: libhandy::Window,
    save_button: gtk::Button,
    recording_stack: gtk::Stack,
    work_label: gtk::Label,
    performers_label: gtk::Label,
    track_list: Rc<List<Track>>,
    recording: RefCell<Option<Recording>>,
    tracks: RefCell<Vec<Track>>,
    callback: RefCell<Option<Box<dyn Fn() -> ()>>>,
}

impl TracksEditor {
    /// Create a new track editor an optionally initialize it with a recording and a list of
    /// tracks.
    pub fn new<P: IsA<gtk::Window>>(
        backend: Rc<Backend>,
        parent: &P,
        recording: Option<Recording>,
        tracks: Vec<Track>,
    ) -> Rc<Self> {
        // UI setup

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/tracks_editor.ui");

        get_widget!(builder, libhandy::Window, window);
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

        let track_list = List::new(&gettext("Add some tracks."));
        scroll.add(&track_list.widget);

        let this = Rc::new(Self {
            backend,
            window,
            save_button,
            recording_stack,
            work_label,
            performers_label,
            track_list,
            recording: RefCell::new(recording),
            tracks: RefCell::new(tracks),
            callback: RefCell::new(None),
        });

        // Signals and callbacks

        this.save_button
            .connect_clicked(clone!(@strong this => move |_| {
                let context = glib::MainContext::default();
                let this = this.clone();
                context.spawn_local(async move {
                    this.backend.db().update_tracks(
                        &this.recording.borrow().as_ref().unwrap().id,
                        this.tracks.borrow().clone(),
                    ).await.unwrap();

                    if let Some(callback) = &*this.callback.borrow() {
                        callback();
                    }

                    this.window.close();
                });

            }));

        recording_button.connect_clicked(clone!(@strong this => move |_| {
            let dialog = RecordingDialog::new(this.backend.clone(), &this.window);

            dialog.set_selected_cb(clone!(@strong this => move |recording| {
                this.recording_selected(&recording);
                this.recording.replace(Some(recording));
            }));

            dialog.show();
            }
        ));

        this.track_list
            .set_make_widget(clone!(@strong this => move |track| {
                this.build_track_row(track)
            }));

        add_track_button.connect_clicked(clone!(@strong this => move |_| {
            let music_library_path = this.backend.get_music_library_path().unwrap();

            let dialog = gtk::FileChooserNative::new(
                Some(&gettext("Select audio files")),
                Some(&this.window),
                gtk::FileChooserAction::Open,
                None,
                None,
            );

            dialog.set_select_multiple(true);
            dialog.set_current_folder(&music_library_path);

            if let gtk::ResponseType::Accept = dialog.run() {
                let mut index = match this.track_list.get_selected_index() {
                    Some(index) => index + 1,
                    None => this.tracks.borrow().len(),
                };

                {
                    let mut tracks = this.tracks.borrow_mut();
                    for file_name in dialog.get_filenames() {
                        let file_name = file_name.strip_prefix(&music_library_path).unwrap();
                        tracks.insert(index, Track {
                            work_parts: Vec::new(),
                            file_name: String::from(file_name.to_str().unwrap()),
                        });
                        index += 1;
                    }
                }

                this.track_list.show_items(this.tracks.borrow().clone());
                this.autofill_parts();
                this.track_list.select_index(index);
            }
        }));

        remove_track_button.connect_clicked(clone!(@strong this => move |_| {
            match this.track_list.get_selected_index() {
                Some(index) => {
                    let mut tracks = this.tracks.borrow_mut();
                    tracks.remove(index);
                    this.track_list.show_items(tracks.clone());
                    this.track_list.select_index(index);
                }
                None => (),
            }
        }));

        move_track_up_button.connect_clicked(clone!(@strong this => move |_| {
            match this.track_list.get_selected_index() {
                Some(index) => {
                    if index > 0 {
                        let mut tracks = this.tracks.borrow_mut();
                        tracks.swap(index - 1, index);
                        this.track_list.show_items(tracks.clone());
                        this.track_list.select_index(index - 1);
                    }
                }
                None => (),
            }
        }));

        move_track_down_button.connect_clicked(clone!(@strong this => move |_| {
            match this.track_list.get_selected_index() {
                Some(index) => {
                    let mut tracks = this.tracks.borrow_mut();
                    if index < tracks.len() - 1 {
                        tracks.swap(index, index + 1);
                        this.track_list.show_items(tracks.clone());
                        this.track_list.select_index(index + 1);
                    }
                }
                None => (),
            }
        }));

        edit_track_button.connect_clicked(clone!(@strong this => move |_| {
            if let Some(index) = this.track_list.get_selected_index() {
                if let Some(recording) = &*this.recording.borrow() {
                    TrackEditor::new(&this.window, this.tracks.borrow()[index].clone(), recording.work.clone(), clone!(@strong this => move |track| {
                        let mut tracks = this.tracks.borrow_mut();
                        tracks[index] = track;
                        this.track_list.show_items(tracks.clone());
                        this.track_list.select_index(index);
                    })).show();
                }
            }
        }));

        // Initialization

        if let Some(recording) = &*this.recording.borrow() {
            this.recording_selected(recording);
        }

        this.track_list.show_items(this.tracks.borrow().clone());

        this
    }

    /// Set a callback to be called when the tracks are saved.
    pub fn set_callback<F: Fn() -> () + 'static>(&self, cb: F) {
        self.callback.replace(Some(Box::new(cb)));
    }

    /// Open the track editor.
    pub fn show(&self) {
        self.window.show();
    }

    /// Create a widget representing a track.
    fn build_track_row(&self, track: &Track) -> gtk::Widget {
        let mut title_parts = Vec::<String>::new();
        for part in &track.work_parts {
            if let Some(recording) = &*self.recording.borrow() {
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
        vbox.set_border_width(6);
        vbox.add(&title_label);
        vbox.add(&file_name_label);

        vbox.upcast()
    }

    /// Set everything up after selecting a recording.
    fn recording_selected(&self, recording: &Recording) {
        self.work_label.set_text(&recording.work.get_title());
        self.performers_label.set_text(&recording.get_performers());
        self.recording_stack.set_visible_child_name("selected");
        self.save_button.set_sensitive(true);
        self.autofill_parts();
    }

    /// Automatically try to put work part information from the selected recording into the
    /// selected tracks.
    fn autofill_parts(&self) {
        if let Some(recording) = &*self.recording.borrow() {
            let mut tracks = self.tracks.borrow_mut();

            for (index, _) in recording.work.parts.iter().enumerate() {
                if let Some(mut track) = tracks.get_mut(index) {
                    track.work_parts = vec![index];
                } else {
                    break;
                }
            }

            self.track_list.show_items(tracks.clone());
        }
    }
}
