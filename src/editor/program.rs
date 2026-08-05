use std::cell::OnceCell;

use adw::{prelude::*, subclass::prelude::*};
use gtk::glib;

use crate::{editor::program_settings::ProgramSettings, program::Program};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/editor/program.blp")]
    pub struct ProgramEditor {
        pub program: OnceCell<Program>,

        #[template_child]
        pub settings: TemplateChild<ProgramSettings>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProgramEditor {
        const NAME: &'static str = "MusicusProgramEditor";
        type Type = super::ProgramEditor;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            ProgramSettings::static_type();
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ProgramEditor {}
    impl WidgetImpl for ProgramEditor {}
    impl AdwDialogImpl for ProgramEditor {}
}

glib::wrapper! {
    /// A dialog for changing how the recordings for a program that is already playing are
    /// selected.
    ///
    /// Only the settings for random selection can be changed. Everything else, including the
    /// program's appearance and the items it is restricted to, stays the same.
    pub struct ProgramEditor(ObjectSubclass<imp::ProgramEditor>)
        @extends adw::Dialog, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

#[gtk::template_callbacks]
impl ProgramEditor {
    pub fn new(program: &Program) -> Self {
        let obj: Self = glib::Object::new();
        obj.imp().settings.load(program);
        obj.imp().program.set(program.to_owned()).unwrap();
        obj
    }

    #[template_callback]
    fn apply(&self) {
        let imp = self.imp();
        imp.settings.apply(imp.program.get().unwrap());
        self.close();
    }
}
