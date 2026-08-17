use std::cell::{OnceCell, RefCell};

use gettextrs::gettext;
use gtk::{
    glib::{self, subclass::Signal, Properties},
    prelude::*,
    subclass::prelude::*,
};
use once_cell::sync::Lazy;

use musicus_library::db::models::{Instrument, Role};

use super::connect_keynav;
use crate::{
    library::{Library, SearchItem},
    util::activatable_row::ActivatableRow,
};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, Properties)]
    #[properties(wrapper_type = super::PerformerRoleSelectorPopover)]
    #[template(file = "data/ui/selector/performer_role.blp")]
    pub struct PerformerRoleSelectorPopover {
        #[property(get, construct_only)]
        pub library: OnceCell<Library>,

        pub roles: RefCell<Vec<SearchItem<Role>>>,
        pub instruments: RefCell<Vec<SearchItem<Instrument>>>,

        #[template_child]
        pub stack: TemplateChild<adw::ViewStack>,
        #[template_child]
        pub role_search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub role_scrolled_window: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub role_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub instrument_search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub instrument_scrolled_window: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub instrument_list: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PerformerRoleSelectorPopover {
        const NAME: &'static str = "MusicusPerformerRoleSelectorPopover";
        type Type = super::PerformerRoleSelectorPopover;
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
    impl ObjectImpl for PerformerRoleSelectorPopover {
        fn constructed(&self) {
            self.parent_constructed();

            self.obj().connect_visible_notify(|obj| {
                if obj.is_visible() {
                    obj.imp().role_search_entry.set_text("");
                    obj.imp().role_scrolled_window.vadjustment().set_value(0.0);
                    obj.imp().instrument_search_entry.set_text("");
                    obj.imp()
                        .instrument_scrolled_window
                        .vadjustment()
                        .set_value(0.0);

                    if obj.imp().stack.visible_child_name().as_deref() == Some("role") {
                        obj.imp().role_search_entry.grab_focus();
                    } else {
                        obj.imp().instrument_search_entry.grab_focus();
                    }
                }
            });

            connect_keynav(&self.role_search_entry, &self.role_list);
            connect_keynav(&self.instrument_search_entry, &self.instrument_list);

            self.obj().search_roles("");
            self.obj().search_instruments("");
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("reset").build(),
                    Signal::builder("role-selected")
                        .param_types([glib::BoxedAnyObject::static_type()])
                        .build(),
                    Signal::builder("instrument-selected")
                        .param_types([glib::BoxedAnyObject::static_type()])
                        .build(),
                    Signal::builder("create-role")
                        .param_types([String::static_type()])
                        .build(),
                    Signal::builder("create-instrument")
                        .param_types([String::static_type()])
                        .build(),
                ]
            });

            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for PerformerRoleSelectorPopover {}
    impl PopoverImpl for PerformerRoleSelectorPopover {}
}

glib::wrapper! {
    pub struct PerformerRoleSelectorPopover(ObjectSubclass<imp::PerformerRoleSelectorPopover>)
        @extends gtk::Popover, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native, gtk::ShortcutManager;
}

#[gtk::template_callbacks]
impl PerformerRoleSelectorPopover {
    pub fn new(library: &Library) -> Self {
        glib::Object::builder().property("library", library).build()
    }

    pub fn connect_reset<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("reset", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            f(&obj);
            None
        })
    }

    pub fn connect_role_selected<F: Fn(&Self, Role) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("role-selected", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let role = values[1]
                .get::<glib::BoxedAnyObject>()
                .unwrap()
                .borrow::<Role>()
                .clone();
            f(&obj, role);
            None
        })
    }

    pub fn connect_instrument_selected<F: Fn(&Self, Instrument) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("instrument-selected", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let role = values[1]
                .get::<glib::BoxedAnyObject>()
                .unwrap()
                .borrow::<Instrument>()
                .clone();
            f(&obj, role);
            None
        })
    }

    pub fn connect_create_role<F: Fn(&Self, String) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("create-role", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let search = values[1].get::<String>().unwrap();
            f(&obj, search);
            None
        })
    }

    pub fn connect_create_instrument<F: Fn(&Self, String) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("create-instrument", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let search = values[1].get::<String>().unwrap();
            f(&obj, search);
            None
        })
    }

    #[template_callback]
    fn reset_button_clicked(&self) {
        self.emit_by_name::<()>("reset", &[]);
        self.popdown();
    }

    #[template_callback]
    fn role_search_changed(&self, entry: &gtk::SearchEntry) {
        self.search_roles(&entry.text());
    }

    #[template_callback]
    fn role_activate(&self, _: &gtk::SearchEntry) {
        if let Some(item) = self.imp().roles.borrow().first() {
            self.select_role(item.to_owned());
        } else {
            self.create_role();
        }
    }

    #[template_callback]
    fn instrument_search_changed(&self, entry: &gtk::SearchEntry) {
        self.search_instruments(&entry.text());
    }

    #[template_callback]
    fn instrument_activate(&self, _: &gtk::SearchEntry) {
        if let Some(item) = self.imp().instruments.borrow().first() {
            self.select_instrument(item.clone());
        } else {
            self.create_instrument();
        }
    }

    #[template_callback]
    fn stop_search(&self, _: &gtk::SearchEntry) {
        self.popdown();
    }

    fn search_roles(&self, search: &str) {
        let imp = self.imp();

        let roles = imp.library.get().unwrap().search_roles(search).unwrap();

        imp.role_list.remove_all();

        for result in &roles {
            let text = result.item.to_string();
            let row = ActivatableRow::new(&super::item_row_child(&text, result.in_library, 0));

            let item = result.clone();
            let obj = self.clone();
            row.connect_activated(move |_: &ActivatableRow| {
                obj.select_role(item.clone());
            });

            imp.role_list.append(&row);
        }

        let create_box = gtk::Box::builder().spacing(12).build();
        create_box.append(&gtk::Image::builder().icon_name("list-add-symbolic").build());
        create_box.append(
            &gtk::Label::builder()
                .label(gettext("Create new role"))
                .halign(gtk::Align::Start)
                .build(),
        );

        let create_row = ActivatableRow::new(&create_box);
        let obj = self.clone();
        create_row.connect_activated(move |_: &ActivatableRow| {
            obj.create_role();
        });

        imp.role_list.append(&create_row);

        imp.roles.replace(roles);
    }

    fn search_instruments(&self, search: &str) {
        let imp = self.imp();

        let instruments = imp
            .library
            .get()
            .unwrap()
            .search_instruments(search)
            .unwrap();

        imp.instrument_list.remove_all();

        for result in &instruments {
            let text = result.item.to_string();
            let row = ActivatableRow::new(&super::item_row_child(&text, result.in_library, 0));

            let item = result.clone();
            let obj = self.clone();
            row.connect_activated(move |_: &ActivatableRow| {
                obj.select_instrument(item.clone());
            });

            imp.instrument_list.append(&row);
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
            obj.create_instrument();
        });

        imp.instrument_list.append(&create_row);

        imp.instruments.replace(instruments);
    }

    fn select_role(&self, item: SearchItem<Role>) {
        let role = if item.in_library {
            item.item
        } else {
            match self
                .imp()
                .library
                .get()
                .unwrap()
                .import_metadata_role(&item.item.role_id)
            {
                Ok(role) => role,
                Err(err) => {
                    log::error!("Failed to import role from metadata database: {err:?}");
                    return;
                }
            }
        };

        self.emit_by_name::<()>("role-selected", &[&glib::BoxedAnyObject::new(role.clone())]);
        self.popdown();
    }

    fn select_instrument(&self, item: SearchItem<Instrument>) {
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

    fn create_role(&self) {
        let search = self.imp().role_search_entry.text().to_string();
        self.emit_by_name::<()>("create-role", &[&search]);
        self.popdown();
    }

    fn create_instrument(&self) {
        let search = self.imp().instrument_search_entry.text().to_string();
        self.emit_by_name::<()>("create-instrument", &[&search]);
        self.popdown();
    }
}
