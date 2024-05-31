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
    #[template(file = "data/ui/translation_editor.blp")]
    pub struct MusicusTranslationEditor {
        #[template_child]
        pub list_box: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub entry_row: TemplateChild<adw::EntryRow>,

        pub translation_entries: RefCell<Vec<MusicusTranslationEntry>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MusicusTranslationEditor {
        const NAME: &'static str = "MusicusTranslationEditor";
        type Type = super::MusicusTranslationEditor;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for MusicusTranslationEditor {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for MusicusTranslationEditor {}
    impl BinImpl for MusicusTranslationEditor {}
}

glib::wrapper! {
    pub struct MusicusTranslationEditor(ObjectSubclass<imp::MusicusTranslationEditor>)
        @extends gtk::Widget, adw::PreferencesGroup;
}

#[gtk::template_callbacks]
impl MusicusTranslationEditor {
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
    fn add_translation(&self, _: &adw::ActionRow) {
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
            obj.imp().translation_entries.borrow_mut().retain(|e| e != entry);
            obj.imp().list_box.remove(entry);
        });

        self.imp().list_box.insert(&entry, self.imp().translation_entries.borrow().len() as i32 + 1);
        entry.grab_focus();

        self.imp().translation_entries.borrow_mut().push(entry);
    }
}
