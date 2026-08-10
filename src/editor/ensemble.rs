mod member_row;

use std::cell::{OnceCell, RefCell};

use adw::{prelude::*, subclass::prelude::*};
use gettextrs::gettext;
use gtk::glib::{self, clone, subclass::Signal};
use member_row::EnsembleEditorMemberRow;
use once_cell::sync::Lazy;

use musicus_library::db::models::{Ensemble, Instrument, Person};

use crate::{
    editor::{simple_entity::SimpleEntityEditor, translation::TranslationEditor},
    library::Library,
    selector::SelectorPopover,
};

mod imp {

    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/editor/ensemble.blp")]
    pub struct EnsembleEditor {
        pub navigation: OnceCell<adw::NavigationView>,
        pub library: OnceCell<Library>,
        pub ensemble_id: OnceCell<String>,
        pub member_rows: RefCell<Vec<EnsembleEditorMemberRow>>,
        pub persons_popover: OnceCell<SelectorPopover>,

        #[template_child]
        pub name_editor: TemplateChild<TranslationEditor>,
        #[template_child]
        pub members_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub member_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub enable_updates_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub save_row: TemplateChild<adw::ButtonRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EnsembleEditor {
        const NAME: &'static str = "MusicusEnsembleEditor";
        type Type = super::EnsembleEditor;
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

    impl ObjectImpl for EnsembleEditor {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("created")
                    .param_types([glib::BoxedAnyObject::static_type()])
                    .build()]
            });

            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for EnsembleEditor {}
    impl NavigationPageImpl for EnsembleEditor {}
}

glib::wrapper! {
    pub struct EnsembleEditor(ObjectSubclass<imp::EnsembleEditor>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

#[gtk::template_callbacks]
impl EnsembleEditor {
    pub fn new(
        navigation: &adw::NavigationView,
        library: &Library,
        ensemble: Option<&Ensemble>,
    ) -> Self {
        let obj: Self = glib::Object::new();

        obj.imp().navigation.set(navigation.to_owned()).unwrap();
        obj.imp().library.set(library.to_owned()).unwrap();

        let persons_popover = SelectorPopover::persons(library);

        let this = obj.clone();
        persons_popover.connect_selected(move |_, person: Person| {
            this.new_member(person);
        });

        let this = obj.clone();
        persons_popover.connect_create(move |_| {
            let editor = SimpleEntityEditor::person(
                this.imp().navigation.get().unwrap(),
                this.imp().library.get().unwrap(),
                None,
            );

            editor.connect_created(clone!(
                #[weak]
                this,
                move |_, person| {
                    this.new_member(person);
                }
            ));

            this.imp().navigation.get().unwrap().push(&editor);
        });

        obj.imp().members_box.append(&persons_popover);
        obj.imp().persons_popover.set(persons_popover).unwrap();

        if let Some(ensemble) = ensemble {
            obj.imp().save_row.set_title(&gettext("_Save changes"));
            obj.imp()
                .ensemble_id
                .set(ensemble.ensemble_id.clone())
                .unwrap();
            obj.imp().name_editor.set_translation(&ensemble.name);
            obj.imp()
                .enable_updates_row
                .set_active(ensemble.enable_updates);

            for (person, instrument) in ensemble.persons.clone() {
                obj.add_member_row(person, instrument);
            }
        }

        obj
    }

    pub fn connect_created<F: Fn(&Self, Ensemble) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("created", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let ensemble = values[1]
                .get::<glib::BoxedAnyObject>()
                .unwrap()
                .borrow::<Ensemble>()
                .clone();
            f(&obj, ensemble);
            None
        })
    }

    #[template_callback]
    fn select_person(&self) {
        self.imp().persons_popover.get().unwrap().popup();
    }

    fn new_member(&self, person: Person) {
        self.add_member_row(person, None);
    }

    fn add_member_row(&self, person: Person, instrument: Option<Instrument>) {
        let row = EnsembleEditorMemberRow::new(
            self.imp().navigation.get().unwrap(),
            self.imp().library.get().unwrap(),
            person,
            instrument,
        );

        row.connect_move(clone!(
            #[weak(rename_to = this)]
            self,
            move |target, source| {
                let mut member_rows = this.imp().member_rows.borrow_mut();
                if let Some(index) = member_rows.iter().position(|p| p == target) {
                    this.imp().member_list.remove(&source);
                    member_rows.retain(|p| p != &source);
                    this.imp().member_list.insert(&source, index as i32);
                    member_rows.insert(index, source);
                }
            }
        ));

        row.connect_remove(clone!(
            #[weak(rename_to = this)]
            self,
            move |row| {
                this.imp().member_list.remove(row);
                this.imp().member_rows.borrow_mut().retain(|c| c != row);
            }
        ));

        self.imp()
            .member_list
            .insert(&row, self.imp().member_rows.borrow().len() as i32);

        self.imp().member_rows.borrow_mut().push(row);
    }

    #[template_callback]
    fn save(&self) {
        let library = self.imp().library.get().unwrap();
        let name = self.imp().name_editor.translation();
        let enable_updates = self.imp().enable_updates_row.is_active();

        let persons = self
            .imp()
            .member_rows
            .borrow()
            .iter()
            .map(EnsembleEditorMemberRow::member)
            .collect::<Vec<(Person, Option<Instrument>)>>();

        if !crate::editor::require_name(self, &name) {
            return;
        }

        if let Some(ensemble_id) = self.imp().ensemble_id.get() {
            if crate::editor::handle_save(
                self,
                library.update_ensemble(ensemble_id, name, persons, enable_updates),
            )
            .is_none()
            {
                return;
            }
        } else {
            let Some(ensemble) = crate::editor::handle_save(
                self,
                library.create_ensemble(name, persons, enable_updates),
            ) else {
                return;
            };

            self.emit_by_name::<()>("created", &[&glib::BoxedAnyObject::new(ensemble.clone())]);
        }

        self.imp().navigation.get().unwrap().pop();
    }
}
