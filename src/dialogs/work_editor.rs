use super::{InstrumentSelector, PersonSelector, PartEditor, SectionEditor};
use crate::backend::*;
use crate::database::*;
use crate::widgets::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use std::cell::RefCell;
use std::convert::TryInto;
use std::rc::Rc;

struct PartOrSection {
    part: Option<WorkPartDescription>,
    section: Option<WorkSectionDescription>,
}

impl PartOrSection {
    pub fn part(part: WorkPartDescription) -> Self {
        PartOrSection {
            part: Some(part),
            section: None,
        }
    }

    pub fn section(section: WorkSectionDescription) -> Self {
        PartOrSection {
            part: None,
            section: Some(section),
        }
    }

    pub fn is_part(&self) -> bool {
        self.part.is_some()
    }

    pub fn unwrap_part(&self) -> WorkPartDescription {
        self.part.as_ref().unwrap().clone()
    }

    pub fn unwrap_section(&self) -> WorkSectionDescription {
        self.section.as_ref().unwrap().clone()
    }

    pub fn get_title(&self) -> String {
        if self.is_part() {
            self.unwrap_part().title
        } else {
            self.unwrap_section().title
        }
    }
}

pub struct WorkEditor<F>
where
    F: Fn(WorkDescription) -> () + 'static, {
    backend: Rc<Backend>,
    window: gtk::Window,
    callback: F,
    save_button: gtk::Button,
    id: i64,
    title_entry: gtk::Entry,
    composer: RefCell<Option<Person>>,
    composer_label: gtk::Label,
    instruments: RefCell<Vec<Instrument>>,
    instrument_list: gtk::ListBox,
    structure: RefCell<Vec<PartOrSection>>,
    part_list: gtk::ListBox,
}

impl<F> WorkEditor<F>
where
    F: Fn(WorkDescription) -> () + 'static, {
    pub fn new<P: IsA<gtk::Window>>(
        backend: Rc<Backend>,
        parent: &P,
        work: Option<WorkDescription>,
        callback: F,
    ) -> Rc<Self> {
        let builder = gtk::Builder::from_resource("/de/johrpan/musicus_editor/ui/work_editor.ui");

        get_widget!(builder, gtk::Window, window);
        get_widget!(builder, gtk::Button, cancel_button);
        get_widget!(builder, gtk::Button, save_button);
        get_widget!(builder, gtk::Entry, title_entry);
        get_widget!(builder, gtk::Button, composer_button);
        get_widget!(builder, gtk::Label, composer_label);
        get_widget!(builder, gtk::ListBox, instrument_list);
        get_widget!(builder, gtk::Button, add_instrument_button);
        get_widget!(builder, gtk::Button, remove_instrument_button);
        get_widget!(builder, gtk::ListBox, part_list);
        get_widget!(builder, gtk::Button, add_part_button);
        get_widget!(builder, gtk::Button, remove_part_button);
        get_widget!(builder, gtk::Button, add_section_button);
        get_widget!(builder, gtk::Button, edit_part_button);
        get_widget!(builder, gtk::Button, move_part_up_button);
        get_widget!(builder, gtk::Button, move_part_down_button);

        let id = match work.clone() {
            Some(work) => {
                title_entry.set_text(&work.title);
                work.id
            }
            None => rand::random::<u32>().into(),
        };

        let composer = RefCell::new(match work.clone() {
            Some(work) => {
                composer_label.set_text(&work.composer.name_fl());
                save_button.set_sensitive(true);
                Some(work.composer)
            }
            None => None,
        });

        let instruments = RefCell::new(match work.clone() {
            Some(work) => work.instruments,
            None => Vec::new(),
        });

        let structure = RefCell::new(match work.clone() {
            Some(work) => {
                let mut result = Vec::new();

                for part in work.parts {
                    result.push(PartOrSection::part(part));
                }

                for section in work.sections {
                    result.insert(
                        section
                            .before_index
                            .try_into()
                            .expect("Section with unrealistic before_index!"),
                        PartOrSection::section(section),
                    );
                }

                result
            }
            None => Vec::new(),
        });

        let result = Rc::new(WorkEditor {
            backend: backend,
            window: window,
            callback: callback,
            save_button: save_button,
            id: id,
            title_entry: title_entry,
            composer: composer,
            composer_label: composer_label,
            instruments: instruments,
            instrument_list: instrument_list,
            structure: structure,
            part_list: part_list,
        });

        cancel_button.connect_clicked(clone!(@strong result => move |_| {
            result.window.close();
        }));

        result.save_button.connect_clicked(clone!(@strong result => move |_| {
            let mut section_count: i64 = 0;
            let mut parts: Vec<WorkPartDescription> = Vec::new();
            let mut sections: Vec<WorkSectionDescription> = Vec::new();

            for (index, pos) in result.structure.borrow().iter().enumerate() {
                if pos.is_part() {
                    parts.push(pos.unwrap_part());
                } else {
                    let mut section = pos.unwrap_section();
                    let index: i64 = index.try_into().unwrap();
                    section.before_index = index - section_count;
                    sections.push(section);
                    section_count += 1;
                }
            }

            let work = WorkDescription {
                id: result.id,
                title: result.title_entry.get_text().to_string(),
                composer: result.composer.borrow().clone().expect("Tried to create work without composer!"),
                instruments: result.instruments.borrow().to_vec(),
                parts: parts,
                sections: sections,
            };

            let c = glib::MainContext::default();
            let clone = result.clone();
            c.spawn_local(async move {
                clone.backend.update_work(work.clone().into()).await.unwrap();
                clone.window.close();
                (clone.callback)(work.clone());
            });
        }));

        composer_button.connect_clicked(clone!(@strong result => move |_| {
            PersonSelector::new(result.backend.clone(), &result.window, clone!(@strong result => move |person| {
                result.composer.replace(Some(person.clone()));
                result.composer_label.set_text(&person.name_fl());
                result.save_button.set_sensitive(true);
            })).show();
        }));

        add_instrument_button.connect_clicked(clone!(@strong result => move |_| {
            InstrumentSelector::new(result.backend.clone(), &result.window, clone!(@strong result => move |instrument| {
                {
                    let mut instruments = result.instruments.borrow_mut();
                    instruments.push(instrument);
                }
                
                result.show_instruments();
            })).show();
        }));

        remove_instrument_button.connect_clicked(clone!(@strong result => move |_| {
            let row = result.get_selected_instrument_row();
            match row {
                Some(row) => {
                    let index = row.get_index();
                    let index: usize = index.try_into().unwrap();
                    result.instruments.borrow_mut().remove(index);
                    result.show_instruments();
                }
                None => (),
            }
        }));

        add_part_button.connect_clicked(clone!(@strong result => move |_| {
            PartEditor::new(result.backend.clone(), &result.window, None, clone!(@strong result => move |part| {
                {
                    let mut structure = result.structure.borrow_mut();
                    structure.push(PartOrSection::part(part));
                }
                
                result.show_parts();
            })).show();
        }));

        add_section_button.connect_clicked(clone!(@strong result => move |_| {
            SectionEditor::new(&result.window, None, clone!(@strong result => move |section| {
                {
                    let mut structure = result.structure.borrow_mut();
                    structure.push(PartOrSection::section(section));
                }
                
                result.show_parts();
            })).show();
        }));

        edit_part_button.connect_clicked(clone!(@strong result => move |_| {
            let row = result.get_selected_part_row();
            match row {
                Some(row) => {
                    let index = row.get_index();
                    let index: usize = index.try_into().unwrap();
                    let pos = &result.structure.borrow()[index];
    
                    if pos.is_part() {
                        let editor =
                            PartEditor::new(result.backend.clone(), &result.window, Some(pos.unwrap_part()), clone!(@strong result => move |part| {
                                result.structure.borrow_mut()[index] = PartOrSection::part(part);
                                result.show_parts();
                            }));

                        editor.show();
                    } else {
                        let editor =
                            SectionEditor::new(&result.window, Some(pos.unwrap_section()), clone!(@strong result => move |section| {
                                result.structure.borrow_mut()[index] = PartOrSection::section(section);
                                result.show_parts();
                            }));

                        editor.show();
                    }
                }
                None => (),
            }
        }));

        remove_part_button.connect_clicked(clone!(@strong result => move |_| {
            let row = result.get_selected_part_row();
            match row {
                Some(row) => {
                    let index = row.get_index();
                    let index: usize = index.try_into().unwrap();
                    result.structure.borrow_mut().remove(index);
                    result.show_parts();
                }
                None => (),
            }
        }));

        move_part_up_button.connect_clicked(clone!(@strong result => move |_| {
            let row = result.get_selected_part_row();
            match row {
                Some(row) => {
                    let index = row.get_index();
                    if index > 0 {
                        let index: usize = index.try_into().unwrap();
                        result.structure.borrow_mut().swap(index - 1, index);
                        result.show_parts();
                    }
                }
                None => (),
            }
        }));

        move_part_down_button.connect_clicked(clone!(@strong result => move |_| {
            let row = result.get_selected_part_row();
            match row {
                Some(row) => {
                    let index = row.get_index();
                    let index: usize = index.try_into().unwrap();
                    if index < result.structure.borrow().len() - 1 {
                        result.structure.borrow_mut().swap(index, index + 1);
                        result.show_parts();
                    }
                }
                None => (),
            }
        }));

        result.window.set_transient_for(Some(parent));

        result.show_instruments();
        result.show_parts();

        result
    }

    pub fn show(&self) {
        self.window.show();
    }

    fn show_instruments(&self) {
        for child in self.instrument_list.get_children() {
            self.instrument_list.remove(&child);
        }

        for (index, instrument) in self.instruments.borrow().iter().enumerate() {
            let label = gtk::Label::new(Some(&instrument.name));
            label.set_halign(gtk::Align::Start);
            let row = SelectorRow::new(index.try_into().unwrap(), &label);
            row.show_all();
            self.instrument_list.insert(&row, -1);
        }
    }

    fn get_selected_instrument_row(&self) -> Option<SelectorRow> {
        match self.instrument_list.get_selected_rows().first() {
            Some(row) => match row.get_child() {
                Some(child) => Some(child.downcast().unwrap()),
                None => None,
            },
            None => None,
        }
    }

    fn show_parts(&self) {
        for child in self.part_list.get_children() {
            self.part_list.remove(&child);
        }

        for (index, part) in self.structure.borrow().iter().enumerate() {
            let label = gtk::Label::new(Some(&part.get_title()));
            label.set_halign(gtk::Align::Start);

            if part.is_part() {
                label.set_margin_start(6);
            } else {
                let attributes = pango::AttrList::new();
                attributes.insert(pango::Attribute::new_weight(pango::Weight::Bold).unwrap());
                label.set_attributes(Some(&attributes));
            }

            let row = SelectorRow::new(index.try_into().unwrap(), &label);
            row.show_all();
            self.part_list.insert(&row, -1);
        }
    }

    fn get_selected_part_row(&self) -> Option<SelectorRow> {
        match self.part_list.get_selected_rows().first() {
            Some(row) => match row.get_child() {
                Some(child) => Some(child.downcast().unwrap()),
                None => None,
            },
            None => None,
        }
    }
}
