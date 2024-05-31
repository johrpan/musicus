use adw::{prelude::*, subclass::prelude::*};
use gtk::glib;

use crate::editor::translation_editor::MusicusTranslationEditor;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/person_editor.blp")]
    pub struct MusicusPersonEditor {}

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
    pub fn new() -> Self {
        glib::Object::new()
    }
}
