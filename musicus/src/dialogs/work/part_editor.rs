use crate::backend::*;
use crate::database::*;
use crate::dialogs::*;
use crate::widgets::*;
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use std::cell::RefCell;
use std::rc::Rc;

/// A dialog for creating or editing a work part.
pub struct PartEditor {
    backend: Rc<Backend>,
    window: libhandy::Window,
    title_entry: gtk::Entry,
    composer_label: gtk::Label,
    reset_composer_button: gtk::Button,
    instrument_list: Rc<List<Instrument>>,
    composer: RefCell<Option<Person>>,
    instruments: RefCell<Vec<Instrument>>,
    ready_cb: RefCell<Option<Box<dyn Fn(WorkPartDescription) -> ()>>>,
}

impl PartEditor {
    /// Create a new part editor and optionally initialize it.
    pub fn new<P: IsA<gtk::Window>>(
        backend: Rc<Backend>,
        parent: &P,
        part: Option<WorkPartDescription>,
    ) -> Rc<Self> {
        // Create UI

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/part_editor.ui");

        get_widget!(builder, libhandy::Window, window);
        get_widget!(builder, gtk::Button, cancel_button);
        get_widget!(builder, gtk::Button, save_button);
        get_widget!(builder, gtk::Entry, title_entry);
        get_widget!(builder, gtk::Button, composer_button);
        get_widget!(builder, gtk::Label, composer_label);
        get_widget!(builder, gtk::Button, reset_composer_button);
        get_widget!(builder, gtk::ScrolledWindow, scroll);
        get_widget!(builder, gtk::Button, add_instrument_button);
        get_widget!(builder, gtk::Button, remove_instrument_button);

        window.set_transient_for(Some(parent));

        let instrument_list = List::new(&gettext("No instruments added."));
        scroll.add(&instrument_list.widget);

        let (composer, instruments) = match part {
            Some(part) => {
                title_entry.set_text(&part.title);
                (part.composer, part.instruments)
            }
            None => (None, Vec::new()),
        };

        let this = Rc::new(Self {
            backend,
            window,
            title_entry,
            composer_label,
            reset_composer_button,
            instrument_list,
            composer: RefCell::new(composer),
            instruments: RefCell::new(instruments),
            ready_cb: RefCell::new(None),
        });

        // Connect signals and callbacks

        cancel_button.connect_clicked(clone!(@strong this => move |_| {
            this.window.close();
        }));

        save_button.connect_clicked(clone!(@strong this => move |_| {
            if let Some(cb) = &*this.ready_cb.borrow() {
                cb(WorkPartDescription {
                    title: this.title_entry.get_text().to_string(),
                    composer: this.composer.borrow().clone(),
                    instruments: this.instruments.borrow().clone(),
                });
            }

            this.window.close();
        }));

        composer_button.connect_clicked(clone!(@strong this => move |_| {
            PersonSelector::new(this.backend.clone(), &this.window, clone!(@strong this => move |person| {
                this.show_composer(Some(&person));
                this.composer.replace(Some(person));
            })).show();
        }));

        this.reset_composer_button
            .connect_clicked(clone!(@strong this => move |_| {
                this.composer.replace(None);
                this.show_composer(None);
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
            InstrumentSelector::new(this.backend.clone(), &this.window, clone!(@strong this => move |instrument| {
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

        // Initialize

        if let Some(composer) = &*this.composer.borrow() {
            this.show_composer(Some(composer));
        }

        this.instrument_list
            .show_items(this.instruments.borrow().clone());

        this
    }

    /// Set the closure to be called when the user wants to save the part.
    pub fn set_ready_cb<F: Fn(WorkPartDescription) -> () + 'static>(&self, cb: F) {
        self.ready_cb.replace(Some(Box::new(cb)));
    }

    /// Show the part editor.
    pub fn show(&self) {
        self.window.show();
    }

    /// Update the UI according to person.
    fn show_composer(&self, person: Option<&Person>) {
        if let Some(person) = person {
            self.composer_label.set_text(&person.name_fl());
            self.reset_composer_button.show();
        } else {
            self.composer_label.set_text(&gettext("Select …"));
            self.reset_composer_button.hide();
        }
    }
}
