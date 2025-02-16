use std::cell::OnceCell;

use adw::{prelude::*, subclass::prelude::*};
use gettextrs::gettext;
use gtk::glib::{self, subclass::Signal};
use once_cell::sync::Lazy;

use crate::{
    db::models::Instrument, editor::translation_editor::MusicusTranslationEditor,
    library::MusicusLibrary,
};

mod imp {

    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/instrument_editor.blp")]
    pub struct MusicusInstrumentEditor {
        pub navigation: OnceCell<adw::NavigationView>,
        pub library: OnceCell<MusicusLibrary>,
        pub instrument_id: OnceCell<String>,

        #[template_child]
        pub name_editor: TemplateChild<MusicusTranslationEditor>,
        #[template_child]
        pub save_row: TemplateChild<adw::ButtonRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MusicusInstrumentEditor {
        const NAME: &'static str = "MusicusInstrumentEditor";
        type Type = super::MusicusInstrumentEditor;
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

    impl ObjectImpl for MusicusInstrumentEditor {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("created")
                    .param_types([Instrument::static_type()])
                    .build()]
            });

            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for MusicusInstrumentEditor {}
    impl NavigationPageImpl for MusicusInstrumentEditor {}
}

glib::wrapper! {
    pub struct MusicusInstrumentEditor(ObjectSubclass<imp::MusicusInstrumentEditor>)
        @extends gtk::Widget, adw::NavigationPage;
}

#[gtk::template_callbacks]
impl MusicusInstrumentEditor {
    pub fn new(
        navigation: &adw::NavigationView,
        library: &MusicusLibrary,
        instrument: Option<&Instrument>,
    ) -> Self {
        let obj: Self = glib::Object::new();

        obj.imp().navigation.set(navigation.to_owned()).unwrap();
        obj.imp().library.set(library.to_owned()).unwrap();

        if let Some(instrument) = instrument {
            obj.imp().save_row.set_title(&gettext("Save changes"));
            obj.imp()
                .instrument_id
                .set(instrument.instrument_id.clone())
                .unwrap();
            obj.imp().name_editor.set_translation(&instrument.name);
        }

        obj
    }

    pub fn connect_created<F: Fn(&Self, Instrument) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("created", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let instrument = values[1].get::<Instrument>().unwrap();
            f(&obj, instrument);
            None
        })
    }

    #[template_callback]
    fn save(&self) {
        let library = self.imp().library.get().unwrap();
        let name = self.imp().name_editor.translation();

        if let Some(instrument_id) = self.imp().instrument_id.get() {
            library.update_instrument(instrument_id, name).unwrap();
        } else {
            let instrument = library.create_instrument(name).unwrap();
            self.emit_by_name::<()>("created", &[&instrument]);
        }

        self.imp().navigation.get().unwrap().pop();
    }
}
