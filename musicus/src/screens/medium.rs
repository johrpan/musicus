use crate::navigator::{NavigationHandle, Screen};
use crate::widgets;
use crate::widgets::{List, Section, Widget};
use adw::prelude::*;
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use musicus_backend::db::Medium;
use std::rc::Rc;

/// A screen for showing the contents of a medium.
pub struct MediumScreen {
    handle: NavigationHandle<()>,
    medium: Medium,
    widget: widgets::Screen,
    list: Rc<List>,
}

impl Screen<Medium, ()> for MediumScreen {
    /// Create a new medium screen for the specified medium and load the
    /// contents asynchronously.
    fn new(medium: Medium, handle: NavigationHandle<()>) -> Rc<Self> {
        let widget = widgets::Screen::new();
        widget.set_title(&medium.name);

        let list = List::new();
        let section = Section::new("Recordings", &list.widget);
        widget.add_content(&section.widget);
        widget.ready();

        let this = Rc::new(Self {
            handle,
            medium,
            widget,
            list,
        });

        this.widget.set_back_cb(clone!(@weak this =>  move || {
            this.handle.pop(None);
        }));

        this.widget.add_action(
            &gettext("Edit medium"),
            clone!(@weak this =>  move || {
                // TODO: Show medium editor.
            }),
        );

        this.widget.add_action(
            &gettext("Delete medium"),
            clone!(@weak this =>  move || {
                // TODO: Delete medium and maybe also the tracks?
            }),
        );

        section.add_action(
            "media-playback-start-symbolic",
            clone!(@weak this =>  move || {
                for track in &this.medium.tracks {
                    this.handle.backend.pl().add_item(track.clone()).unwrap();
                }
            }),
        );

        this.list
            .set_make_widget_cb(clone!(@weak this =>  @default-panic, move |index| {
                let track = &this.medium.tracks[index];

                let mut parts = Vec::<String>::new();
                for part in &track.work_parts {
                    parts.push(track.recording.work.parts[*part].title.clone());
                }

                let title = if parts.is_empty() {
                    gettext("Unknown")
                } else {
                    parts.join(", ")
                };

                let row = adw::ActionRow::new();
                row.set_selectable(false);
                row.set_activatable(false);
                row.set_title(Some(&title));
                row.set_margin_start(12);

                row.upcast()
            }));

        this.list.update(this.medium.tracks.len());

        this
    }
}

impl Widget for MediumScreen {
    fn get_widget(&self) -> gtk::Widget {
        self.widget.widget.clone().upcast()
    }
}
