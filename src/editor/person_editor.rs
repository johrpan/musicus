use adw::{prelude::*, subclass::prelude::*};
use gtk::glib;

use crate::{db::models::Person, editor::translation_editor::MusicusTranslationEditor};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/person_editor.blp")]
    pub struct MusicusPersonEditor {
        #[template_child]
        pub name_editor: TemplateChild<MusicusTranslationEditor>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MusicusPersonEditor {
        const NAME: &'static str = "MusicusPersonEditor";
        type Type = super::MusicusPersonEditor;
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

    impl ObjectImpl for MusicusPersonEditor {}
    impl WidgetImpl for MusicusPersonEditor {}
    impl NavigationPageImpl for MusicusPersonEditor {}
}

glib::wrapper! {
    pub struct MusicusPersonEditor(ObjectSubclass<imp::MusicusPersonEditor>)
        @extends gtk::Widget, adw::NavigationPage;
}

#[gtk::template_callbacks]
impl MusicusPersonEditor {
    pub fn new(person: Option<&Person>) -> Self {
        let obj: Self = glib::Object::new();

        if let Some(person) = person {
            obj.imp().name_editor.set_translation(&person.name);
        }

        obj
    }
}
