use super::work_part::WorkPartEditor;
use crate::navigator::{NavigationHandle, Screen};
use crate::selectors::{InstrumentSelector, PersonSelector};
use crate::widgets::{List, Widget};

use adw::prelude::*;
use anyhow::Result;
use gettextrs::gettext;
use glib::clone;
use gtk_macros::get_widget;
use musicus_backend::db::{self, generate_id, Instrument, Person, Work, WorkPart};
use std::cell::RefCell;
use std::rc::Rc;

/// A widget for editing and creating works.
pub struct WorkEditor {
    handle: NavigationHandle<Work>,
    widget: gtk::Stack,
    save_button: gtk::Button,
    title_row: adw::EntryRow,
    info_bar: gtk::InfoBar,
    composer_row: adw::ActionRow,
    instrument_list: Rc<List>,
    part_list: Rc<List>,
    id: String,
    composer: RefCell<Option<Person>>,
    instruments: RefCell<Vec<Instrument>>,
    parts: RefCell<Vec<WorkPart>>,
}

impl Screen<Option<Work>, Work> for WorkEditor {
    /// Create a new work editor widget and optionally initialize it.
    fn new(work: Option<Work>, handle: NavigationHandle<Work>) -> Rc<Self> {
        // Create UI

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/work_editor.ui");

        get_widget!(builder, gtk::Stack, widget);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::Button, save_button);
        get_widget!(builder, gtk::InfoBar, info_bar);
        get_widget!(builder, adw::EntryRow, title_row);
        get_widget!(builder, gtk::Button, composer_button);
        get_widget!(builder, adw::ActionRow, composer_row);
        get_widget!(builder, gtk::Frame, instrument_frame);
        get_widget!(builder, gtk::Button, add_instrument_button);
        get_widget!(builder, gtk::Frame, structure_frame);
        get_widget!(builder, gtk::Button, add_part_button);

        let instrument_list = List::new();
        instrument_frame.set_child(Some(&instrument_list.widget));

        let part_list = List::new();
        part_list.set_enable_dnd(true);
        structure_frame.set_child(Some(&part_list.widget));

        let (id, composer, instruments, structure) = match work {
            Some(work) => {
                title_row.set_text(&work.title);
                (work.id, Some(work.composer), work.instruments, work.parts)
            }
            None => (generate_id(), None, Vec::new(), Vec::new()),
        };

        let this = Rc::new(Self {
            handle,
            widget,
            save_button,
            id,
            info_bar,
            title_row,
            composer_row,
            instrument_list,
            part_list,
            composer: RefCell::new(composer),
            instruments: RefCell::new(instruments),
            parts: RefCell::new(structure),
        });

        // Connect signals and callbacks

        back_button.connect_clicked(clone!(@weak this =>  move |_| {
            this.handle.pop(None);
        }));

        this.save_button
            .connect_clicked(clone!(@weak this =>  move |_| {
                match this.save() {
                    Ok(work) => {
                        this.handle.pop(Some(work));
                    }
                    Err(_) => {
                        this.info_bar.set_revealed(true);
                        this.widget.set_visible_child_name("content");
                    }
                }
            }));

        composer_button.connect_clicked(clone!(@weak this =>  move |_| {
            spawn!(@clone this, async move {
                if let Some(person) = push!(this.handle, PersonSelector).await {
                    this.show_composer(&person);
                    this.composer.replace(Some(person));
                }
            });
        }));

        this.title_row
            .connect_changed(clone!(@weak this =>  move |_| this.validate()));

        this.instrument_list.set_make_widget_cb(
            clone!(@weak this =>  @default-panic, move |index| {
                let instrument = &this.instruments.borrow()[index];

                let delete_button = gtk::Button::from_icon_name("user-trash-symbolic");
                delete_button.set_valign(gtk::Align::Center);

                delete_button.connect_clicked(clone!(@strong this => move |_| {
                    let length = {
                        let mut instruments = this.instruments.borrow_mut();
                        instruments.remove(index);
                        instruments.len()
                    };

                    this.instrument_list.update(length);
                }));

                let row = adw::ActionRow::builder()
                    .title(&instrument.name)
                    .build();

                row.add_suffix(&delete_button);

                row.upcast()
            }),
        );

        add_instrument_button.connect_clicked(clone!(@weak this =>  move |_| {
            spawn!(@clone this, async move {
                if let Some(instrument) = push!(this.handle, InstrumentSelector).await {
                    let length = {
                        let mut instruments = this.instruments.borrow_mut();
                        instruments.push(instrument);
                        instruments.len()
                    };

                    this.instrument_list.update(length);
                }
            });
        }));

        this.part_list
            .set_make_widget_cb(clone!(@weak this => @default-panic,  move |index| {
                let part = &this.parts.borrow()[index];

                let delete_button = gtk::Button::from_icon_name("user-trash-symbolic");
                delete_button.set_valign(gtk::Align::Center);

                delete_button.connect_clicked(clone!(@weak this =>  move |_| {
                    let length = {
                        let mut structure = this.parts.borrow_mut();
                        structure.remove(index);
                        structure.len()
                    };

                    this.part_list.update(length);
                }));

                let edit_button = gtk::Button::from_icon_name("document-edit-symbolic");
                edit_button.set_valign(gtk::Align::Center);

                edit_button.connect_clicked(clone!(@weak this =>  move |_| {
                    spawn!(@clone this, async move {
                        let part = this.parts.borrow()[index].clone();
                        if let Some(part) = push!(this.handle, WorkPartEditor, Some(part)).await {
                            let length = {
                                let mut structure = this.parts.borrow_mut();
                                structure[index] = part;
                                structure.len()
                            };

                            this.part_list.update(length);
                        }
                    });
                }));

                let row = adw::ActionRow::builder()
                    .focusable(false)
                    .title(&part.title)
                    .activatable_widget(&edit_button)
                    .build();

                row.add_suffix(&delete_button);
                row.add_suffix(&edit_button);

                row.upcast()
            }));

        this.part_list
            .set_move_cb(clone!(@weak this =>  move |old_index, new_index| {
                let length = {
                    let mut parts = this.parts.borrow_mut();
                    parts.swap(old_index, new_index);
                    parts.len()
                };

                this.part_list.update(length);
            }));

        add_part_button.connect_clicked(clone!(@weak this =>  move |_| {
            spawn!(@clone this, async move {
                if let Some(part) = push!(this.handle, WorkPartEditor, None).await {
                    let length = {
                        let mut parts = this.parts.borrow_mut();
                        parts.push(part);
                        parts.len()
                    };

                    this.part_list.update(length);
                }
            });
        }));

        // Initialization

        if let Some(composer) = &*this.composer.borrow() {
            this.show_composer(composer);
        }

        this.instrument_list.update(this.instruments.borrow().len());
        this.part_list.update(this.parts.borrow().len());

        this
    }
}

impl WorkEditor {
    /// Update the UI according to person.
    fn show_composer(&self, person: &Person) {
        self.composer_row.set_title(&gettext("Composer"));
        self.composer_row.set_subtitle(&person.name_fl());
        self.validate();
    }

    /// Validate inputs and enable/disable saving.
    fn validate(&self) {
        self.save_button
            .set_sensitive(!self.title_row.text().is_empty() && self.composer.borrow().is_some());
    }

    /// Save the work.
    fn save(self: &Rc<Self>) -> Result<Work> {
        let work = Work::new(
            self.id.clone(),
            self.title_row.text().to_string(),
            self.composer
                .borrow()
                .clone()
                .expect("Tried to create work without composer!"),
            self.instruments.borrow().clone(),
            self.parts.borrow().clone(),
        );

        db::update_work(&mut self.handle.backend.db().lock().unwrap(), work.clone())?;
        self.handle.backend.library_changed();

        Ok(work)
    }
}

impl Widget for WorkEditor {
    fn get_widget(&self) -> gtk::Widget {
        self.widget.clone().upcast()
    }
}
