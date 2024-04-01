use adw::prelude::WidgetExt;
use gtk::{glib, subclass::prelude::*};
use std::cell::OnceCell;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/program_tile.blp")]
    pub struct MusicusProgramTile {
        #[template_child]
        pub edit_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub title_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub description_label: TemplateChild<gtk::Label>,

        pub program: OnceCell<Program>,
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
        let obj: Self = glib::Object::new();
        let imp = obj.imp();

        if let Some(design) = program.design {
            obj.add_css_class("highlight");
            obj.add_css_class(match design {
                ProgramTileDesign::Program1 => "program1",
                ProgramTileDesign::Program2 => "program2",
                ProgramTileDesign::Program3 => "program3",
                ProgramTileDesign::Program4 => "program4",
                ProgramTileDesign::Program5 => "program5",
                ProgramTileDesign::Program6 => "program6",
            })
        }
        
        imp.title_label.set_label(&program.title);
        imp.description_label.set_label(&program.description);
        imp.program.set(program).unwrap();

        obj
    }

    pub fn program(&self) -> &Program {
        self.imp().program.get().unwrap()
    }
}

#[derive(Debug, Default)]
pub struct Program {
    pub title: String,
    pub description: String,
    pub design: Option<ProgramTileDesign>,
}

#[derive(Clone, Copy, Debug)]
pub enum ProgramTileDesign {
    Program1,
    Program2,
    Program3,
    Program4,
    Program5,
    Program6,
}
