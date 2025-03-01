use crate::{
    db::{
        self,
        models::{Composer, Instrument, Person, Work},
    },
    editor::{
        instrument_editor::MusicusInstrumentEditor,
        instrument_selector_popover::MusicusInstrumentSelectorPopover,
        person_editor::MusicusPersonEditor, person_selector_popover::MusicusPersonSelectorPopover,
        translation_editor::MusicusTranslationEditor,
        work_editor_composer_row::MusicusWorkEditorComposerRow,
        work_editor_part_row::MusicusWorkEditorPartRow,
    },
    library::MusicusLibrary,
};

use adw::{prelude::*, subclass::prelude::*};
use gettextrs::gettext;
use gtk::glib::{
    clone, Properties,
    {self, subclass::Signal},
};
use once_cell::sync::Lazy;

use std::cell::{Cell, OnceCell, RefCell};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, Properties)]
    #[properties(wrapper_type = super::MusicusWorkEditor)]
    #[template(file = "data/ui/work_editor.blp")]
    pub struct MusicusWorkEditor {
        #[property(get, construct_only)]
        pub navigation: OnceCell<adw::NavigationView>,

        #[property(get, construct_only)]
        pub library: OnceCell<MusicusLibrary>,

        pub work_id: OnceCell<String>,
        pub is_part_editor: Cell<bool>,

        // Holding a reference to each composer row is the simplest way to enumerate all
        // results when finishing the process of editing the work. The composer rows
        // handle all state related to the composer.
        pub composer_rows: RefCell<Vec<MusicusWorkEditorComposerRow>>,
        pub part_rows: RefCell<Vec<MusicusWorkEditorPartRow>>,

        pub instruments: RefCell<Vec<Instrument>>,

        pub persons_popover: OnceCell<MusicusPersonSelectorPopover>,
        pub instruments_popover: OnceCell<MusicusInstrumentSelectorPopover>,

        #[template_child]
        pub name_editor: TemplateChild<MusicusTranslationEditor>,
        #[template_child]
        pub composer_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub select_person_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub part_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub instrument_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub select_instrument_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub save_row: TemplateChild<adw::ButtonRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MusicusWorkEditor {
        const NAME: &'static str = "MusicusWorkEditor";
        type Type = super::MusicusWorkEditor;
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

    #[glib::derived_properties]
    impl ObjectImpl for MusicusWorkEditor {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("created")
                    .param_types([Work::static_type()])
                    .build()]
            });

            SIGNALS.as_ref()
        }

        fn constructed(&self) {
            self.parent_constructed();

            let persons_popover = MusicusPersonSelectorPopover::new(self.library.get().unwrap());

            let obj = self.obj().clone();
            persons_popover.connect_person_selected(move |_, person| {
                obj.add_composer(person);
            });

            let obj = self.obj().clone();
            persons_popover.connect_create(move |_| {
                let editor = MusicusPersonEditor::new(&obj.navigation(), &obj.library(), None);

                editor.connect_created(clone!(
                    #[weak]
                    obj,
                    move |_, person| {
                        obj.add_composer(person);
                    }
                ));

                obj.navigation().push(&editor);
            });

            self.select_person_box.append(&persons_popover);
            self.persons_popover.set(persons_popover).unwrap();

            let instruments_popover =
                MusicusInstrumentSelectorPopover::new(self.library.get().unwrap());

            let obj = self.obj().clone();
            instruments_popover.connect_instrument_selected(move |_, instrument| {
                obj.add_instrument_row(instrument);
            });

            let obj = self.obj().clone();
            instruments_popover.connect_create(move |_| {
                let editor = MusicusInstrumentEditor::new(&obj.navigation(), &obj.library(), None);

                editor.connect_created(clone!(
                    #[weak]
                    obj,
                    move |_, instrument| {
                        obj.add_instrument_row(instrument);
                    }
                ));

                obj.navigation().push(&editor);
            });

            self.select_instrument_box.append(&instruments_popover);
            self.instruments_popover.set(instruments_popover).unwrap();
        }
    }

    impl WidgetImpl for MusicusWorkEditor {}
    impl NavigationPageImpl for MusicusWorkEditor {}
}

glib::wrapper! {
    pub struct MusicusWorkEditor(ObjectSubclass<imp::MusicusWorkEditor>)
        @extends gtk::Widget, adw::NavigationPage;
}

#[gtk::template_callbacks]
impl MusicusWorkEditor {
    pub fn new(
        navigation: &adw::NavigationView,
        library: &MusicusLibrary,
        work: Option<&Work>,
        is_part_editor: bool,
    ) -> Self {
        let obj: Self = glib::Object::builder()
            .property("navigation", navigation)
            .property("library", library)
            .build();

        if is_part_editor {
            obj.set_title(&gettext("Work part"));
            obj.imp().save_row.set_title(&gettext("Add work part"));
            obj.imp().is_part_editor.set(true);
        }

        if let Some(work) = work {
            obj.imp().save_row.set_title(&gettext("_Save changes"));
            obj.imp().work_id.set(work.work_id.clone()).unwrap();

            obj.imp().name_editor.set_translation(&work.name);

            for part in &work.parts {
                obj.add_part_row(part.clone());
            }

            for composer in &work.persons {
                obj.add_composer_row(composer.clone());
            }

            for instrument in &work.instruments {
                obj.add_instrument_row(instrument.clone());
            }
        }

        obj
    }

    pub fn connect_created<F: Fn(&Self, Work) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("created", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let work = values[1].get::<Work>().unwrap();
            f(&obj, work);
            None
        })
    }

    #[template_callback]
    fn add_person(&self, _: &adw::ActionRow) {
        self.imp().persons_popover.get().unwrap().popup();
    }

    #[template_callback]
    fn add_part(&self, _: &adw::ActionRow) {
        let editor = MusicusWorkEditor::new(&self.navigation(), &self.library(), None, true);

        editor.connect_created(clone!(
            #[weak(rename_to = this)]
            self,
            move |_, part| {
                this.add_part_row(part);
            }
        ));

        self.navigation().push(&editor);
    }

    #[template_callback]
    fn add_instrument(&self, _: &adw::ActionRow) {
        self.imp().instruments_popover.get().unwrap().popup();
    }

    fn add_composer(&self, person: Person) {
        let role = self.library().composer_default_role().unwrap();
        let composer = Composer { person, role };
        self.add_composer_row(composer);
    }

    fn add_part_row(&self, part: Work) {
        let row = MusicusWorkEditorPartRow::new(&self.navigation(), &self.library(), part);

        row.connect_remove(clone!(
            #[weak(rename_to = this)]
            self,
            move |row| {
                this.imp().part_list.remove(row);
                this.imp().part_rows.borrow_mut().retain(|p| p != row);
            }
        ));

        self.imp()
            .part_list
            .insert(&row, self.imp().part_rows.borrow().len() as i32);

        self.imp().part_rows.borrow_mut().push(row);
    }

    fn add_composer_row(&self, composer: Composer) {
        let row = MusicusWorkEditorComposerRow::new(&self.navigation(), &self.library(), composer);

        row.connect_remove(clone!(
            #[weak(rename_to = this)]
            self,
            move |row| {
                this.imp().composer_list.remove(row);
                this.imp().composer_rows.borrow_mut().retain(|c| c != row);
            }
        ));

        self.imp()
            .composer_list
            .insert(&row, self.imp().composer_rows.borrow().len() as i32);

        self.imp().composer_rows.borrow_mut().push(row);
    }

    fn add_instrument_row(&self, instrument: Instrument) {
        let row = adw::ActionRow::builder()
            .title(instrument.to_string())
            .build();

        let remove_button = gtk::Button::builder()
            .icon_name("user-trash-symbolic")
            .valign(gtk::Align::Center)
            .css_classes(["flat"])
            .build();

        remove_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            #[weak]
            row,
            #[strong]
            instrument,
            move |_| {
                this.imp().instrument_list.remove(&row);
                this.imp()
                    .instruments
                    .borrow_mut()
                    .retain(|i| *i != instrument);
            }
        ));

        row.add_suffix(&remove_button);

        self.imp()
            .instrument_list
            .insert(&row, self.imp().instruments.borrow().len() as i32);

        self.imp().instruments.borrow_mut().push(instrument);
    }

    #[template_callback]
    fn save(&self) {
        let library = self.imp().library.get().unwrap();

        let name = self.imp().name_editor.translation();

        let parts = self
            .imp()
            .part_rows
            .borrow()
            .iter()
            .map(|p| p.part())
            .collect::<Vec<Work>>();

        let composers = self
            .imp()
            .composer_rows
            .borrow()
            .iter()
            .map(|c| c.composer())
            .collect::<Vec<Composer>>();
        let instruments = self.imp().instruments.borrow().clone();

        if self.imp().is_part_editor.get() {
            let work_id = self
                .imp()
                .work_id
                .get()
                .map(|w| w.to_string())
                .unwrap_or_else(db::generate_id);

            let part = Work {
                work_id,
                name,
                parts,
                persons: composers,
                instruments,
            };

            self.emit_by_name::<()>("created", &[&part]);
        } else {
            if let Some(work_id) = self.imp().work_id.get() {
                library
                    .update_work(work_id, name, parts, composers, instruments)
                    .unwrap();
            } else {
                let work = library
                    .create_work(name, parts, composers, instruments)
                    .unwrap();
                self.emit_by_name::<()>("created", &[&work]);
            }
        }

        self.imp().navigation.get().unwrap().pop();
    }
}
