use super::work_part::WorkPartEditor;
use super::work_section::WorkSectionEditor;
use crate::navigator::{NavigationHandle, Screen};
use crate::selectors::{InstrumentSelector, PersonSelector};
use crate::widgets::{List, Widget};
use adw::prelude::*;
use anyhow::Result;
use gettextrs::gettext;
use glib::clone;
use gtk_macros::get_widget;
use musicus_backend::db::{generate_id, Instrument, Person, Work, WorkPart, WorkSection};
use std::cell::RefCell;
use std::rc::Rc;

/// Either a work part or a work section.
#[derive(Clone)]
enum PartOrSection {
    Part(WorkPart),
    Section(WorkSection),
}

impl PartOrSection {
    pub fn get_title(&self) -> String {
        match self {
            PartOrSection::Part(part) => part.title.clone(),
            PartOrSection::Section(section) => section.title.clone(),
        }
    }
}

/// A widget for editing and creating works.
pub struct WorkEditor {
    handle: NavigationHandle<Work>,
    widget: gtk::Stack,
    save_button: gtk::Button,
    title_entry: gtk::Entry,
    info_bar: gtk::InfoBar,
    composer_row: adw::ActionRow,
    instrument_list: Rc<List>,
    part_list: Rc<List>,
    id: String,
    composer: RefCell<Option<Person>>,
    instruments: RefCell<Vec<Instrument>>,
    structure: RefCell<Vec<PartOrSection>>,
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
        get_widget!(builder, gtk::Entry, title_entry);
        get_widget!(builder, gtk::Button, composer_button);
        get_widget!(builder, adw::ActionRow, composer_row);
        get_widget!(builder, gtk::Frame, instrument_frame);
        get_widget!(builder, gtk::Button, add_instrument_button);
        get_widget!(builder, gtk::Frame, structure_frame);
        get_widget!(builder, gtk::Button, add_part_button);
        get_widget!(builder, gtk::Button, add_section_button);

        let instrument_list = List::new();
        instrument_frame.set_child(Some(&instrument_list.widget));

        let part_list = List::new();
        part_list.set_enable_dnd(true);
        structure_frame.set_child(Some(&part_list.widget));

        let (id, composer, instruments, structure) = match work {
            Some(work) => {
                title_entry.set_text(&work.title);

                let mut structure = Vec::new();

                for part in work.parts {
                    structure.push(PartOrSection::Part(part));
                }

                for section in work.sections {
                    structure.insert(section.before_index, PartOrSection::Section(section));
                }

                (work.id, Some(work.composer), work.instruments, structure)
            }
            None => (generate_id(), None, Vec::new(), Vec::new()),
        };

        let this = Rc::new(Self {
            handle,
            widget,
            save_button,
            id,
            info_bar,
            title_entry,
            composer_row,
            instrument_list,
            part_list,
            composer: RefCell::new(composer),
            instruments: RefCell::new(instruments),
            structure: RefCell::new(structure),
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

        this.title_entry
            .connect_changed(clone!(@weak this =>  move |_| this.validate()));

        this.instrument_list.set_make_widget_cb(
            clone!(@weak this =>  @default-panic, move |index| {
                let instrument = &this.instruments.borrow()[index];

                let delete_button = gtk::Button::from_icon_name(Some("user-trash-symbolic"));
                delete_button.set_valign(gtk::Align::Center);

                delete_button.connect_clicked(clone!(@strong this => move |_| {
                    let length = {
                        let mut instruments = this.instruments.borrow_mut();
                        instruments.remove(index);
                        instruments.len()
                    };

                    this.instrument_list.update(length);
                }));

                let row = adw::ActionRowBuilder::new()
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

        this.part_list.set_make_widget_cb(clone!(@weak this => @default-panic,  move |index| {
            let pos = &this.structure.borrow()[index];

            let delete_button = gtk::Button::from_icon_name(Some("user-trash-symbolic"));
            delete_button.set_valign(gtk::Align::Center);

            delete_button.connect_clicked(clone!(@weak this =>  move |_| {
                let length = {
                    let mut structure = this.structure.borrow_mut();
                    structure.remove(index);
                    structure.len()
                };

                this.part_list.update(length);
            }));

            let edit_button = gtk::Button::from_icon_name(Some("document-edit-symbolic"));
            edit_button.set_valign(gtk::Align::Center);

            edit_button.connect_clicked(clone!(@weak this =>  move |_| {
                spawn!(@clone this, async move {
                    let part_or_section = this.structure.borrow()[index].clone();
                    match part_or_section {
                        PartOrSection::Part(part) => {
                            if let Some(part) = push!(this.handle, WorkPartEditor, Some(part)).await {
                                let length = {
                                    let mut structure = this.structure.borrow_mut();
                                    structure[index] = PartOrSection::Part(part);
                                    structure.len()
                                };

                                this.part_list.update(length);
                            }
                        }
                        PartOrSection::Section(section) => {
                            if let Some(section) = push!(this.handle, WorkSectionEditor, Some(section)).await {
                                let length = {
                                    let mut structure = this.structure.borrow_mut();
                                    structure[index] = PartOrSection::Section(section);
                                    structure.len()
                                };

                                this.part_list.update(length);
                            }
                        }
                    }
                });
            }));

            let row = adw::ActionRowBuilder::new()
                .focusable(false)
                .title(&pos.get_title())
                .activatable_widget(&edit_button)
                .build();

            row.add_suffix(&delete_button);
            row.add_suffix(&edit_button);

            if let PartOrSection::Part(_) = pos {
                // TODO: Replace with better solution to differentiate parts and sections.
                row.set_margin_start(12);
            }

            row.upcast()
        }));

        this.part_list
            .set_move_cb(clone!(@weak this =>  move |old_index, new_index| {
                let length = {
                    let mut structure = this.structure.borrow_mut();
                    structure.swap(old_index, new_index);
                    structure.len()
                };

                this.part_list.update(length);
            }));

        add_part_button.connect_clicked(clone!(@weak this =>  move |_| {
            spawn!(@clone this, async move {
                if let Some(part) = push!(this.handle, WorkPartEditor, None).await {
                    let length = {
                        let mut structure = this.structure.borrow_mut();
                        structure.push(PartOrSection::Part(part));
                        structure.len()
                    };

                    this.part_list.update(length);
                }
            });
        }));

        add_section_button.connect_clicked(clone!(@strong this => move |_| {
            spawn!(@clone this, async move {
                if let Some(section) = push!(this.handle, WorkSectionEditor, None).await {
                    let length = {
                        let mut structure = this.structure.borrow_mut();
                        structure.push(PartOrSection::Section(section));
                        structure.len()
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
        this.part_list.update(this.structure.borrow().len());

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
            .set_sensitive(!self.title_entry.text().is_empty() && self.composer.borrow().is_some());
    }

    /// Save the work.
    fn save(self: &Rc<Self>) -> Result<Work> {
        let mut section_count: usize = 0;
        let mut parts = Vec::new();
        let mut sections = Vec::new();

        for (index, pos) in self.structure.borrow().iter().enumerate() {
            match pos {
                PartOrSection::Part(part) => parts.push(part.clone()),
                PartOrSection::Section(section) => {
                    let mut section = section.clone();
                    section.before_index = index - section_count;
                    sections.push(section);
                    section_count += 1;
                }
            }
        }

        let work = Work {
            id: self.id.clone(),
            title: self.title_entry.text().to_string(),
            composer: self
                .composer
                .borrow()
                .clone()
                .expect("Tried to create work without composer!"),
            instruments: self.instruments.borrow().clone(),
            parts,
            sections,
        };

        self.handle.backend.db().update_work(work.clone())?;
        self.handle.backend.library_changed();

        Ok(work)
    }
}

impl Widget for WorkEditor {
    fn get_widget(&self) -> gtk::Widget {
        self.widget.clone().upcast()
    }
}
