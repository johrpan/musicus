use std::cell::OnceCell;

use adw::{prelude::*, subclass::prelude::*};
use gettextrs::gettext;
use gtk::glib::{self, subclass::Signal};
use once_cell::sync::Lazy;

use crate::{db::models::Person, editor::translation::TranslationEditor, library::Library};

mod imp {

    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/editor/person.blp")]
    pub struct PersonEditor {
        pub navigation: OnceCell<adw::NavigationView>,
        pub library: OnceCell<Library>,
        pub person_id: OnceCell<String>,

        #[template_child]
        pub name_editor: TemplateChild<TranslationEditor>,
        #[template_child]
        pub save_row: TemplateChild<adw::ButtonRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PersonEditor {
        const NAME: &'static str = "MusicusPersonEditor";
        type Type = super::PersonEditor;
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

    impl ObjectImpl for PersonEditor {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("created")
                    .param_types([Person::static_type()])
                    .build()]
            });

            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for PersonEditor {}
    impl NavigationPageImpl for PersonEditor {}
}

glib::wrapper! {
    pub struct PersonEditor(ObjectSubclass<imp::PersonEditor>)
        @extends gtk::Widget, adw::NavigationPage;
}

#[gtk::template_callbacks]
impl PersonEditor {
    pub fn new(
        navigation: &adw::NavigationView,
        library: &Library,
        person: Option<&Person>,
    ) -> Self {
        let obj: Self = glib::Object::new();

        obj.imp().navigation.set(navigation.to_owned()).unwrap();
        obj.imp().library.set(library.to_owned()).unwrap();

        if let Some(person) = person {
            obj.imp().save_row.set_title(&gettext("_Save changes"));
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
    fn save(&self) {
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
