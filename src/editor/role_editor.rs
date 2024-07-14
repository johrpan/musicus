use std::cell::OnceCell;

use adw::{prelude::*, subclass::prelude::*};
use gettextrs::gettext;
use gtk::glib::{self, subclass::Signal};
use once_cell::sync::Lazy;

use crate::{
    db::models::Role, editor::translation_editor::MusicusTranslationEditor, library::MusicusLibrary,
};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/role_editor.blp")]
    pub struct MusicusRoleEditor {
        pub navigation: OnceCell<adw::NavigationView>,
        pub library: OnceCell<MusicusLibrary>,
        pub role_id: OnceCell<String>,

        #[template_child]
        pub name_editor: TemplateChild<MusicusTranslationEditor>,
        #[template_child]
        pub save_button: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MusicusRoleEditor {
        const NAME: &'static str = "MusicusRoleEditor";
        type Type = super::MusicusRoleEditor;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            MusicusTranslationEditor::static_type();
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for MusicusRoleEditor {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("created")
                    .param_types([Role::static_type()])
                    .build()]
            });

            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for MusicusRoleEditor {}
    impl NavigationPageImpl for MusicusRoleEditor {}
}

glib::wrapper! {
    pub struct MusicusRoleEditor(ObjectSubclass<imp::MusicusRoleEditor>)
        @extends gtk::Widget, adw::NavigationPage;
}

#[gtk::template_callbacks]
impl MusicusRoleEditor {
    pub fn new(
        navigation: &adw::NavigationView,
        library: &MusicusLibrary,
        role: Option<&Role>,
    ) -> Self {
        let obj: Self = glib::Object::new();

        obj.imp().navigation.set(navigation.to_owned()).unwrap();
        obj.imp().library.set(library.to_owned()).unwrap();

        if let Some(role) = role {
            obj.imp().save_button.set_label(&gettext("Save changes"));
            obj.imp().role_id.set(role.role_id.clone()).unwrap();
            obj.imp().name_editor.set_translation(&role.name);
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
    fn save(&self, _: &gtk::Button) {
        let library = self.imp().library.get().unwrap();
        let name = self.imp().name_editor.translation();

        if let Some(role_id) = self.imp().role_id.get() {
            library.update_role(role_id, name).unwrap();
        } else {
            let role = library.create_role(name).unwrap();
            self.emit_by_name::<()>("created", &[&role]);
        }

        self.imp().navigation.get().unwrap().pop();
    }
}
