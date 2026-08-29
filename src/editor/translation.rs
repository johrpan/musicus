use std::{cell::RefCell, collections::BTreeMap};

use adw::{prelude::*, subclass::prelude::*};
use gtk::glib;

use musicus_library::db::TranslatedString;

use crate::{editor::translation_entry::TranslationEntry, util};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/editor/translation.blp")]
    pub struct TranslationEditor {
        #[template_child]
        pub list_box: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub entry_row: TemplateChild<adw::EntryRow>,

        pub translation_entries: RefCell<Vec<TranslationEntry>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TranslationEditor {
        const NAME: &'static str = "MusicusTranslationEditor";
        type Type = super::TranslationEditor;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for TranslationEditor {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for TranslationEditor {}
    impl BinImpl for TranslationEditor {}
}

glib::wrapper! {
    pub struct TranslationEditor(ObjectSubclass<imp::TranslationEditor>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

#[gtk::template_callbacks]
impl TranslationEditor {
    pub fn new() -> Self {
        glib::Object::new()
    }

    /// The caption of the generic (default-locale) entry row.
    pub fn set_title(&self, title: &str) {
        self.imp().entry_row.set_title(title);
    }

    pub fn set_generic(&self, name: &str) {
        self.imp().entry_row.set_text(name);
    }

    pub fn set_translation(&self, translation: &TranslatedString) {
        self.imp()
            .entry_row
            .set_text(translation.generic_value().unwrap_or_default());

        for (lang, translation) in translation.iter().filter(|(lang, _)| *lang != "generic") {
            self.add_entry(lang, translation);
        }
    }

    #[template_callback]
    fn add_translation(&self) {
        self.add_entry(&util::LANG, &self.imp().entry_row.text());
    }

    pub fn translation(&self) -> Result<TranslatedString, musicus_library::LibraryError> {
        let imp = self.imp();
        let mut translation = BTreeMap::<String, String>::new();

        translation.insert(String::from("generic"), imp.entry_row.text().into());
        for entry in &*imp.translation_entries.borrow() {
            translation.insert(entry.lang(), entry.translation());
        }

        TranslatedString::try_from_map(translation)
    }

    fn add_entry(&self, lang: &str, translation: &str) {
        let entry = TranslationEntry::new(lang, translation);

        let obj = self.clone();
        entry.connect_remove(move |entry| {
            obj.imp()
                .translation_entries
                .borrow_mut()
                .retain(|e| e != entry);
            obj.imp().list_box.remove(entry);
        });

        self.imp().list_box.insert(
            &entry,
            self.imp().translation_entries.borrow().len() as i32 + 1,
        );
        entry.grab_focus();

        self.imp().translation_entries.borrow_mut().push(entry);
    }
}
