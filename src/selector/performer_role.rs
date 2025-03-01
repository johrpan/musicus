use std::cell::{OnceCell, RefCell};

use gettextrs::gettext;
use gtk::{
    glib::{self, subclass::Signal, Properties},
    pango,
    prelude::*,
    subclass::prelude::*,
};
use once_cell::sync::Lazy;

use crate::{
    db::models::{Instrument, Role},
    library::Library,
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

        pub roles: RefCell<Vec<Role>>,
        pub instruments: RefCell<Vec<Instrument>>,

        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub role_view: TemplateChild<adw::ToolbarView>,
        #[template_child]
        pub role_search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub role_scrolled_window: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub role_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub instrument_view: TemplateChild<adw::ToolbarView>,
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
                    obj.imp().stack.set_visible_child(&*obj.imp().role_view);
                    obj.imp().role_search_entry.set_text("");
                    obj.imp().role_search_entry.grab_focus();
                    obj.imp().role_scrolled_window.vadjustment().set_value(0.0);
                }
            });

            self.obj().search_roles("");
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("selected")
                        .param_types([Role::static_type(), Instrument::static_type()])
                        .build(),
                    Signal::builder("create-role").build(),
                    Signal::builder("create-instrument").build(),
                ]
            });

            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for PerformerRoleSelectorPopover {
        // TODO: Fix focus.
        fn focus(&self, direction_type: gtk::DirectionType) -> bool {
            if direction_type == gtk::DirectionType::Down {
                if self.stack.visible_child() == Some(self.role_list.get().upcast()) {
                    self.role_list.child_focus(direction_type)
                } else {
                    self.instrument_list.child_focus(direction_type)
                }
            } else {
                self.parent_focus(direction_type)
            }
        }
    }

    impl PopoverImpl for PerformerRoleSelectorPopover {}
}

glib::wrapper! {
    pub struct PerformerRoleSelectorPopover(ObjectSubclass<imp::PerformerRoleSelectorPopover>)
        @extends gtk::Widget, gtk::Popover;
}

#[gtk::template_callbacks]
impl PerformerRoleSelectorPopover {
    pub fn new(library: &Library) -> Self {
        glib::Object::builder().property("library", library).build()
    }

    pub fn connect_selected<F: Fn(&Self, Role, Option<Instrument>) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("selected", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let role = values[1].get::<Role>().unwrap();
            let instrument = values[2].get::<Option<Instrument>>().unwrap();
            f(&obj, role, instrument);
            None
        })
    }

    pub fn connect_create_role<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("create-role", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            f(&obj);
            None
        })
    }

    pub fn connect_create_instrument<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("create-instrument", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            f(&obj);
            None
        })
    }

    #[template_callback]
    fn role_search_changed(&self, entry: &gtk::SearchEntry) {
        self.search_roles(&entry.text());
    }

    #[template_callback]
    fn role_activate(&self, _: &gtk::SearchEntry) {
        if let Some(role) = self.imp().roles.borrow().first() {
            self.select_role(role.to_owned());
        } else {
            self.create_role();
        }
    }

    #[template_callback]
    fn back_button_clicked(&self) {
        self.imp().stack.set_visible_child(&*self.imp().role_view);
        self.imp().role_search_entry.grab_focus();
    }

    #[template_callback]
    fn instrument_search_changed(&self, entry: &gtk::SearchEntry) {
        self.search_instruments(&entry.text());
    }

    #[template_callback]
    fn instrument_activate(&self, _: &gtk::SearchEntry) {
        if let Some(instrument) = self.imp().instruments.borrow().first() {
            self.select_instrument(instrument.clone());
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

        for role in &roles {
            let row = ActivatableRow::new(
                &gtk::Label::builder()
                    .label(role.to_string())
                    .halign(gtk::Align::Start)
                    .ellipsize(pango::EllipsizeMode::Middle)
                    .build(),
            );

            row.set_tooltip_text(Some(&role.to_string()));

            let role = role.clone();
            let obj = self.clone();
            row.connect_activated(move |_: &ActivatableRow| {
                obj.select_role(role.clone());
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

        for instrument in &instruments {
            let row = ActivatableRow::new(
                &gtk::Label::builder()
                    .label(instrument.to_string())
                    .halign(gtk::Align::Start)
                    .ellipsize(pango::EllipsizeMode::Middle)
                    .build(),
            );

            row.set_tooltip_text(Some(&instrument.to_string()));

            let instrument = instrument.clone();
            let obj = self.clone();
            row.connect_activated(move |_: &ActivatableRow| {
                obj.select_instrument(instrument.clone());
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

    fn select_role(&self, role: Role) {
        if role == self.library().performer_default_role().unwrap() {
            self.imp().instrument_search_entry.set_text("");
            self.imp().instrument_search_entry.grab_focus();
            self.imp()
                .instrument_scrolled_window
                .vadjustment()
                .set_value(0.0);
            self.imp()
                .stack
                .set_visible_child(&*self.imp().instrument_view);

            self.search_instruments("");
        } else {
            self.emit_by_name::<()>("selected", &[&role, &None::<Instrument>]);
            self.popdown();
        }
    }

    fn select_instrument(&self, instrument: Instrument) {
        let role = self.library().performer_default_role().unwrap();
        self.emit_by_name::<()>("selected", &[&role, &instrument]);
        self.popdown();
    }

    fn create_role(&self) {
        self.emit_by_name::<()>("create-role", &[]);
        self.popdown();
    }

    fn create_instrument(&self) {
        self.emit_by_name::<()>("create-instrument", &[]);
        self.popdown();
    }
}
