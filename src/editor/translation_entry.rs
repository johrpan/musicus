use adw::{prelude::*, subclass::prelude::*};
use gtk::glib::{self, subclass::Signal};
use once_cell::sync::Lazy;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/editor/translation_entry.blp")]
    pub struct TranslationEntry {
        #[template_child]
        pub lang_popover: TemplateChild<gtk::Popover>,
        #[template_child]
        pub lang_entry: TemplateChild<gtk::Entry>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TranslationEntry {
        const NAME: &'static str = "MusicusTranslationEntry";
        type Type = super::TranslationEntry;
        type ParentType = adw::EntryRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for TranslationEntry {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> =
                Lazy::new(|| vec![Signal::builder("remove").build()]);

            SIGNALS.as_ref()
        }

        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for TranslationEntry {}
    impl ListBoxRowImpl for TranslationEntry {}
    impl PreferencesRowImpl for TranslationEntry {}
    impl EntryRowImpl for TranslationEntry {}
    impl EditableImpl for TranslationEntry {}
}

glib::wrapper! {
    pub struct TranslationEntry(ObjectSubclass<imp::TranslationEntry>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::EntryRow,
        @implements gtk::Editable;
}

#[gtk::template_callbacks]
impl TranslationEntry {
    pub fn new(lang: &str, translation: &str) -> Self {
        let obj: Self = glib::Object::new();
        obj.set_text(translation);
        obj.imp().lang_entry.set_text(lang);
        obj
    }

    pub fn connect_remove<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("remove", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            f(&obj);
            None
        })
    }

    pub fn lang(&self) -> String {
        self.imp().lang_entry.text().into()
    }

    pub fn translation(&self) -> String {
        self.text().to_string()
    }

    #[template_callback]
    fn open_lang_popover(&self) {
        self.imp().lang_popover.popup();
        self.imp().lang_entry.grab_focus();
    }

    #[template_callback]
    fn remove(&self) {
        self.emit_by_name::<()>("remove", &[]);
    }
}
