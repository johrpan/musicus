use std::cell::OnceCell;

use adw::{prelude::*, subclass::prelude::*};
use gettextrs::gettext;
use gtk::glib::{self, subclass::Signal};
use musicus_library::db::models::Tag;
use once_cell::sync::Lazy;

use crate::{editor::translation::TranslationEditor, library::Library, util};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/editor/tag.blp")]
    pub struct TagEditor {
        pub navigation: OnceCell<adw::NavigationView>,
        pub library: OnceCell<Library>,
        pub tag_id: OnceCell<String>,

        #[template_child]
        pub name_editor: TemplateChild<TranslationEditor>,
        #[template_child]
        pub takes_value_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub enable_updates_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub save_row: TemplateChild<adw::ButtonRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TagEditor {
        const NAME: &'static str = "MusicusTagEditor";
        type Type = super::TagEditor;
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

    impl ObjectImpl for TagEditor {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("created")
                    .param_types([glib::BoxedAnyObject::static_type()])
                    .build()]
            });

            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for TagEditor {}
    impl NavigationPageImpl for TagEditor {}
}

glib::wrapper! {
    pub struct TagEditor(ObjectSubclass<imp::TagEditor>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

#[gtk::template_callbacks]
impl TagEditor {
    pub fn new(navigation: &adw::NavigationView, library: &Library, tag: Option<&Tag>) -> Self {
        let obj: Self = glib::Object::new();
        let imp = obj.imp();

        imp.save_row.set_title(&gettext("_Create tag"));

        let _ = imp.navigation.set(navigation.to_owned());
        let _ = imp.library.set(library.to_owned());

        if let Some(tag) = tag {
            imp.save_row.set_title(&gettext("_Save changes"));
            let _ = imp.tag_id.set(tag.tag_id.to_owned());
            imp.name_editor.set_translation(&tag.name);
            imp.takes_value_row.set_active(tag.takes_value);
            imp.enable_updates_row.set_active(tag.enable_updates);
        }

        obj
    }

    pub fn set_name(&self, name: &str) {
        self.imp().name_editor.set_generic(name);
    }

    pub fn connect_created<F: Fn(&Self, Tag) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("created", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let tag = values[1]
                .get::<glib::BoxedAnyObject>()
                .unwrap()
                .borrow::<Tag>()
                .clone();
            f(&obj, tag);
            None
        })
    }

    #[template_callback]
    fn save(&self) {
        let imp = self.imp();
        let library = imp.library.get().expect("editor should have a library");

        let name = imp.name_editor.translation();
        let takes_value = imp.takes_value_row.is_active();
        let enable_updates = imp.enable_updates_row.is_active();

        if name.0.values().all(|value| value.trim().is_empty()) {
            self.report(gettext("Please enter a name."));
            return;
        }

        let result = match imp.tag_id.get() {
            Some(id) => library.update_tag(id, name, takes_value, enable_updates),
            None => library
                .create_tag(name, takes_value, enable_updates)
                .map(|tag| self.emit_by_name::<()>("created", &[&glib::BoxedAnyObject::new(tag)])),
        };

        match result {
            Err(err) => match util::find_toast_overlay(self) {
                Some(toast_overlay) => util::error_toast("Failed to save", err, &toast_overlay),
                None => log::error!("Failed to save: {err:?}"),
            },
            Ok(_) => {
                imp.navigation
                    .get()
                    .expect("editor should have a navigation view")
                    .pop();
            }
        }
    }

    fn report(&self, message: String) {
        match util::find_toast_overlay(self) {
            Some(toast_overlay) => toast_overlay.add_toast(adw::Toast::new(&message)),
            None => log::warn!("{message}"),
        }
    }
}
