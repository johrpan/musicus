use std::cell::OnceCell;

use adw::subclass::prelude::*;
use gtk::{glib, prelude::*};

use crate::{player::Player, program::Program};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/program_section.blp")]
    pub struct ProgramSection {
        pub player: OnceCell<Player>,

        #[template_child]
        pub card: TemplateChild<adw::Bin>,
        #[template_child]
        pub title_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub description_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProgramSection {
        const NAME: &'static str = "MusicusProgramSection";
        type Type = super::ProgramSection;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ProgramSection {}
    impl WidgetImpl for ProgramSection {}
    impl BinImpl for ProgramSection {}
}

glib::wrapper! {
    pub struct ProgramSection(ObjectSubclass<imp::ProgramSection>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

#[gtk::template_callbacks]
impl ProgramSection {
    pub fn new(player: &Player) -> Self {
        let obj: Self = glib::Object::new();
        obj.imp().player.set(player.to_owned()).unwrap();
        obj
    }

    pub fn set_program(&self, program: &Program) {
        let imp = self.imp();

        imp.card.set_css_classes(&[
            "program-section-tile",
            "card",
            &program.design().css_class(),
        ]);

        imp.title_label
            .set_label(&program.title().unwrap_or_default());

        let description = program.description().unwrap_or_default();
        imp.description_label.set_label(&description);
        imp.description_label.set_visible(!description.is_empty());
    }

    #[template_callback]
    fn cancel(&self) {
        self.imp().player.get().unwrap().cancel_program();
    }
}
