use std::{cell::OnceCell, str::FromStr};

use adw::{prelude::*, subclass::prelude::*};
use gtk::{
    gio,
    glib::{self, subclass::Signal},
};
use once_cell::sync::Lazy;

use crate::{
    editor::program_settings::ProgramSettings,
    program::{Program, ProgramDesign},
};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/editor/default_program.blp")]
    pub struct DefaultProgramEditor {
        pub navigation: OnceCell<adw::NavigationView>,
        pub action_group: OnceCell<gio::SimpleActionGroup>,

        #[template_child]
        pub title_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub description_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub settings: TemplateChild<ProgramSettings>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DefaultProgramEditor {
        const NAME: &'static str = "MusicusDefaultProgramEditor";
        type Type = super::DefaultProgramEditor;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            ProgramSettings::static_type();
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for DefaultProgramEditor {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("save")
                    .param_types([Program::static_type()])
                    .build()]
            });

            SIGNALS.as_ref()
        }

        fn constructed(&self) {
            self.parent_constructed();

            let set_design_action = gio::ActionEntry::builder("set-design")
                .parameter_type(Some(glib::VariantTy::STRING))
                .state(glib::Variant::from("program-1"))
                .build();

            let actions = gio::SimpleActionGroup::new();
            actions.add_action_entries([set_design_action]);
            self.obj().insert_action_group("program", Some(&actions));
            self.action_group.set(actions).unwrap();
        }
    }

    impl WidgetImpl for DefaultProgramEditor {}
    impl NavigationPageImpl for DefaultProgramEditor {}
}

glib::wrapper! {
    pub struct DefaultProgramEditor(ObjectSubclass<imp::DefaultProgramEditor>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

#[gtk::template_callbacks]
impl DefaultProgramEditor {
    pub fn new(navigation: &adw::NavigationView, program: Option<&Program>) -> Self {
        let obj: Self = glib::Object::new();

        if let Some(program) = program {
            if let Some(title) = program.title() {
                obj.imp().title_row.set_text(&title);
            }

            if let Some(description) = program.description() {
                obj.imp().description_row.set_text(&description);
            }

            if let Err(err) = obj.activate_action(
                "program.set-design",
                Some(&glib::Variant::from(&program.design().to_string())),
            ) {
                log::warn!("Failed to initialize program design buttons: {err:?}");
            }

            obj.imp().settings.load(program);
        }

        obj.imp().navigation.set(navigation.to_owned()).unwrap();
        obj
    }

    pub fn connect_save<F: Fn(&Self, Program) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("save", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let program = values[1].get::<Program>().unwrap();
            f(&obj, program);
            None
        })
    }

    #[template_callback]
    fn save(&self) {
        let program = Program::new(
            &self.imp().title_row.text(),
            &self.imp().description_row.text(),
            ProgramDesign::from_str(
                &self
                    .imp()
                    .action_group
                    .get()
                    .unwrap()
                    .action_state("set-design")
                    .map(|v| v.get::<String>().unwrap_or_default())
                    .unwrap_or_default(),
            )
            .unwrap_or_default(),
        );

        self.imp().settings.apply(&program);

        self.emit_by_name::<()>("save", &[&program]);
        self.imp().navigation.get().unwrap().pop();
    }
}
