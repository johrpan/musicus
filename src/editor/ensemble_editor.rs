use std::cell::OnceCell;

use adw::{prelude::*, subclass::prelude::*};
use gettextrs::gettext;
use gtk::glib::{self, subclass::Signal};
use once_cell::sync::Lazy;

use crate::{
    db::models::Ensemble, editor::translation_editor::MusicusTranslationEditor,
    library::MusicusLibrary,
};

mod imp {

    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/ensemble_editor.blp")]
    pub struct MusicusEnsembleEditor {
        pub navigation: OnceCell<adw::NavigationView>,
        pub library: OnceCell<MusicusLibrary>,
        pub ensemble_id: OnceCell<String>,

        #[template_child]
        pub name_editor: TemplateChild<MusicusTranslationEditor>,
        #[template_child]
        pub save_row: TemplateChild<adw::ButtonRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MusicusEnsembleEditor {
        const NAME: &'static str = "MusicusEnsembleEditor";
        type Type = super::MusicusEnsembleEditor;
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

    impl ObjectImpl for MusicusEnsembleEditor {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("created")
                    .param_types([Ensemble::static_type()])
                    .build()]
            });

            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for MusicusEnsembleEditor {}
    impl NavigationPageImpl for MusicusEnsembleEditor {}
}

glib::wrapper! {
    pub struct MusicusEnsembleEditor(ObjectSubclass<imp::MusicusEnsembleEditor>)
        @extends gtk::Widget, adw::NavigationPage;
}

#[gtk::template_callbacks]
impl MusicusEnsembleEditor {
    pub fn new(
        navigation: &adw::NavigationView,
        library: &MusicusLibrary,
        ensemble: Option<&Ensemble>,
    ) -> Self {
        let obj: Self = glib::Object::new();

        obj.imp().navigation.set(navigation.to_owned()).unwrap();
        obj.imp().library.set(library.to_owned()).unwrap();

        if let Some(ensemble) = ensemble {
            obj.imp().save_row.set_title(&gettext("_Save changes"));
            obj.imp()
                .ensemble_id
                .set(ensemble.ensemble_id.clone())
                .unwrap();
            obj.imp().name_editor.set_translation(&ensemble.name);
        }

        obj
    }

    pub fn connect_created<F: Fn(&Self, Ensemble) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("created", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let ensemble = values[1].get::<Ensemble>().unwrap();
            f(&obj, ensemble);
            None
        })
    }

    #[template_callback]
    fn save(&self) {
        let library = self.imp().library.get().unwrap();
        let name = self.imp().name_editor.translation();

        if let Some(ensemble_id) = self.imp().ensemble_id.get() {
            library.update_ensemble(ensemble_id, name).unwrap();
        } else {
            let ensemble = library.create_ensemble(name).unwrap();
            self.emit_by_name::<()>("created", &[&ensemble]);
        }

        self.imp().navigation.get().unwrap().pop();
    }
}
