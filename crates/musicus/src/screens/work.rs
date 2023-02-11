use super::RecordingScreen;
use crate::editors::WorkEditor;
use crate::navigator::{NavigationHandle, NavigatorWindow, Screen};
use crate::widgets;
use crate::widgets::{List, Section, Widget};
use adw::builders::ActionRowBuilder;
use adw::prelude::*;
use gettextrs::gettext;
use glib::clone;
use musicus_backend::db::{self, Recording, Work};
use std::cell::RefCell;
use std::rc::Rc;

/// A screen for showing recordings of a work.
pub struct WorkScreen {
    handle: NavigationHandle<()>,
    work: Work,
    widget: widgets::Screen,
    recording_list: Rc<List>,
    recordings: RefCell<Vec<Recording>>,
}

impl Screen<Work, ()> for WorkScreen {
    /// Create a new work screen for the specified work and load the
    /// contents asynchronously.
    fn new(work: Work, handle: NavigationHandle<()>) -> Rc<Self> {
        let widget = widgets::Screen::new();
        widget.set_title(&work.title);
        widget.set_subtitle(&work.composer.name_fl());

        let recording_list = List::new();

        let this = Rc::new(Self {
            handle,
            work,
            widget,
            recording_list,
            recordings: RefCell::new(Vec::new()),
        });

        this.widget.set_back_cb(clone!(@weak this =>  move || {
            this.handle.pop(None);
        }));

        this.widget.add_action(
            &gettext("Edit work"),
            clone!(@weak this =>  move || {
                spawn!(@clone this, async move {
                    let window = NavigatorWindow::new(this.handle.backend.clone());
                    replace!(window.navigator, WorkEditor, Some(this.work.clone())).await;
                });
            }),
        );

        this.widget.add_action(
            &gettext("Delete work"),
            clone!(@weak this =>  move || {
                spawn!(@clone this, async move {
                    db::delete_work(&mut this.handle.backend.db().lock().unwrap(), &this.work.id).unwrap();
                    this.handle.backend.library_changed();
                });
            }),
        );

        this.widget.set_search_cb(clone!(@weak this =>  move || {
            this.recording_list.invalidate_filter();
        }));

        this.recording_list.set_make_widget_cb(
            clone!(@weak this =>  @default-panic, move |index| {
                let recording = &this.recordings.borrow()[index];

                let row = ActionRowBuilder::new()
                    .activatable(true)
                    .title(&recording.work.get_title())
                    .subtitle(&recording.get_performers())
                    .build();

                let recording = recording.to_owned();
                row.connect_activated(clone!(@weak this =>  move |_| {
                    let recording = recording.clone();
                    spawn!(@clone this, async move {
                        push!(this.handle, RecordingScreen, recording.clone()).await;
                    });
                }));

                row.upcast()
            }),
        );

        this.recording_list
            .set_filter_cb(clone!(@weak this =>  @default-panic, move |index| {
                let recording = &this.recordings.borrow()[index];
                let search = this.widget.get_search();
                let text = recording.work.get_title() + &recording.get_performers();
                search.is_empty() || text.to_lowercase().contains(&search)
            }));

        // Load the content.

        let recordings = db::get_recordings_for_work(
            &mut this.handle.backend.db().lock().unwrap(),
            &this.work.id,
        )
        .unwrap();

        if !recordings.is_empty() {
            let length = recordings.len();
            this.recordings.replace(recordings);
            this.recording_list.update(length);

            let section = Section::new("Recordings", &this.recording_list.widget);
            this.widget.add_content(&section.widget);
        }

        this.widget.ready();

        this
    }
}

impl Widget for WorkScreen {
    fn get_widget(&self) -> gtk::Widget {
        self.widget.widget.clone().upcast()
    }
}
