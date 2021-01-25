use super::work_part::WorkPartEditor;
use super::work_section::WorkSectionEditor;
use crate::backend::Backend;
use crate::database::*;
use crate::selectors::{InstrumentSelector, PersonSelector};
use crate::widgets::{List, Navigator, NavigatorScreen};
use anyhow::Result;
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use libhandy::prelude::*;
use std::cell::RefCell;
use std::convert::TryInto;
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
    widget: gtk::Stack,
    backend: Rc<Backend>,
    save_button: gtk::Button,
    title_entry: gtk::Entry,
    info_bar: gtk::InfoBar,
    composer_row: libhandy::ActionRow,
    upload_switch: gtk::Switch,
    instrument_list: Rc<List>,
    part_list: Rc<List>,
    id: String,
    composer: RefCell<Option<Person>>,
    instruments: RefCell<Vec<Instrument>>,
    structure: RefCell<Vec<PartOrSection>>,
    saved_cb: RefCell<Option<Box<dyn Fn(Work) -> ()>>>,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl WorkEditor {
    /// Create a new work editor widget and optionally initialize it.
    pub fn new(backend: Rc<Backend>, work: Option<Work>) -> Rc<Self> {
        // Create UI

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/work_editor.ui");

        get_widget!(builder, gtk::Stack, widget);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::Button, save_button);
        get_widget!(builder, gtk::InfoBar, info_bar);
        get_widget!(builder, gtk::Entry, title_entry);
        get_widget!(builder, gtk::Button, composer_button);
        get_widget!(builder, libhandy::ActionRow, composer_row);
        get_widget!(builder, gtk::Switch, upload_switch);
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
                    structure.insert(
                        section.before_index.try_into().unwrap(),
                        PartOrSection::Section(section),
                    );
                }

                (work.id, Some(work.composer), work.instruments, structure)
            }
            None => (generate_id(), None, Vec::new(), Vec::new()),
        };

        let this = Rc::new(Self {
            widget,
            backend,
            save_button,
            id,
            info_bar,
            title_entry,
            composer_row,
            upload_switch,
            instrument_list,
            part_list,
            composer: RefCell::new(composer),
            instruments: RefCell::new(instruments),
            structure: RefCell::new(structure),
            saved_cb: RefCell::new(None),
            navigator: RefCell::new(None),
        });

        // Connect signals and callbacks

        back_button.connect_clicked(clone!(@strong this => move |_| {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                navigator.pop();
            }
        }));

        this.save_button
            .connect_clicked(clone!(@strong this => move |_| {
                let context = glib::MainContext::default();
                let clone = this.clone();
                context.spawn_local(async move {
                    clone.widget.set_visible_child_name("loading");
                    match clone.clone().save().await {
                        Ok(_) => {
                            let navigator = clone.navigator.borrow().clone();
                            if let Some(navigator) = navigator {
                                navigator.pop();
                            }
                        }
                        Err(_) => {
                            clone.info_bar.set_revealed(true);
                            clone.widget.set_visible_child_name("content");
                        }
                    }

                });
            }));

        composer_button.connect_clicked(clone!(@strong this => move |_| {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                let selector = PersonSelector::new(this.backend.clone());

                selector.set_selected_cb(clone!(@strong this, @strong navigator => move |person| {
                    this.show_composer(person);
                    this.composer.replace(Some(person.clone()));
                    navigator.clone().pop();
                }));

                navigator.push(selector);
            }
        }));

        this.instrument_list.set_make_widget_cb(clone!(@strong this => move |index| {
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

            let row = libhandy::ActionRow::new();
            row.set_title(Some(&instrument.name));
            row.add_suffix(&delete_button);

            row.upcast()
        }));

        add_instrument_button.connect_clicked(clone!(@strong this => move |_| {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                let selector = InstrumentSelector::new(this.backend.clone());

                selector.set_selected_cb(clone!(@strong this, @strong navigator => move |instrument| {
                    let length = {
                        let mut instruments = this.instruments.borrow_mut();
                        instruments.push(instrument.clone());
                        instruments.len()
                    };

                    this.instrument_list.update(length);
                    navigator.clone().pop();
                }));

                navigator.push(selector);
            }
        }));

        this.part_list.set_make_widget_cb(clone!(@strong this => move |index| {
            let pos = &this.structure.borrow()[index];

            let delete_button = gtk::Button::from_icon_name(Some("user-trash-symbolic"));
            delete_button.set_valign(gtk::Align::Center);

            delete_button.connect_clicked(clone!(@strong this => move |_| {
                let length = {
                    let mut structure = this.structure.borrow_mut();
                    structure.remove(index);
                    structure.len()
                };

                this.part_list.update(length);
            }));

            let edit_button = gtk::Button::from_icon_name(Some("document-edit-symbolic"));
            edit_button.set_valign(gtk::Align::Center);

            edit_button.connect_clicked(clone!(@strong this => move |_| {
                let navigator = this.navigator.borrow().clone();
                if let Some(navigator) = navigator {
                    match this.structure.borrow()[index].clone() {
                        PartOrSection::Part(part) => {
                            let editor = WorkPartEditor::new(this.backend.clone(), Some(part));

                            editor.set_ready_cb(clone!(@strong this, @strong navigator => move |part| {
                                let length = {
                                    let mut structure = this.structure.borrow_mut();
                                    structure[index] = PartOrSection::Part(part);
                                    structure.len()
                                };

                                this.part_list.update(length);
                                navigator.clone().pop();
                            }));

                            navigator.push(editor);
                        }
                        PartOrSection::Section(section) => {
                            let editor = WorkSectionEditor::new(Some(section));

                            editor.set_ready_cb(clone!(@strong this, @strong navigator => move |section| {
                                let length = {
                                    let mut structure = this.structure.borrow_mut();
                                    structure[index] = PartOrSection::Section(section);
                                    structure.len()
                                };

                                this.part_list.update(length);
                                navigator.clone().pop();
                            }));

                            navigator.push(editor);
                        }
                    }
                }
            }));

            let row = libhandy::ActionRow::new();
            row.set_activatable(true);
            row.set_title(Some(&pos.get_title()));
            row.add_suffix(&delete_button);
            row.add_suffix(&edit_button);
            row.set_activatable_widget(Some(&edit_button));

            if let PartOrSection::Part(_) = pos {
                // TODO: Replace with better solution to differentiate parts and sections.
                row.set_margin_start(12);
            }

            row.upcast()
        }));

        this.part_list.set_move_cb(clone!(@strong this => move |old_index, new_index| {
            let length = {
                let mut structure = this.structure.borrow_mut();
                structure.swap(old_index, new_index);
                structure.len()
            };

            this.part_list.update(length);
        }));

        add_part_button.connect_clicked(clone!(@strong this => move |_| {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                let editor = WorkPartEditor::new(this.backend.clone(), None);

                editor.set_ready_cb(clone!(@strong this, @strong navigator => move |part| {
                    let length = {
                        let mut structure = this.structure.borrow_mut();
                        structure.push(PartOrSection::Part(part));
                        structure.len()
                    };

                    this.part_list.update(length);
                    navigator.clone().pop();
                }));

                navigator.push(editor);
            }
        }));

        add_section_button.connect_clicked(clone!(@strong this => move |_| {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                let editor = WorkSectionEditor::new(None);

                editor.set_ready_cb(clone!(@strong this, @strong navigator => move |section| {
                    let length = {
                        let mut structure = this.structure.borrow_mut();
                        structure.push(PartOrSection::Section(section));
                        structure.len()
                    };

                    this.part_list.update(length);
                    navigator.clone().pop();
                }));

                navigator.push(editor);
            }
        }));

        // Initialization

        if let Some(composer) = &*this.composer.borrow() {
            this.show_composer(composer);
        }

        this.instrument_list.update(this.instruments.borrow().len());
        this.part_list.update(this.structure.borrow().len());

        this
    }

    /// The closure to call when a work was created.
    pub fn set_saved_cb<F: Fn(Work) -> () + 'static>(&self, cb: F) {
        self.saved_cb.replace(Some(Box::new(cb)));
    }

    /// Update the UI according to person.
    fn show_composer(&self, person: &Person) {
        self.composer_row.set_title(Some(&gettext("Composer")));
        self.composer_row.set_subtitle(Some(&person.name_fl()));
        self.save_button.set_sensitive(true);
    }

    /// Save the work and possibly upload it to the server.
    async fn save(self: Rc<Self>) -> Result<()> {
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
            title: self.title_entry.get_text().unwrap().to_string(),
            composer: self
                .composer
                .borrow()
                .clone()
                .expect("Tried to create work without composer!"),
            instruments: self.instruments.borrow().clone(),
            parts: parts,
            sections: sections,
        };

        let upload = self.upload_switch.get_active();
        if upload {
            self.backend.post_work(&work).await?;
        }

        self.backend
            .db()
            .update_work(work.clone().into())
            .await
            .unwrap();

        self.backend.library_changed();

        if let Some(cb) = &*self.saved_cb.borrow() {
            cb(work.clone());
        }

        Ok(())
    }
}

impl NavigatorScreen for WorkEditor {
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
