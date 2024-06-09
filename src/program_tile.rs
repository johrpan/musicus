use std::cell::OnceCell;

use gtk::{
    glib::{self, Properties},
    prelude::*,
    subclass::prelude::*,
};

use crate::program::{Program, ProgramDesign};

mod imp {
    use super::*;

    #[derive(Properties, Debug, Default, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::MusicusProgramTile)]
    #[template(file = "data/ui/program_tile.blp")]
    pub struct MusicusProgramTile {
        #[property(get, construct_only)]
        pub program: OnceCell<Program>,

        #[template_child]
        pub edit_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub title_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub description_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MusicusProgramTile {
        const NAME: &'static str = "MusicusProgramTile";
        type Type = super::MusicusProgramTile;
        type ParentType = gtk::FlowBoxChild;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for MusicusProgramTile {}

    impl WidgetImpl for MusicusProgramTile {}
    impl FlowBoxChildImpl for MusicusProgramTile {}
}

glib::wrapper! {
    pub struct MusicusProgramTile(ObjectSubclass<imp::MusicusProgramTile>)
        @extends gtk::Widget, gtk::FlowBoxChild;
}

impl MusicusProgramTile {
    pub fn new(program: Program) -> Self {
        let obj: Self = glib::Object::builder()
            .property("program", &program)
            .build();

        let imp = obj.imp();

        if program.design() != ProgramDesign::Generic {
            obj.add_css_class("highlight");
            obj.add_css_class(match program.design() {
                ProgramDesign::Generic => "generic",
                ProgramDesign::Program1 => "program1",
                ProgramDesign::Program2 => "program2",
                ProgramDesign::Program3 => "program3",
                ProgramDesign::Program4 => "program4",
                ProgramDesign::Program5 => "program5",
                ProgramDesign::Program6 => "program6",
            })
        }

        if let Some(title) = program.title() {
            imp.title_label.set_label(&title);
        }

        if let Some(description) = program.description() {
            imp.description_label.set_label(&description);
        }

        obj
    }
}
