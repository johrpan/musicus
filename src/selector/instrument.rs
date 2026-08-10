use std::cell::{OnceCell, RefCell};

use gettextrs::gettext;
use gtk::{
    glib::{self, subclass::Signal, Properties},
    prelude::*,
    subclass::prelude::*,
};
use once_cell::sync::Lazy;

use musicus_library::db::models::Instrument;

use crate::{
    library::{Library, SearchItem},
    util::activatable_row::ActivatableRow,
};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, Properties)]
    #[properties(wrapper_type = super::InstrumentSelectorPopover)]
    #[template(file = "data/ui/selector/instrument.blp")]
    pub struct InstrumentSelectorPopover {
        #[property(get, construct_only)]
        pub library: OnceCell<Library>,

        pub instruments: RefCell<Vec<SearchItem<Instrument>>>,

        #[template_child]
        pub search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub scrolled_window: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub list_box: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for InstrumentSelectorPopover {
        const NAME: &'static str = "MusicusInstrumentSelectorPopover";
        type Type = super::InstrumentSelectorPopover;
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
    impl ObjectImpl for InstrumentSelectorPopover {
        fn constructed(&self) {
            self.parent_constructed();

            self.obj().connect_visible_notify(|obj| {
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
                        .param_types([glib::BoxedAnyObject::static_type()])
                        .build(),
                    Signal::builder("create").build(),
                ]
            });

            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for InstrumentSelectorPopover {
        // TODO: Fix focus.
        fn focus(&self, direction_type: gtk::DirectionType) -> bool {
            if direction_type == gtk::DirectionType::Down {
                self.list_box.child_focus(direction_type)
            } else {
                self.parent_focus(direction_type)
            }
        }
    }

    impl PopoverImpl for InstrumentSelectorPopover {}
}

glib::wrapper! {
    pub struct InstrumentSelectorPopover(ObjectSubclass<imp::InstrumentSelectorPopover>)
        @extends gtk::Popover, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native, gtk::ShortcutManager;
}

#[gtk::template_callbacks]
impl InstrumentSelectorPopover {
    pub fn new(library: &Library) -> Self {
        glib::Object::builder().property("library", library).build()
    }

    pub fn connect_instrument_selected<F: Fn(&Self, Instrument) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("instrument-selected", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let instrument = values[1]
                .get::<glib::BoxedAnyObject>()
                .unwrap()
                .borrow::<Instrument>()
                .clone();
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
        if let Some(item) = self.imp().instruments.borrow().first() {
            self.select(item.clone());
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

        for result in &instruments {
            let text = result.item.to_string();
            let row = ActivatableRow::new(&super::item_row_child(&text, result.in_library));

            let item = result.clone();
            let obj = self.clone();
            row.connect_activated(move |_: &ActivatableRow| {
                obj.select(item.clone());
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

        let create_row = ActivatableRow::new(&create_box);
        let obj = self.clone();
        create_row.connect_activated(move |_: &ActivatableRow| {
            obj.create();
        });

        imp.list_box.append(&create_row);

        imp.instruments.replace(instruments);
    }

    fn select(&self, item: SearchItem<Instrument>) {
        let instrument = if item.in_library {
            item.item
        } else {
            match self
                .imp()
                .library
                .get()
                .unwrap()
                .import_metadata_instrument(&item.item.instrument_id)
            {
                Ok(instrument) => instrument,
                Err(err) => {
                    log::error!("Failed to import instrument from metadata database: {err:?}");
                    return;
                }
            }
        };

        self.emit_by_name::<()>(
            "instrument-selected",
            &[&glib::BoxedAnyObject::new(instrument.clone())],
        );
        self.popdown();
    }

    fn create(&self) {
        self.emit_by_name::<()>("create", &[]);
        self.popdown();
    }
}
