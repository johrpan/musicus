use crate::{db::models::Instrument, library::MusicusLibrary};

use gettextrs::gettext;
use gtk::{
    glib::{self, subclass::Signal, Properties},
    prelude::*,
    subclass::prelude::*,
};
use once_cell::sync::Lazy;

use std::cell::{OnceCell, RefCell};

use super::activatable_row::MusicusActivatableRow;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, Properties)]
    #[properties(wrapper_type = super::MusicusInstrumentSelectorPopover)]
    #[template(file = "data/ui/instrument_selector_popover.blp")]
    pub struct MusicusInstrumentSelectorPopover {
        #[property(get, construct_only)]
        pub library: OnceCell<MusicusLibrary>,

        pub instruments: RefCell<Vec<Instrument>>,

        #[template_child]
        pub search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub scrolled_window: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub list_box: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MusicusInstrumentSelectorPopover {
        const NAME: &'static str = "MusicusInstrumentSelectorPopover";
        type Type = super::MusicusInstrumentSelectorPopover;
        type ParentType = gtk::Popover;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for MusicusInstrumentSelectorPopover {
        fn constructed(&self) {
            self.parent_constructed();

            self.obj()
                .connect_visible_notify(|obj: &super::MusicusInstrumentSelectorPopover| {
                    if obj.is_visible() {
                        obj.imp().search_entry.set_text("");
                        obj.imp().search_entry.grab_focus();
                        obj.imp().scrolled_window.vadjustment().set_value(0.0);
                    }
                });

            self.obj().search("");
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("instrument-selected")
                        .param_types([Instrument::static_type()])
                        .build(),
                    Signal::builder("create").build(),
                ]
            });

            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for MusicusInstrumentSelectorPopover {
        // TODO: Fix focus.
        fn focus(&self, direction_type: gtk::DirectionType) -> bool {
            if direction_type == gtk::DirectionType::Down {
                self.list_box.child_focus(direction_type)
            } else {
                self.parent_focus(direction_type)
            }
        }
    }

    impl PopoverImpl for MusicusInstrumentSelectorPopover {}
}

glib::wrapper! {
    pub struct MusicusInstrumentSelectorPopover(ObjectSubclass<imp::MusicusInstrumentSelectorPopover>)
        @extends gtk::Widget, gtk::Popover;
}

#[gtk::template_callbacks]
impl MusicusInstrumentSelectorPopover {
    pub fn new(library: &MusicusLibrary) -> Self {
        glib::Object::builder().property("library", library).build()
    }

    pub fn connect_instrument_selected<F: Fn(&Self, Instrument) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("instrument-selected", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let instrument = values[1].get::<Instrument>().unwrap();
            f(&obj, instrument);
            None
        })
    }

    pub fn connect_create<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("create", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            f(&obj);
            None
        })
    }

    #[template_callback]
    fn search_changed(&self, entry: &gtk::SearchEntry) {
        self.search(&entry.text());
    }

    #[template_callback]
    fn activate(&self, _: &gtk::SearchEntry) {
        if let Some(instrument) = self.imp().instruments.borrow().first() {
            self.select(instrument.clone());
        } else {
            self.create();
        }
    }

    #[template_callback]
    fn stop_search(&self, _: &gtk::SearchEntry) {
        self.popdown();
    }

    fn search(&self, search: &str) {
        let imp = self.imp();

        let instruments = imp
            .library
            .get()
            .unwrap()
            .search_instruments(search)
            .unwrap();

        imp.list_box.remove_all();

        for instrument in &instruments {
            let row = MusicusActivatableRow::new(
                &gtk::Label::builder()
                    .label(instrument.to_string())
                    .halign(gtk::Align::Start)
                    .build(),
            );

            row.set_tooltip_text(Some(&instrument.to_string()));

            let instrument = instrument.clone();
            let obj = self.clone();
            row.connect_activated(move |_: &MusicusActivatableRow| {
                obj.select(instrument.clone());
            });

            imp.list_box.append(&row);
        }

        let create_box = gtk::Box::builder().spacing(12).build();
        create_box.append(&gtk::Image::builder().icon_name("list-add-symbolic").build());
        create_box.append(
            &gtk::Label::builder()
                .label(gettext("Create new instrument"))
                .halign(gtk::Align::Start)
                .build(),
        );

        let create_row = MusicusActivatableRow::new(&create_box);
        let obj = self.clone();
        create_row.connect_activated(move |_: &MusicusActivatableRow| {
            obj.create();
        });

        imp.list_box.append(&create_row);

        imp.instruments.replace(instruments);
    }

    fn select(&self, instrument: Instrument) {
        self.emit_by_name::<()>("instrument-selected", &[&instrument]);
        self.popdown();
    }

    fn create(&self) {
        self.emit_by_name::<()>("create", &[]);
        self.popdown();
    }
}
