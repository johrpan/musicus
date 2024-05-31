use crate::{db::models::Role, library::MusicusLibrary};

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
    #[properties(wrapper_type = super::MusicusRoleSelectorPopover)]
    #[template(file = "data/ui/role_selector_popover.blp")]
    pub struct MusicusRoleSelectorPopover {
        #[property(get, construct_only)]
        pub library: OnceCell<MusicusLibrary>,

        pub roles: RefCell<Vec<Role>>,

        #[template_child]
        pub search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub scrolled_window: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub list_box: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MusicusRoleSelectorPopover {
        const NAME: &'static str = "MusicusRoleSelectorPopover";
        type Type = super::MusicusRoleSelectorPopover;
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
    impl ObjectImpl for MusicusRoleSelectorPopover {
        fn constructed(&self) {
            self.parent_constructed();

            self.obj()
                .connect_visible_notify(|obj: &super::MusicusRoleSelectorPopover| {
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
                vec![Signal::builder("role-selected")
                    .param_types([Role::static_type()])
                    .build()]
            });

            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for MusicusRoleSelectorPopover {
        // TODO: Fix focus.
        fn focus(&self, direction_type: gtk::DirectionType) -> bool {
            if direction_type == gtk::DirectionType::Down {
                self.list_box.child_focus(direction_type)
            } else {
                self.parent_focus(direction_type)
            }
        }
    }

    impl PopoverImpl for MusicusRoleSelectorPopover {}
}

glib::wrapper! {
    pub struct MusicusRoleSelectorPopover(ObjectSubclass<imp::MusicusRoleSelectorPopover>)
        @extends gtk::Widget, gtk::Popover;
}

#[gtk::template_callbacks]
impl MusicusRoleSelectorPopover {
    pub fn new(library: &MusicusLibrary) -> Self {
        glib::Object::builder().property("library", library).build()
    }

    pub fn connect_role_selected<F: Fn(&Self, Role) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("role-selected", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let role = values[1].get::<Role>().unwrap();
            f(&obj, role);
            None
        })
    }

    #[template_callback]
    fn search_changed(&self, entry: &gtk::SearchEntry) {
        self.search(&entry.text());
    }

    #[template_callback]
    fn activate(&self, _: &gtk::SearchEntry) {
        if let Some(role) = self.imp().roles.borrow().first() {
            self.select(role.clone());
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

        let roles = imp.library.get().unwrap().search_roles(search).unwrap();

        imp.list_box.remove_all();

        for role in &roles {
            let row = MusicusActivatableRow::new(
                &gtk::Label::builder()
                    .label(role.to_string())
                    .halign(gtk::Align::Start)
                    .build(),
            );

            let role = role.clone();
            let obj = self.clone();
            row.connect_activated(move |_: &MusicusActivatableRow| {
                obj.select(role.clone());
            });

            imp.list_box.append(&row);
        }

        let create_box = gtk::Box::builder().spacing(12).build();
        create_box.append(&gtk::Image::builder().icon_name("list-add-symbolic").build());
        create_box.append(
            &gtk::Label::builder()
                .label(gettext("Create new role"))
                .halign(gtk::Align::Start)
                .build(),
        );

        let create_row = MusicusActivatableRow::new(&create_box);
        let obj = self.clone();
        create_row.connect_activated(move |_: &MusicusActivatableRow| {
            obj.create();
        });

        imp.list_box.append(&create_row);

        imp.roles.replace(roles);
    }

    fn select(&self, role: Role) {
        self.emit_by_name::<()>("role-selected", &[&role]);
        self.popdown();
    }

    fn create(&self) {
        log::info!("Create role!");
        self.popdown();
    }
}
