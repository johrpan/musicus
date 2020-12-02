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
use std::cell::RefCell;
use std::convert::TryInto;
use std::rc::Rc;

/// Either a work part or a work section.
#[derive(Clone)]
enum PartOrSection {
    Part(WorkPart),
    Section(WorkSection),
}

/// A widget for editing and creating works.
pub struct WorkEditor {
    widget: gtk::Stack,
    backend: Rc<Backend>,
    save_button: gtk::Button,
    title_entry: gtk::Entry,
    info_bar: gtk::InfoBar,
    composer_label: gtk::Label,
    upload_switch: gtk::Switch,
    instrument_list: Rc<List<Instrument>>,
    part_list: Rc<List<PartOrSection>>,
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
        get_widget!(builder, gtk::Label, composer_label);
        get_widget!(builder, gtk::Switch, upload_switch);
        get_widget!(builder, gtk::ScrolledWindow, instruments_scroll);
        get_widget!(builder, gtk::Button, add_instrument_button);
        get_widget!(builder, gtk::Button, remove_instrument_button);
        get_widget!(builder, gtk::ScrolledWindow, structure_scroll);
        get_widget!(builder, gtk::Button, add_part_button);
        get_widget!(builder, gtk::Button, remove_part_button);
        get_widget!(builder, gtk::Button, add_section_button);
        get_widget!(builder, gtk::Button, edit_part_button);
        get_widget!(builder, gtk::Button, move_part_up_button);
        get_widget!(builder, gtk::Button, move_part_down_button);

        let instrument_list = List::new(&gettext("No instruments added."));
        instruments_scroll.add(&instrument_list.widget);

        let part_list = List::new(&gettext("No work parts added."));
        structure_scroll.add(&part_list.widget);

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
            composer_label,
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

        this.instrument_list.set_make_widget(|instrument| {
            let label = gtk::Label::new(Some(&instrument.name));
            label.set_ellipsize(pango::EllipsizeMode::End);
            label.set_halign(gtk::Align::Start);
            label.set_margin_start(6);
            label.set_margin_end(6);
            label.set_margin_top(6);
            label.set_margin_bottom(6);
            label.upcast()
        });

        add_instrument_button.connect_clicked(clone!(@strong this => move |_| {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                let selector = InstrumentSelector::new(this.backend.clone());

                selector.set_selected_cb(clone!(@strong this, @strong navigator => move |instrument| {
                    let mut instruments = this.instruments.borrow_mut();

                    let index = match this.instrument_list.get_selected_index() {
                        Some(index) => index + 1,
                        None => instruments.len(),
                    };

                    instruments.insert(index, instrument.clone());
                    this.instrument_list.show_items(instruments.clone());
                    this.instrument_list.select_index(index);

                    navigator.clone().pop();
                }));

                navigator.push(selector);
            }
        }));

        remove_instrument_button.connect_clicked(clone!(@strong this => move |_| {
            if let Some(index) = this.instrument_list.get_selected_index() {
                let mut instruments = this.instruments.borrow_mut();
                instruments.remove(index);
                this.instrument_list.show_items(instruments.clone());
                this.instrument_list.select_index(index);
            }
        }));

        this.part_list.set_make_widget(|pos| {
            let label = gtk::Label::new(None);
            label.set_ellipsize(pango::EllipsizeMode::End);
            label.set_halign(gtk::Align::Start);
            label.set_margin_end(6);
            label.set_margin_top(6);
            label.set_margin_bottom(6);

            match pos {
                PartOrSection::Part(part) => {
                    label.set_text(&part.title);
                    label.set_margin_start(12);
                }
                PartOrSection::Section(section) => {
                    let attrs = pango::AttrList::new();
                    attrs.insert(pango::Attribute::new_weight(pango::Weight::Bold).unwrap());
                    label.set_attributes(Some(&attrs));
                    label.set_text(&section.title);
                    label.set_margin_start(6);
                }
            }

            label.upcast()
        });

        add_part_button.connect_clicked(clone!(@strong this => move |_| {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                let editor = WorkPartEditor::new(this.backend.clone(), None);

                editor.set_ready_cb(clone!(@strong this, @strong navigator => move |part| {
                    let mut structure = this.structure.borrow_mut();

                    let index = match this.part_list.get_selected_index() {
                        Some(index) => index + 1,
                        None => structure.len(),
                    };

                    structure.insert(index, PartOrSection::Part(part));
                    this.part_list.show_items(structure.clone());
                    this.part_list.select_index(index);

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
                    let mut structure = this.structure.borrow_mut();

                    let index = match this.part_list.get_selected_index() {
                        Some(index) => index + 1,
                        None => structure.len(),
                    };

                    structure.insert(index, PartOrSection::Section(section));
                    this.part_list.show_items(structure.clone());
                    this.part_list.select_index(index);

                    navigator.clone().pop();
                }));

                navigator.push(editor);
            }
        }));

        edit_part_button.connect_clicked(clone!(@strong this => move |_| {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                if let Some(index) = this.part_list.get_selected_index() {
                    match this.structure.borrow()[index].clone() {
                        PartOrSection::Part(part) => {
                            let editor = WorkPartEditor::new(this.backend.clone(), Some(part));

                            editor.set_ready_cb(clone!(@strong this, @strong navigator => move |part| {
                                let mut structure = this.structure.borrow_mut();
                                structure[index] = PartOrSection::Part(part);
                                this.part_list.show_items(structure.clone());
                                this.part_list.select_index(index);
                                navigator.clone().pop();
                            }));

                            navigator.push(editor);
                        }
                        PartOrSection::Section(section) => {
                            let editor = WorkSectionEditor::new(Some(section));

                            editor.set_ready_cb(clone!(@strong this, @strong navigator => move |section| {
                                let mut structure = this.structure.borrow_mut();
                                structure[index] = PartOrSection::Section(section);
                                this.part_list.show_items(structure.clone());
                                this.part_list.select_index(index);
                                navigator.clone().pop();
                            }));

                            navigator.push(editor);
                        }
                    }
                }
            }
        }));

        remove_part_button.connect_clicked(clone!(@strong this => move |_| {
            if let Some(index) = this.part_list.get_selected_index() {
                let mut structure = this.structure.borrow_mut();
                structure.remove(index);
                this.part_list.show_items(structure.clone());
                this.part_list.select_index(index);
            }
        }));

        move_part_up_button.connect_clicked(clone!(@strong this => move |_| {
            if let Some(index) = this.part_list.get_selected_index() {
                if index > 0 {
                    let mut structure = this.structure.borrow_mut();
                    structure.swap(index - 1, index);
                    this.part_list.show_items(structure.clone());
                    this.part_list.select_index(index - 1);
                }
            }
        }));

        move_part_down_button.connect_clicked(clone!(@strong this => move |_| {
            if let Some(index) = this.part_list.get_selected_index() {
                let mut structure = this.structure.borrow_mut();
                if index < structure.len() - 1 {
                    structure.swap(index, index + 1);
                    this.part_list.show_items(structure.clone());
                    this.part_list.select_index(index + 1);
                }
            }
        }));

        // Initialization

        if let Some(composer) = &*this.composer.borrow() {
            this.show_composer(composer);
        }

        this.instrument_list
            .show_items(this.instruments.borrow().clone());
        this.part_list.show_items(this.structure.borrow().clone());

        this
    }

    /// The closure to call when a work was created.
    pub fn set_saved_cb<F: Fn(Work) -> () + 'static>(&self, cb: F) {
        self.saved_cb.replace(Some(Box::new(cb)));
    }

    /// Update the UI according to person.
    fn show_composer(&self, person: &Person) {
        self.composer_label.set_text(&person.name_fl());
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
            title: self.title_entry.get_text().to_string(),
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
