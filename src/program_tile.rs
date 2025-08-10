use std::cell::{OnceCell, RefCell};

use gtk::{
    gio,
    glib::{self, clone, Properties},
    prelude::*,
    subclass::prelude::*,
};

use crate::{config, editor::program::ProgramEditor, program::Program};

mod imp {
    use super::*;

    #[derive(Properties, Debug, Default, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::ProgramTile)]
    #[template(file = "data/ui/program_tile.blp")]
    pub struct ProgramTile {
        #[property(get, construct_only)]
        pub navigation: OnceCell<adw::NavigationView>,
        #[property(get, construct_only)]
        pub key: OnceCell<String>,
        #[property(get, set = Self::set_program)]
        pub program: RefCell<Program>,

        #[template_child]
        pub edit_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub title_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub description_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProgramTile {
        const NAME: &'static str = "MusicusProgramTile";
        type Type = super::ProgramTile;
        type ParentType = gtk::FlowBoxChild;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for ProgramTile {
        fn constructed(&self) {
            self.parent_constructed();

            let settings = gio::Settings::new(config::APP_ID);
            self.set_program_from_settings(&settings);

            let obj = self.obj().to_owned();
            settings.connect_changed(Some(self.key.get().unwrap()), move |settings, _| {
                obj.imp().set_program_from_settings(settings);
            });
        }
    }

    impl WidgetImpl for ProgramTile {}
    impl FlowBoxChildImpl for ProgramTile {}

    impl ProgramTile {
        fn set_program_from_settings(&self, settings: &gio::Settings) {
            match Program::deserialize(&settings.string(self.key.get().unwrap())) {
                Ok(program) => self.set_program(&program),
                Err(err) => log::error!("Failed to deserialize program from settings: {err:?}"),
            }
        }

        fn set_program(&self, program: &Program) {
            self.obj().set_css_classes(&[
                "program-tile",
                "card",
                "activatable",
                &program.design().css_class(),
            ]);

            if let Some(title) = program.title() {
                self.title_label.set_label(&title);
            }

            if let Some(description) = program.description() {
                self.description_label.set_label(&description);
            }

            self.program.replace(program.to_owned());
        }
    }
}

glib::wrapper! {
    pub struct ProgramTile(ObjectSubclass<imp::ProgramTile>)
        @extends gtk::Widget, gtk::FlowBoxChild;
}

#[gtk::template_callbacks]
impl ProgramTile {
    pub fn new_for_setting(navigation: &adw::NavigationView, key: &str) -> Self {
        let obj: Self = glib::Object::builder()
            .property("navigation", navigation)
            .property("key", key)
            .build();

        obj
    }

    #[template_callback]
    fn edit_button_clicked(&self) {
        let editor = ProgramEditor::new(&self.navigation(), Some(&self.program()));

        editor.connect_save(clone!(
            #[weak(rename_to = obj)]
            self,
            move |_, program| {
                let settings = gio::Settings::new(config::APP_ID);
                if let Err(err) = settings.set_string(&obj.key(), &program.serialize()) {
                    log::error!("Failed to save program to settings: {err:?}");
                };
                obj.set_program(&program);
            }
        ));

        self.navigation().push(&editor);
    }
}
