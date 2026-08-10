//! The editor for entities that are just a translated name.
//!
//! They differ only in their title, the label on the save button,
//! and which pair of library calls they make.

use std::cell::OnceCell;

use adw::{prelude::*, subclass::prelude::*};
use anyhow::Result;
use gettextrs::gettext;
use gtk::glib::{self, subclass::Signal};
use musicus_library::db::{
    models::{Instrument, Person, Role},
    TranslatedString,
};
use once_cell::sync::Lazy;

use crate::{editor::translation::TranslationEditor, library::Library, util};

/// A type of entity that consists of a translated name and nothing else.
pub trait SimpleEntityKind: 'static {
    type Item: Clone + 'static;

    fn id(item: &Self::Item) -> &str;
    fn name(item: &Self::Item) -> &TranslatedString;
    fn enable_updates(item: &Self::Item) -> bool;

    fn create(
        library: &Library,
        name: TranslatedString,
        enable_updates: bool,
    ) -> Result<Self::Item>;

    fn update(
        library: &Library,
        id: &str,
        name: TranslatedString,
        enable_updates: bool,
    ) -> Result<()>;

    fn page_title() -> String;
    fn create_label() -> String;
}

/// The type-erased half of [`SimpleEntityKind`], so that the editor itself does
/// not have to be a generic `GObject`.
pub(crate) trait EntitySource {
    fn page_title(&self) -> String;
    fn create_label(&self) -> String;

    /// Create the entity and return it for the "created" signal.
    fn create(
        &self,
        library: &Library,
        name: TranslatedString,
        enable_updates: bool,
    ) -> Result<glib::BoxedAnyObject>;

    fn update(
        &self,
        library: &Library,
        id: &str,
        name: TranslatedString,
        enable_updates: bool,
    ) -> Result<()>;
}

struct KindSource<K: SimpleEntityKind>(std::marker::PhantomData<K>);

impl<K: SimpleEntityKind> EntitySource for KindSource<K> {
    fn page_title(&self) -> String {
        K::page_title()
    }

    fn create_label(&self) -> String {
        K::create_label()
    }

    fn create(
        &self,
        library: &Library,
        name: TranslatedString,
        enable_updates: bool,
    ) -> Result<glib::BoxedAnyObject> {
        Ok(glib::BoxedAnyObject::new(K::create(
            library,
            name,
            enable_updates,
        )?))
    }

    fn update(
        &self,
        library: &Library,
        id: &str,
        name: TranslatedString,
        enable_updates: bool,
    ) -> Result<()> {
        K::update(library, id, name, enable_updates)
    }
}

mod imp {
    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/editor/simple_entity.blp")]
    pub struct SimpleEntityEditor {
        pub navigation: OnceCell<adw::NavigationView>,
        pub library: OnceCell<Library>,
        pub entity_id: OnceCell<String>,
        pub(super) source: OnceCell<Box<dyn EntitySource>>,

        #[template_child]
        pub name_editor: TemplateChild<TranslationEditor>,
        #[template_child]
        pub enable_updates_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub save_row: TemplateChild<adw::ButtonRow>,
    }

    impl std::fmt::Debug for SimpleEntityEditor {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("SimpleEntityEditor").finish_non_exhaustive()
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SimpleEntityEditor {
        const NAME: &'static str = "MusicusSimpleEntityEditor";
        type Type = super::SimpleEntityEditor;
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

    impl ObjectImpl for SimpleEntityEditor {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("created")
                    .param_types([glib::BoxedAnyObject::static_type()])
                    .build()]
            });

            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for SimpleEntityEditor {}
    impl NavigationPageImpl for SimpleEntityEditor {}
}

glib::wrapper! {
    pub struct SimpleEntityEditor(ObjectSubclass<imp::SimpleEntityEditor>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

#[gtk::template_callbacks]
impl SimpleEntityEditor {
    fn new<K: SimpleEntityKind>(
        navigation: &adw::NavigationView,
        library: &Library,
        item: Option<&K::Item>,
    ) -> Self {
        let obj: Self = glib::Object::new();
        let imp = obj.imp();

        let source: Box<dyn EntitySource> = Box::new(KindSource::<K>(std::marker::PhantomData));

        obj.set_title(&source.page_title());
        imp.save_row.set_title(&source.create_label());

        let _ = imp.navigation.set(navigation.to_owned());
        let _ = imp.library.set(library.to_owned());
        let _ = imp.source.set(source);

        if let Some(item) = item {
            imp.save_row.set_title(&gettext("_Save changes"));
            let _ = imp.entity_id.set(K::id(item).to_owned());
            imp.name_editor.set_translation(K::name(item));
            imp.enable_updates_row.set_active(K::enable_updates(item));
        }

        obj
    }

    pub fn person(
        navigation: &adw::NavigationView,
        library: &Library,
        person: Option<&Person>,
    ) -> Self {
        Self::new::<PersonKind>(navigation, library, person)
    }

    pub fn instrument(
        navigation: &adw::NavigationView,
        library: &Library,
        instrument: Option<&Instrument>,
    ) -> Self {
        Self::new::<InstrumentKind>(navigation, library, instrument)
    }

    pub fn role(navigation: &adw::NavigationView, library: &Library, role: Option<&Role>) -> Self {
        Self::new::<RoleKind>(navigation, library, role)
    }

    pub fn set_name(&self, name: &str) {
        self.imp().name_editor.set_generic(name);
    }

    pub fn connect_created<T: Clone + 'static, F: Fn(&Self, T) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("created", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let item = values[1]
                .get::<glib::BoxedAnyObject>()
                .unwrap()
                .borrow::<T>()
                .clone();
            f(&obj, item);
            None
        })
    }

    #[template_callback]
    fn save(&self) {
        let imp = self.imp();
        let library = imp.library.get().expect("editor should have a library");
        let source = imp.source.get().expect("editor should have a source");

        let name = imp.name_editor.translation();
        let enable_updates = imp.enable_updates_row.is_active();

        if name.0.values().all(|value| value.trim().is_empty()) {
            self.report(gettext("Please enter a name."));
            return;
        }

        let result = match imp.entity_id.get() {
            Some(id) => source.update(library, id, name, enable_updates),
            None => source
                .create(library, name, enable_updates)
                .map(|item| self.emit_by_name::<()>("created", &[&item])),
        };

        match result {
            Err(err) => match util::find_toast_overlay(self) {
                Some(toast_overlay) => util::error_toast("Failed to save", err, &toast_overlay),
                None => log::error!("Failed to save: {err:?}"),
            },
            Ok(_) => {
                imp.navigation
                    .get()
                    .expect("editor should have a navigation view")
                    .pop();
            }
        }
    }

    fn report(&self, message: String) {
        match util::find_toast_overlay(self) {
            Some(toast_overlay) => toast_overlay.add_toast(adw::Toast::new(&message)),
            None => log::warn!("{message}"),
        }
    }
}

pub struct PersonKind;

impl SimpleEntityKind for PersonKind {
    type Item = Person;

    fn id(item: &Person) -> &str {
        &item.person_id
    }

    fn name(item: &Person) -> &TranslatedString {
        &item.name
    }

    fn enable_updates(item: &Person) -> bool {
        item.enable_updates
    }

    fn create(library: &Library, name: TranslatedString, enable_updates: bool) -> Result<Person> {
        library.create_person(name, enable_updates)
    }

    fn update(
        library: &Library,
        id: &str,
        name: TranslatedString,
        enable_updates: bool,
    ) -> Result<()> {
        library.update_person(id, name, enable_updates)
    }

    fn page_title() -> String {
        gettext("Person")
    }

    fn create_label() -> String {
        gettext("_Create person")
    }
}

pub struct InstrumentKind;

impl SimpleEntityKind for InstrumentKind {
    type Item = Instrument;

    fn id(item: &Instrument) -> &str {
        &item.instrument_id
    }

    fn name(item: &Instrument) -> &TranslatedString {
        &item.name
    }

    fn enable_updates(item: &Instrument) -> bool {
        item.enable_updates
    }

    fn create(
        library: &Library,
        name: TranslatedString,
        enable_updates: bool,
    ) -> Result<Instrument> {
        library.create_instrument(name, enable_updates)
    }

    fn update(
        library: &Library,
        id: &str,
        name: TranslatedString,
        enable_updates: bool,
    ) -> Result<()> {
        library.update_instrument(id, name, enable_updates)
    }

    fn page_title() -> String {
        gettext("Instrument")
    }

    fn create_label() -> String {
        gettext("_Create instrument")
    }
}

pub struct RoleKind;

impl SimpleEntityKind for RoleKind {
    type Item = Role;

    fn id(item: &Role) -> &str {
        &item.role_id
    }

    fn name(item: &Role) -> &TranslatedString {
        &item.name
    }

    fn enable_updates(item: &Role) -> bool {
        item.enable_updates
    }

    fn create(library: &Library, name: TranslatedString, enable_updates: bool) -> Result<Role> {
        library.create_role(name, enable_updates)
    }

    fn update(
        library: &Library,
        id: &str,
        name: TranslatedString,
        enable_updates: bool,
    ) -> Result<()> {
        library.update_role(id, name, enable_updates)
    }

    fn page_title() -> String {
        gettext("Role")
    }

    fn create_label() -> String {
        gettext("_Create role")
    }
}
