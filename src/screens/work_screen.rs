use crate::backend::*;
use crate::database::*;
use crate::widgets::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use libhandy::HeaderBarExt;
use std::cell::RefCell;
use std::rc::Rc;

pub struct WorkScreen {
    pub widget: gtk::Box,
    stack: gtk::Stack,
    recording_list: Rc<List<RecordingDescription>>,
    back: RefCell<Option<Box<dyn Fn() -> () + 'static>>>,
}

impl WorkScreen {
    pub fn new(backend: Rc<Backend>, work: WorkDescription) -> Rc<Self> {
        let builder = gtk::Builder::from_resource("/de/johrpan/musicus_editor/ui/work_screen.ui");

        get_widget!(builder, gtk::Box, widget);
        get_widget!(builder, libhandy::HeaderBar, header);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::MenuButton, menu_button);
        get_widget!(builder, gtk::SearchEntry, search_entry);
        get_widget!(builder, gtk::Stack, stack);
        get_widget!(builder, gtk::Frame, recording_frame);

        header.set_title(Some(&work.title));
        header.set_subtitle(Some(&work.composer.name_fl()));

        let recording_list = List::new(
            |recording: &RecordingDescription| {
                let work_label = gtk::Label::new(Some(&recording.work.get_title()));

                work_label.set_ellipsize(pango::EllipsizeMode::End);
                work_label.set_halign(gtk::Align::Start);

                let performers_label = gtk::Label::new(Some(&recording.get_performers()));
                performers_label.set_ellipsize(pango::EllipsizeMode::End);
                performers_label.set_opacity(0.5);
                performers_label.set_halign(gtk::Align::Start);

                let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
                vbox.add(&work_label);
                vbox.add(&performers_label);

                vbox.upcast()
            },
            clone!(@strong search_entry => move |recording: &RecordingDescription| {
                let search = search_entry.get_text().to_string().to_lowercase();
                let text = recording.work.get_title() + &recording.get_performers();
                search.is_empty() || text.contains(&search)
            }),
            "No recordings found.",
        );

        recording_frame.add(&recording_list.widget);

        let result = Rc::new(Self {
            widget,
            stack,
            recording_list,
            back: RefCell::new(None),
        });

        search_entry.connect_search_changed(clone!(@strong result => move |_| {
            result.recording_list.invalidate_filter();
        }));

        back_button.connect_clicked(clone!(@strong result => move |_| {
            if let Some(back) = &*result.back.borrow() {
                back();
            }
        }));

        let context = glib::MainContext::default();
        let clone = result.clone();
        context.spawn_local(async move {
            let recordings = backend.get_recordings_for_work(work.id).await.unwrap();

            if recordings.is_empty() {
                clone.stack.set_visible_child_name("nothing");
            } else {
                clone.recording_list.show_items(recordings);
                clone.stack.set_visible_child_name("content");
            }
        });

        result
    }

    pub fn set_back<B>(&self, back: B)
    where
        B: Fn() -> () + 'static,
    {
        self.back.replace(Some(Box::new(back)));
    }

    pub fn set_recording_selected<S>(&self, selected: S)
    where
        S: Fn(&RecordingDescription) -> () + 'static,
    {
        self.recording_list.set_selected(selected);
    }
}
