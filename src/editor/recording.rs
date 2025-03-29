mod ensemble_row;
mod performer_row;

use std::cell::{OnceCell, RefCell};

use adw::{prelude::*, subclass::prelude::*};
use ensemble_row::RecordingEditorEnsembleRow;
use gettextrs::gettext;
use gtk::glib::{self, clone, subclass::Signal, Properties};
use once_cell::sync::Lazy;
use performer_row::RecordingEditorPerformerRow;

use crate::{
    db::models::{Ensemble, EnsemblePerformer, Performer, Person, Recording, Work},
    editor::{ensemble::EnsembleEditor, person::PersonEditor, work::WorkEditor},
    library::Library,
    selector::{
        ensemble::EnsembleSelectorPopover, person::PersonSelectorPopover, work::WorkSelectorPopover,
    },
};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, Properties)]
    #[properties(wrapper_type = super::RecordingEditor)]
    #[template(file = "data/ui/editor/recording.blp")]
    pub struct RecordingEditor {
        #[property(get, construct_only)]
        pub navigation: OnceCell<adw::NavigationView>,

        #[property(get, construct_only)]
        pub library: OnceCell<Library>,

        pub recording_id: OnceCell<String>,

        pub work: RefCell<Option<Work>>,
        pub performer_rows: RefCell<Vec<RecordingEditorPerformerRow>>,
        pub ensemble_rows: RefCell<Vec<RecordingEditorEnsembleRow>>,

        pub work_selector_popover: OnceCell<WorkSelectorPopover>,
        pub persons_popover: OnceCell<PersonSelectorPopover>,
        pub ensembles_popover: OnceCell<EnsembleSelectorPopover>,

        #[template_child]
        pub work_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub select_work_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub year_row: TemplateChild<adw::SpinRow>,
        #[template_child]
        pub performers_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub performer_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub ensembles_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub ensemble_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub save_row: TemplateChild<adw::ButtonRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RecordingEditor {
        const NAME: &'static str = "MusicusRecordingEditor";
        type Type = super::RecordingEditor;
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
    impl ObjectImpl for RecordingEditor {
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

            let work_selector_popover = WorkSelectorPopover::new(self.library.get().unwrap());

            let obj = self.obj().clone();
            work_selector_popover.connect_selected(move |_, work| {
                obj.set_work(work);
            });

            let obj = self.obj().clone();
            work_selector_popover.connect_create(move |_| {
                let editor = WorkEditor::new(&obj.navigation(), &obj.library(), None, false);

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

            let persons_popover = PersonSelectorPopover::new(self.library.get().unwrap());

            let obj = self.obj().clone();
            persons_popover.connect_person_selected(move |_, person| {
                obj.new_performer(person);
            });

            let obj = self.obj().clone();
            persons_popover.connect_create(move |_| {
                let editor = PersonEditor::new(&obj.navigation(), &obj.library(), None);

                editor.connect_created(clone!(
                    #[weak]
                    obj,
                    move |_, person| {
                        obj.new_performer(person);
                    }
                ));

                obj.navigation().push(&editor);
            });

            self.performers_box.append(&persons_popover);
            self.persons_popover.set(persons_popover).unwrap();

            let ensembles_popover = EnsembleSelectorPopover::new(self.library.get().unwrap());

            let obj = self.obj().clone();
            ensembles_popover.connect_ensemble_selected(move |_, ensemble| {
                obj.new_ensemble_performer(ensemble);
            });

            let obj = self.obj().clone();
            ensembles_popover.connect_create(move |_| {
                let editor = EnsembleEditor::new(&obj.navigation(), &obj.library(), None);

                editor.connect_created(clone!(
                    #[weak]
                    obj,
                    move |_, ensemble| {
                        obj.new_ensemble_performer(ensemble);
                    }
                ));

                obj.navigation().push(&editor);
            });

            self.ensembles_box.append(&ensembles_popover);
            self.ensembles_popover.set(ensembles_popover).unwrap();
        }
    }

    impl WidgetImpl for RecordingEditor {}
    impl NavigationPageImpl for RecordingEditor {}
}

glib::wrapper! {
    pub struct RecordingEditor(ObjectSubclass<imp::RecordingEditor>)
        @extends gtk::Widget, adw::NavigationPage;
}

#[gtk::template_callbacks]
impl RecordingEditor {
    pub fn new(
        navigation: &adw::NavigationView,
        library: &Library,
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
    fn select_work(&self) {
        self.imp().work_selector_popover.get().unwrap().popup();
    }

    #[template_callback]
    fn select_person(&self) {
        self.imp().persons_popover.get().unwrap().popup();
    }

    #[template_callback]
    fn select_ensemble(&self) {
        self.imp().ensembles_popover.get().unwrap().popup();
    }

    fn set_work(&self, work: Work) {
        self.imp().work_row.set_title(&work.name.get());
        self.imp().work_row.set_subtitle(
            &work
                .composers_string()
                .unwrap_or_else(|| gettext("No composers")),
        );
        self.imp().save_row.set_sensitive(true);
        self.imp().work.replace(Some(work));
    }

    fn new_performer(&self, person: Person) {
        let performer = Performer {
            person,
            role: None,
            instrument: None,
        };

        self.add_performer_row(performer);
    }

    fn add_performer_row(&self, performer: Performer) {
        let row = RecordingEditorPerformerRow::new(&self.navigation(), &self.library(), performer);

        row.connect_move(clone!(
            #[weak(rename_to = this)]
            self,
            move |target, source| {
                let mut performer_rows = this.imp().performer_rows.borrow_mut();
                if let Some(index) = performer_rows.iter().position(|p| p == target) {
                    this.imp().performer_list.remove(&source);
                    performer_rows.retain(|p| p != &source);
                    this.imp().performer_list.insert(&source, index as i32);
                    performer_rows.insert(index, source);
                }
            }
        ));

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
            role: None,
        };

        self.add_ensemble_row(performer);
    }

    fn add_ensemble_row(&self, ensemble_performer: EnsemblePerformer) {
        let row = RecordingEditorEnsembleRow::new(
            &self.navigation(),
            &self.library(),
            ensemble_performer,
        );

        row.connect_move(clone!(
            #[weak(rename_to = this)]
            self,
            move |target, source| {
                let mut ensemble_rows = this.imp().ensemble_rows.borrow_mut();
                if let Some(index) = ensemble_rows.iter().position(|p| p == target) {
                    this.imp().ensemble_list.remove(&source);
                    ensemble_rows.retain(|p| p != &source);
                    this.imp().ensemble_list.insert(&source, index as i32);
                    ensemble_rows.insert(index, source);
                }
            }
        ));

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
        if let Some(work) = &*self.imp().work.borrow() {
            let library = self.imp().library.get().unwrap();

            let work = work.to_owned();
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
}
