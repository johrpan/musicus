use std::cell::RefCell;
use std::collections::HashMap;

use adw::{prelude::*, subclass::prelude::*};
use gtk::glib;

use crate::db::TranslatedString;
use crate::editor::translation_entry::MusicusTranslationEntry;
use crate::util;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/translation_section.blp")]
    pub struct MusicusTranslationSection {
        #[template_child]
        pub entry_row: TemplateChild<adw::EntryRow>,

        pub translation_entries: RefCell<Vec<MusicusTranslationEntry>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MusicusTranslationSection {
        const NAME: &'static str = "MusicusTranslationSection";
        type Type = super::MusicusTranslationSection;
        type ParentType = adw::PreferencesGroup;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for MusicusTranslationSection {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for MusicusTranslationSection {}
    impl PreferencesGroupImpl for MusicusTranslationSection {}
}

glib::wrapper! {
    pub struct MusicusTranslationSection(ObjectSubclass<imp::MusicusTranslationSection>)
        @extends gtk::Widget, adw::PreferencesGroup;
}

#[gtk::template_callbacks]
impl MusicusTranslationSection {
    pub fn new(translation: TranslatedString) -> Self {
        let obj: Self = glib::Object::new();
        let mut translation = translation.0;

        obj.imp()
            .entry_row
            .set_text(&translation.remove("generic").unwrap_or_default());

        for (lang, translation) in translation {
            obj.add_entry(&lang, &translation);
        }

        obj
    }

    #[template_callback]
    fn add_translation(&self, _: &gtk::Button) {
        self.add_entry(&util::LANG, &self.imp().entry_row.text());
    }

    fn translation(&self) -> TranslatedString {
        let imp = self.imp();
        let mut translation = HashMap::<String, String>::new();

        translation.insert(String::from("generic"), imp.entry_row.text().into());
        for entry in &*imp.translation_entries.borrow() {
            translation.insert(entry.lang(), entry.translation());
        }

        TranslatedString(translation)
    }

    fn add_entry(&self, lang: &str, translation: &str) {
        let entry = MusicusTranslationEntry::new(lang, translation);

        let obj = self.clone();
        entry.connect_remove(move |entry| {
            let mut entries = obj.imp().translation_entries.borrow_mut();
            if let Some(index) = entries.iter().position(|e| e == entry) {
                entries.remove(index);
            }
            obj.remove(entry);
        });

        self.add(&entry);
        self.imp().translation_entries.borrow_mut().push(entry);
    }
}
