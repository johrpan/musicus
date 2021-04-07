use crate::navigator::{NavigationHandle, Screen};
use crate::widgets::Widget;
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use musicus_backend::db::Medium;
use musicus_backend::import::ImportSession;
use std::rc::Rc;
use std::sync::Arc;

/// A dialog for presenting the selected medium when importing music.
pub struct MediumPreview {
    handle: NavigationHandle<()>,
    session: Arc<ImportSession>,
    widget: gtk::Box,
}

impl Screen<(Arc<ImportSession>, Medium), ()> for MediumPreview {
    /// Create a new medium preview screen.
    fn new((session, medium): (Arc<ImportSession>, Medium), handle: NavigationHandle<()>) -> Rc<Self> {
        // Create UI

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/medium_preview.ui");

        get_widget!(builder, gtk::Box, widget);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::Button, import_button);
        get_widget!(builder, gtk::Stack, done_stack);
        get_widget!(builder, gtk::Box, medium_box);
        get_widget!(builder, gtk::Label, name_label);

        let this = Rc::new(Self {
            handle,
            session,
            widget,
        });

        // Connect signals and callbacks

        back_button.connect_clicked(clone!(@weak this => move |_| {
            this.handle.pop(None);
        }));

        // Populate the widget

        name_label.set_text(&medium.name);

        let mut last_recording_id = "";
        let mut last_list = None::<gtk::ListBox>;

        for track in &medium.tracks {
            if track.recording.id != last_recording_id {
                last_recording_id = &track.recording.id;

                let list = gtk::ListBoxBuilder::new()
                    .selection_mode(gtk::SelectionMode::None)
                    .build();

                let header = libadwaita::ActionRowBuilder::new()
                    .activatable(false)
                    .title(&track.recording.work.get_title())
                    .subtitle(&track.recording.get_performers())
                    .build();

                list.append(&header);

                if let Some(list) = &last_list {
                    let frame = gtk::FrameBuilder::new()
                        .margin_bottom(12)
                        .build();

                    frame.set_child(Some(list));
                    medium_box.append(&frame);

                    last_list = None;
                }

                last_list = Some(list);
            }

            if let Some(list) = &last_list {
                let mut parts = Vec::<String>::new();
                for part in &track.work_parts {
                    parts.push(track.recording.work.parts[*part].title.clone());
                }

                let title = if parts.is_empty() {
                    gettext("Unknown")
                } else {
                    parts.join(", ")
                };

                let row = libadwaita::ActionRowBuilder::new()
                    .activatable(false)
                    .title(&title)
                    .subtitle(&track.path)
                    .margin_start(12)
                    .build();

                list.append(&row);
            }
        }

        if let Some(list) = &last_list {
            let frame = gtk::FrameBuilder::new()
                .margin_bottom(12)
                .build();

            frame.set_child(Some(list));
            medium_box.append(&frame);
        }

        this
    }
}

impl Widget for MediumPreview {
    fn get_widget(&self) -> gtk::Widget {
        self.widget.clone().upcast()
    }
}
