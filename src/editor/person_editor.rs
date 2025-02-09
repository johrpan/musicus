use std::cell::OnceCell;

use adw::{prelude::*, subclass::prelude::*};
use gettextrs::gettext;
use gtk::glib::{self, subclass::Signal};
use once_cell::sync::Lazy;

use crate::{
    db::models::Person, editor::translation_editor::MusicusTranslationEditor,
    library::MusicusLibrary,
};

mod imp {

    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/person_editor.blp")]
    pub struct MusicusPersonEditor {
        pub navigation: OnceCell<adw::NavigationView>,
        pub library: OnceCell<MusicusLibrary>,
        pub person_id: OnceCell<String>,

        #[template_child]
        pub name_editor: TemplateChild<MusicusTranslationEditor>,
        #[template_child]
        pub save_button: TemplateChild<gtk::Button>,
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

    impl ObjectImpl for MusicusPersonEditor {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("created")
                    .param_types([Person::static_type()])
                    .build()]
            });

            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for MusicusPersonEditor {}
    impl NavigationPageImpl for MusicusPersonEditor {}
}

glib::wrapper! {
    pub struct MusicusPersonEditor(ObjectSubclass<imp::MusicusPersonEditor>)
        @extends gtk::Widget, adw::NavigationPage;
}

#[gtk::template_callbacks]
impl MusicusPersonEditor {
    pub fn new(
        navigation: &adw::NavigationView,
        library: &MusicusLibrary,
        person: Option<&Person>,
    ) -> Self {
        let obj: Self = glib::Object::new();

        obj.imp().navigation.set(navigation.to_owned()).unwrap();
        obj.imp().library.set(library.to_owned()).unwrap();

        if let Some(person) = person {
            obj.imp().save_button.set_label(&gettext("Save changes"));
            obj.imp().person_id.set(person.person_id.clone()).unwrap();
            obj.imp().name_editor.set_translation(&person.name);
        }

        obj
    }

    pub fn connect_created<F: Fn(&Self, Person) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("created", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let person = values[1].get::<Person>().unwrap();
            f(&obj, person);
            None
        })
    }

    #[template_callback]
    fn save(&self, _: &gtk::Button) {
        let library = self.imp().library.get().unwrap();
        let name = self.imp().name_editor.translation();

        if let Some(person_id) = self.imp().person_id.get() {
            library.update_person(person_id, name).unwrap();
        } else {
            let person = library.create_person(name).unwrap();
            self.emit_by_name::<()>("created", &[&person]);
        }

        self.imp().navigation.get().unwrap().pop();
    }
}
