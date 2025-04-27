use std::cell::OnceCell;

use adw::{prelude::*, subclass::prelude::*};
use gettextrs::gettext;
use gtk::glib::{self, subclass::Signal};
use once_cell::sync::Lazy;

use crate::{db::models::Role, editor::translation::TranslationEditor, library::Library};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/editor/role.blp")]
    pub struct RoleEditor {
        pub navigation: OnceCell<adw::NavigationView>,
        pub library: OnceCell<Library>,
        pub role_id: OnceCell<String>,

        #[template_child]
        pub name_editor: TemplateChild<TranslationEditor>,
        #[template_child]
        pub enable_updates_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub save_row: TemplateChild<adw::ButtonRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RoleEditor {
        const NAME: &'static str = "MusicusRoleEditor";
        type Type = super::RoleEditor;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            TranslationEditor::static_type();
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for RoleEditor {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("created")
                    .param_types([Role::static_type()])
                    .build()]
            });

            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for RoleEditor {}
    impl NavigationPageImpl for RoleEditor {}
}

glib::wrapper! {
    pub struct RoleEditor(ObjectSubclass<imp::RoleEditor>)
        @extends gtk::Widget, adw::NavigationPage;
}

#[gtk::template_callbacks]
impl RoleEditor {
    pub fn new(navigation: &adw::NavigationView, library: &Library, role: Option<&Role>) -> Self {
        let obj: Self = glib::Object::new();

        obj.imp().navigation.set(navigation.to_owned()).unwrap();
        obj.imp().library.set(library.to_owned()).unwrap();

        if let Some(role) = role {
            obj.imp().save_row.set_title(&gettext("_Save changes"));
            obj.imp().role_id.set(role.role_id.clone()).unwrap();
            obj.imp().name_editor.set_translation(&role.name);
            obj.imp().enable_updates_row.set_active(role.enable_updates);
        }

        obj
    }

    pub fn connect_created<F: Fn(&Self, Role) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("created", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let role = values[1].get::<Role>().unwrap();
            f(&obj, role);
            None
        })
    }

    #[template_callback]
    fn save(&self) {
        let library = self.imp().library.get().unwrap();
        let name = self.imp().name_editor.translation();
        let enable_updates = self.imp().enable_updates_row.is_active();

        if let Some(role_id) = self.imp().role_id.get() {
            library.update_role(role_id, name, enable_updates).unwrap();
        } else {
            let role = library.create_role(name, enable_updates).unwrap();
            self.emit_by_name::<()>("created", &[&role]);
        }

        self.imp().navigation.get().unwrap().pop();
    }
}
