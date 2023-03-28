use super::selector::Selector;
use crate::editors::{PersonEditor, RecordingEditor, WorkEditor};
use crate::navigator::{NavigationHandle, Screen};
use crate::widgets::Widget;

use adw::prelude::*;
use gettextrs::gettext;
use glib::clone;
use log::warn;
use musicus_backend::db::{self, Person, Recording, Work};
use std::rc::Rc;

/// A screen for selecting a recording.
pub struct RecordingSelector {
    handle: NavigationHandle<Recording>,
    selector: Rc<Selector<Person>>,
}

impl Screen<(), Recording> for RecordingSelector {
    fn new(_: (), handle: NavigationHandle<Recording>) -> Rc<Self> {
        // Create UI

        let selector = Selector::<Person>::new();
        selector.set_title(&gettext("Select composer"));

        let this = Rc::new(Self { handle, selector });

        // Connect signals and callbacks

        this.selector.set_back_cb(clone!(@weak this =>  move || {
            this.handle.pop(None);
        }));

        this.selector.set_add_cb(clone!(@weak this =>  move || {
            spawn!(@clone this, async move {
                if let Some(person) = push!(this.handle, PersonEditor, None).await {
                    // We can assume that there are no existing works of this composer and
                    // immediately show the work editor. Going back from the work editor will
                    // correctly show the person selector again.

                    let work = Work::from_composer(person);
                    if let Some(work) = push!(this.handle, WorkEditor, Some(work)).await {
                        // There will also be no existing recordings, so we show the recording
                        // editor next.

                        let recording = Recording::from_work(work);
                        if let Some(recording) = push!(this.handle, RecordingEditor, Some(recording)).await {
                            this.handle.pop(Some(recording));
                        }
                    }
                }
            });
        }));

        this.selector.set_make_widget(clone!(@weak this =>  @default-panic, move |person| {
            let row = adw::ActionRow::builder()
                .activatable(true)
                .title(person.name_lf())
                .build();

            let person = person.to_owned();
            row.connect_activated(clone!(@weak this =>  move |_| {
                // Instead of returning the person from here, like the person selector does, we
                // show a second selector for choosing the work.

                let person = person.clone();
                spawn!(@clone this, async move {
                    if let Some(work) = push!(this.handle, RecordingSelectorWorkScreen, person).await {
                        // Now the user can select a recording for that work.

                        if let Some(recording) = push!(this.handle, RecordingSelectorRecordingScreen, work).await {
                            this.handle.pop(Some(recording));
                        }
                    }
                });
            }));

            row.upcast()
        }));

        this.selector
            .set_filter(|search, person| person.name_fl().to_lowercase().contains(search));

        this.selector.set_items(
            db::get_recent_persons(&mut this.handle.backend.db().lock().unwrap()).unwrap(),
        );

        this
    }
}

impl Widget for RecordingSelector {
    fn get_widget(&self) -> gtk::Widget {
        self.selector.widget.clone().upcast()
    }
}

/// The work selector within the recording selector.
struct RecordingSelectorWorkScreen {
    handle: NavigationHandle<Work>,
    person: Person,
    selector: Rc<Selector<Work>>,
}

impl Screen<Person, Work> for RecordingSelectorWorkScreen {
    fn new(person: Person, handle: NavigationHandle<Work>) -> Rc<Self> {
        let selector = Selector::<Work>::new();
        selector.set_title(&gettext("Select work"));
        selector.set_subtitle(&person.name_fl());

        let this = Rc::new(Self {
            handle,
            person,
            selector,
        });

        this.selector.set_back_cb(clone!(@weak this =>  move || {
            this.handle.pop(None);
        }));

        this.selector.set_add_cb(clone!(@weak this =>  move || {
            spawn!(@clone this, async move {
                let work = Work::from_composer(this.person.clone());
                if let Some(work) = push!(this.handle, WorkEditor, Some(work)).await {
                    this.handle.pop(Some(work));
                }
            });
        }));

        this.selector
            .set_make_widget(clone!(@weak this =>  @default-panic, move |work| {
                let row = adw::ActionRow::builder()
                    .activatable(true)
                    .title(&work.title)
                    .build();

                let work = work.to_owned();
                row.connect_activated(clone!(@weak this =>  move |_| {
                    this.handle.pop(Some(work.clone()));
                }));

                row.upcast()
            }));

        this.selector
            .set_filter(|search, work| work.title.to_lowercase().contains(search));

        this.selector.set_items(
            db::get_works(
                &mut this.handle.backend.db().lock().unwrap(),
                &this.person.id,
            )
            .unwrap(),
        );

        this
    }
}

impl Widget for RecordingSelectorWorkScreen {
    fn get_widget(&self) -> gtk::Widget {
        self.selector.widget.clone().upcast()
    }
}

/// The actual recording selector within the recording selector.
struct RecordingSelectorRecordingScreen {
    handle: NavigationHandle<Recording>,
    work: Work,
    selector: Rc<Selector<Recording>>,
}

impl Screen<Work, Recording> for RecordingSelectorRecordingScreen {
    fn new(work: Work, handle: NavigationHandle<Recording>) -> Rc<Self> {
        let selector = Selector::<Recording>::new();
        selector.set_title(&gettext("Select recording"));
        selector.set_subtitle(&work.get_title());

        let this = Rc::new(Self {
            handle,
            work,
            selector,
        });

        this.selector.set_back_cb(clone!(@weak this =>  move || {
            this.handle.pop(None);
        }));

        this.selector.set_add_cb(clone!(@weak this =>  move || {
            spawn!(@clone this, async move {
                let recording = Recording::from_work(this.work.clone());
                if let Some(recording) = push!(this.handle, RecordingEditor, Some(recording)).await {
                    this.handle.pop(Some(recording));
                }
            });
        }));

        this.selector
            .set_make_widget(clone!(@weak this =>  @default-panic, move |recording| {
                let row = adw::ActionRow::builder()
                    .activatable(true)
                    .title(recording.get_performers())
                    .build();

                let recording = recording.to_owned();
                row.connect_activated(clone!(@weak this =>  move |_| {
                    if let Err(err) = db::update_recording(&mut this.handle.backend.db().lock().unwrap(), recording.clone()) {
                        warn!("Failed to update access time. {err}");
                    }

                    this.handle.pop(Some(recording.clone()));
                }));

                row.upcast()
            }));

        this.selector.set_filter(|search, recording| {
            recording.get_performers().to_lowercase().contains(search)
        });

        this.selector.set_items(
            db::get_recordings_for_work(
                &mut this.handle.backend.db().lock().unwrap(),
                &this.work.id,
            )
            .unwrap(),
        );

        this
    }
}

impl Widget for RecordingSelectorRecordingScreen {
    fn get_widget(&self) -> gtk::Widget {
        self.selector.widget.clone().upcast()
    }
}
