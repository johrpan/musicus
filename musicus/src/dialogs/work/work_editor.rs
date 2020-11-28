use super::part_editor::*;
use super::section_editor::*;
use crate::backend::*;
use crate::database::*;
use crate::dialogs::*;
use crate::widgets::*;
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
    pub widget: gtk::Box,
    backend: Rc<Backend>,
    parent: gtk::Window,
    save_button: gtk::Button,
    title_entry: gtk::Entry,
    composer_label: gtk::Label,
    instrument_list: Rc<List<Instrument>>,
    part_list: Rc<List<PartOrSection>>,
    id: String,
    composer: RefCell<Option<Person>>,
    instruments: RefCell<Vec<Instrument>>,
    structure: RefCell<Vec<PartOrSection>>,
    cancel_cb: RefCell<Option<Box<dyn Fn() -> ()>>>,
    saved_cb: RefCell<Option<Box<dyn Fn(Work) -> ()>>>,
}

impl WorkEditor {
    /// Create a new work editor widget and optionally initialize it. The parent window is used
    /// as the parent for newly created dialogs.
    pub fn new<P: IsA<gtk::Window>>(
        backend: Rc<Backend>,
        parent: &P,
        work: Option<Work>,
    ) -> Rc<Self> {
        // Create UI

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/work_editor.ui");

        get_widget!(builder, gtk::Box, widget);
        get_widget!(builder, gtk::Button, cancel_button);
        get_widget!(builder, gtk::Button, save_button);
        get_widget!(builder, gtk::Entry, title_entry);
        get_widget!(builder, gtk::Button, composer_button);
        get_widget!(builder, gtk::Label, composer_label);
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
            parent: parent.clone().upcast(),
            save_button,
            id,
            title_entry,
            composer_label,
            instrument_list,
            part_list,
            composer: RefCell::new(composer),
            instruments: RefCell::new(instruments),
            structure: RefCell::new(structure),
            cancel_cb: RefCell::new(None),
            saved_cb: RefCell::new(None),
        });

        // Connect signals and callbacks

        cancel_button.connect_clicked(clone!(@strong this => move |_| {
            if let Some(cb) = &*this.cancel_cb.borrow() {
                cb();
            }
        }));

        this.save_button.connect_clicked(clone!(@strong this => move |_| {
            let mut section_count: usize = 0;
            let mut parts = Vec::new();
            let mut sections = Vec::new();

            for (index, pos) in this.structure.borrow().iter().enumerate() {
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
                id: this.id.clone(),
                title: this.title_entry.get_text().to_string(),
                composer: this.composer.borrow().clone().expect("Tried to create work without composer!"),
                instruments: this.instruments.borrow().clone(),
                parts: parts,
                sections: sections,
            };

            let c = glib::MainContext::default();
            let clone = this.clone();
            c.spawn_local(async move {
                clone.backend.db().update_work(work.clone().into()).await.unwrap();
                if let Some(cb) = &*clone.saved_cb.borrow() {
                    cb(work);
                }
            });
        }));

        composer_button.connect_clicked(clone!(@strong this => move |_| {
            let dialog = PersonSelector::new(this.backend.clone(), &this.parent);

            dialog.set_selected_cb(clone!(@strong this => move |person| {
                this.show_composer(&person);
                this.composer.replace(Some(person));
            }));

            dialog.show();
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
            InstrumentSelector::new(this.backend.clone(), &this.parent, clone!(@strong this => move |instrument| {
                let mut instruments = this.instruments.borrow_mut();

                let index = match this.instrument_list.get_selected_index() {
                    Some(index) => index + 1,
                    None => instruments.len(),
                };

                instruments.insert(index, instrument);
                this.instrument_list.show_items(instruments.clone());
                this.instrument_list.select_index(index);
            })).show();
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
            let editor = PartEditor::new(this.backend.clone(), &this.parent, None);

            editor.set_ready_cb(clone!(@strong this => move |part| {
                let mut structure = this.structure.borrow_mut();

                let index = match this.part_list.get_selected_index() {
                    Some(index) => index + 1,
                    None => structure.len(),
                };

                structure.insert(index, PartOrSection::Part(part));
                this.part_list.show_items(structure.clone());
                this.part_list.select_index(index);
            }));

            editor.show();
        }));

        add_section_button.connect_clicked(clone!(@strong this => move |_| {
            let editor = SectionEditor::new(&this.parent, None);

            editor.set_ready_cb(clone!(@strong this => move |section| {
                let mut structure = this.structure.borrow_mut();

                let index = match this.part_list.get_selected_index() {
                    Some(index) => index + 1,
                    None => structure.len(),
                };

                structure.insert(index, PartOrSection::Section(section));
                this.part_list.show_items(structure.clone());
                this.part_list.select_index(index);
            }));

            editor.show();
        }));

        edit_part_button.connect_clicked(clone!(@strong this => move |_| {
            if let Some(index) = this.part_list.get_selected_index() {
                match this.structure.borrow()[index].clone() {
                    PartOrSection::Part(part) => {
                        let editor = PartEditor::new(
                            this.backend.clone(),
                            &this.parent,
                            Some(part),
                        );

                        editor.set_ready_cb(clone!(@strong this => move |part| {
                            let mut structure = this.structure.borrow_mut();
                            structure[index] = PartOrSection::Part(part);
                            this.part_list.show_items(structure.clone());
                            this.part_list.select_index(index);
                        }));

                        editor.show();
                    }
                    PartOrSection::Section(section) => {
                        let editor = SectionEditor::new(&this.parent, Some(section));

                        editor.set_ready_cb(clone!(@strong this => move |section| {
                            let mut structure = this.structure.borrow_mut();
                            structure[index] = PartOrSection::Section(section);
                            this.part_list.show_items(structure.clone());
                            this.part_list.select_index(index);
                        }));

                        editor.show();
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

    /// The closure to call when the editor is canceled.
    pub fn set_cancel_cb<F: Fn() -> () + 'static>(&self, cb: F) {
        self.cancel_cb.replace(Some(Box::new(cb)));
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
}
