use std::cell::{OnceCell, RefCell};

use adw::{prelude::*, subclass::prelude::*};
use gettextrs::gettext;
use gtk::glib::{self, clone, subclass::Signal, Properties};
use once_cell::sync::Lazy;

use crate::{
    db::models::{Ensemble, EnsemblePerformer, Performer, Person, Recording, Work},
    editor::{
        ensemble_editor::MusicusEnsembleEditor,
        ensemble_selector_popover::MusicusEnsembleSelectorPopover,
        person_editor::MusicusPersonEditor, person_selector_popover::MusicusPersonSelectorPopover,
        recording_editor_ensemble_row::MusicusRecordingEditorEnsembleRow,
        recording_editor_performer_row::MusicusRecordingEditorPerformerRow,
        work_selector_popover::MusicusWorkSelectorPopover,
    },
    library::MusicusLibrary,
};

mod imp {
    use crate::editor::work_editor::MusicusWorkEditor;

    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, Properties)]
    #[properties(wrapper_type = super::MusicusRecordingEditor)]
    #[template(file = "data/ui/recording_editor.blp")]
    pub struct MusicusRecordingEditor {
        #[property(get, construct_only)]
        pub navigation: OnceCell<adw::NavigationView>,

        #[property(get, construct_only)]
        pub library: OnceCell<MusicusLibrary>,

        pub recording_id: OnceCell<String>,

        pub work: RefCell<Option<Work>>,
        pub performer_rows: RefCell<Vec<MusicusRecordingEditorPerformerRow>>,
        pub ensemble_rows: RefCell<Vec<MusicusRecordingEditorEnsembleRow>>,

        pub work_selector_popover: OnceCell<MusicusWorkSelectorPopover>,
        pub persons_popover: OnceCell<MusicusPersonSelectorPopover>,
        pub ensembles_popover: OnceCell<MusicusEnsembleSelectorPopover>,

        #[template_child]
        pub work_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub select_work_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub year_row: TemplateChild<adw::SpinRow>,
        #[template_child]
        pub performer_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub select_person_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub ensemble_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub select_ensemble_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub save_row: TemplateChild<adw::ButtonRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MusicusRecordingEditor {
        const NAME: &'static str = "MusicusRecordingEditor";
        type Type = super::MusicusRecordingEditor;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for MusicusRecordingEditor {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("created")
                    .param_types([Recording::static_type()])
                    .build()]
            });

            SIGNALS.as_ref()
        }

        fn constructed(&self) {
            self.parent_constructed();

            let work_selector_popover =
                MusicusWorkSelectorPopover::new(self.library.get().unwrap());

            let obj = self.obj().clone();
            work_selector_popover.connect_selected(move |_, work| {
                obj.set_work(work);
            });

            let obj = self.obj().clone();
            work_selector_popover.connect_create(move |_| {
                let editor = MusicusWorkEditor::new(&obj.navigation(), &obj.library(), None, false);

                editor.connect_created(clone!(
                    #[weak]
                    obj,
                    move |_, work| {
                        obj.set_work(work);
                    }
                ));

                obj.navigation().push(&editor);
            });

            self.select_work_box.append(&work_selector_popover);
            self.work_selector_popover
                .set(work_selector_popover)
                .unwrap();

            let persons_popover = MusicusPersonSelectorPopover::new(self.library.get().unwrap());

            let obj = self.obj().clone();
            persons_popover.connect_person_selected(move |_, person| {
                obj.new_performer(person);
            });

            let obj = self.obj().clone();
            persons_popover.connect_create(move |_| {
                let editor = MusicusPersonEditor::new(&obj.navigation(), &obj.library(), None);

                editor.connect_created(clone!(
                    #[weak]
                    obj,
                    move |_, person| {
                        obj.new_performer(person);
                    }
                ));

                obj.navigation().push(&editor);
            });

            self.select_person_box.append(&persons_popover);
            self.persons_popover.set(persons_popover).unwrap();

            let ensembles_popover =
                MusicusEnsembleSelectorPopover::new(self.library.get().unwrap());

            let obj = self.obj().clone();
            ensembles_popover.connect_ensemble_selected(move |_, ensemble| {
                obj.new_ensemble_performer(ensemble);
            });

            let obj = self.obj().clone();
            ensembles_popover.connect_create(move |_| {
                let editor = MusicusEnsembleEditor::new(&obj.navigation(), &obj.library(), None);

                editor.connect_created(clone!(
                    #[weak]
                    obj,
                    move |_, ensemble| {
                        obj.new_ensemble_performer(ensemble);
                    }
                ));

                obj.navigation().push(&editor);
            });

            self.select_ensemble_box.append(&ensembles_popover);
            self.ensembles_popover.set(ensembles_popover).unwrap();
        }
    }

    impl WidgetImpl for MusicusRecordingEditor {}
    impl NavigationPageImpl for MusicusRecordingEditor {}
}

glib::wrapper! {
    pub struct MusicusRecordingEditor(ObjectSubclass<imp::MusicusRecordingEditor>)
        @extends gtk::Widget, adw::NavigationPage;
}

#[gtk::template_callbacks]
impl MusicusRecordingEditor {
    pub fn new(
        navigation: &adw::NavigationView,
        library: &MusicusLibrary,
        recording: Option<&Recording>,
    ) -> Self {
        let obj: Self = glib::Object::builder()
            .property("navigation", navigation)
            .property("library", library)
            .build();

        if let Some(recording) = recording {
            obj.imp().save_row.set_title(&gettext("_Save changes"));
            obj.imp()
                .recording_id
                .set(recording.recording_id.clone())
                .unwrap();

            obj.set_work(recording.work.clone());

            if let Some(year) = recording.year {
                obj.imp().year_row.set_value(year as f64);
            }

            for performer in recording.persons.clone() {
                obj.add_performer_row(performer);
            }

            for ensemble_performer in recording.ensembles.clone() {
                obj.add_ensemble_row(ensemble_performer);
            }
        }

        obj
    }

    pub fn connect_created<F: Fn(&Self, Recording) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("created", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let recording = values[1].get::<Recording>().unwrap();
            f(&obj, recording);
            None
        })
    }

    #[template_callback]
    fn select_work(&self, _: &adw::ActionRow) {
        self.imp().work_selector_popover.get().unwrap().popup();
    }

    #[template_callback]
    fn select_person(&self, _: &adw::ActionRow) {
        self.imp().persons_popover.get().unwrap().popup();
    }

    #[template_callback]
    fn select_ensemble(&self, _: &adw::ActionRow) {
        self.imp().ensembles_popover.get().unwrap().popup();
    }

    fn set_work(&self, work: Work) {
        self.imp().work_row.set_title(&work.name.get());
        self.imp().work_row.set_subtitle(
            &work
                .composers_string()
                .unwrap_or_else(|| gettext("No composers")),
        );
        self.imp().work.replace(Some(work));
    }

    fn new_performer(&self, person: Person) {
        let performer = Performer {
            person,
            role: self.library().performer_default_role().unwrap(),
            instrument: None,
        };

        self.add_performer_row(performer);
    }

    fn add_performer_row(&self, performer: Performer) {
        let row =
            MusicusRecordingEditorPerformerRow::new(&self.navigation(), &self.library(), performer);

        row.connect_remove(clone!(
            #[weak(rename_to = this)]
            self,
            move |row| {
                this.imp().performer_list.remove(row);
                this.imp().performer_rows.borrow_mut().retain(|c| c != row);
            }
        ));

        self.imp()
            .performer_list
            .insert(&row, self.imp().performer_rows.borrow().len() as i32);

        self.imp().performer_rows.borrow_mut().push(row);
    }

    fn new_ensemble_performer(&self, ensemble: Ensemble) {
        let performer = EnsemblePerformer {
            ensemble,
            role: self.library().performer_default_role().unwrap(),
        };

        self.add_ensemble_row(performer);
    }

    fn add_ensemble_row(&self, ensemble_performer: EnsemblePerformer) {
        let row = MusicusRecordingEditorEnsembleRow::new(
            &self.navigation(),
            &self.library(),
            ensemble_performer,
        );

        row.connect_remove(clone!(
            #[weak(rename_to = this)]
            self,
            move |row| {
                this.imp().ensemble_list.remove(row);
                this.imp().ensemble_rows.borrow_mut().retain(|c| c != row);
            }
        ));

        self.imp()
            .ensemble_list
            .insert(&row, self.imp().ensemble_rows.borrow().len() as i32);

        self.imp().ensemble_rows.borrow_mut().push(row);
    }

    #[template_callback]
    fn save(&self) {
        let library = self.imp().library.get().unwrap();

        // TODO: No work selected?
        let work = self.imp().work.borrow().as_ref().unwrap().clone();
        let year = self.imp().year_row.value() as i32;

        let performers = self
            .imp()
            .performer_rows
            .borrow()
            .iter()
            .map(|p| p.performer())
            .collect::<Vec<Performer>>();

        let ensembles = self
            .imp()
            .ensemble_rows
            .borrow()
            .iter()
            .map(|e| e.ensemble())
            .collect::<Vec<EnsemblePerformer>>();

        if let Some(recording_id) = self.imp().recording_id.get() {
            library
                .update_recording(recording_id, work, Some(year), performers, ensembles)
                .unwrap();
        } else {
            let recording = library
                .create_recording(work, Some(year), performers, ensembles)
                .unwrap();
            self.emit_by_name::<()>("created", &[&recording]);
        }

        self.imp().navigation.get().unwrap().pop();
    }
}
