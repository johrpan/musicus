mod ensemble_row;

use std::cell::{OnceCell, RefCell};

use adw::{prelude::*, subclass::prelude::*};
use ensemble_row::RecordingEditorEnsembleRow;
use gettextrs::gettext;
use gtk::glib::{self, clone, subclass::Signal, Properties};
use once_cell::sync::Lazy;

use crate::editor::performer_row::PerformerRow;

use musicus_library::db::models::{
    Ensemble, EnsemblePerformer, Performer, Person, Recording, Tag, TagValue, Work,
};

use crate::{
    editor::{
        create, ensemble::EnsembleEditor, simple_entity::SimpleEntityEditor, tag::TagEditor,
        tag_row::TagRow,
    },
    library::Library,
    selector::{work::WorkSelectorPopover, RecordingPrefill, RecordingWork, SelectorPopover},
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
        pub performer_rows: RefCell<Vec<PerformerRow>>,
        pub ensemble_rows: RefCell<Vec<RecordingEditorEnsembleRow>>,
        pub tag_rows: RefCell<Vec<TagRow>>,

        pub work_selector_popover: OnceCell<WorkSelectorPopover>,
        pub persons_popover: OnceCell<SelectorPopover>,
        pub ensembles_popover: OnceCell<SelectorPopover>,
        pub tags_popover: OnceCell<SelectorPopover>,

        #[template_child]
        pub work_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub select_work_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub performers_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub performer_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub ensembles_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub ensemble_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub tags_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub tag_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub enable_updates_row: TemplateChild<adw::SwitchRow>,
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
                    .param_types([glib::BoxedAnyObject::static_type()])
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
            work_selector_popover.connect_create(move |_, prefill| {
                create::work(
                    &obj.navigation(),
                    &obj.library(),
                    prefill,
                    clone!(
                        #[weak]
                        obj,
                        move |work| {
                            obj.set_work(work);
                        }
                    ),
                );
            });

            self.select_work_box.append(&work_selector_popover);
            self.work_selector_popover
                .set(work_selector_popover)
                .unwrap();

            let persons_popover = SelectorPopover::persons(self.library.get().unwrap());

            let obj = self.obj().clone();
            persons_popover.connect_selected(move |_, person: Person| {
                obj.new_performer(person);
            });

            let obj = self.obj().clone();
            persons_popover.connect_create(move |_, search| {
                let editor = SimpleEntityEditor::person(&obj.navigation(), &obj.library(), None);
                editor.set_name(&search);

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

            let ensembles_popover = SelectorPopover::ensembles(self.library.get().unwrap());

            let obj = self.obj().clone();
            ensembles_popover.connect_selected(move |_, ensemble: Ensemble| {
                obj.new_ensemble_performer(ensemble);
            });

            let obj = self.obj().clone();
            ensembles_popover.connect_create(move |_, search| {
                let editor = EnsembleEditor::new(&obj.navigation(), &obj.library(), None);
                editor.set_name(&search);

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

            let tags_popover = SelectorPopover::tags(self.library.get().unwrap());

            let obj = self.obj().clone();
            tags_popover.connect_selected(move |_, tag: Tag| {
                obj.add_tag_row(TagValue { tag, value: None });
            });

            let obj = self.obj().clone();
            tags_popover.connect_create(move |_, search| {
                let editor = TagEditor::new(&obj.navigation(), &obj.library(), None);
                editor.set_name(&search);

                editor.connect_created(clone!(
                    #[weak]
                    obj,
                    move |_, tag| {
                        obj.add_tag_row(TagValue { tag, value: None });
                    }
                ));

                obj.navigation().push(&editor);
            });

            self.tags_box.append(&tags_popover);
            self.tags_popover.set(tags_popover).unwrap();
        }
    }

    impl WidgetImpl for RecordingEditor {}
    impl NavigationPageImpl for RecordingEditor {}
}

glib::wrapper! {
    pub struct RecordingEditor(ObjectSubclass<imp::RecordingEditor>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
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

            for performer in recording.persons.clone() {
                obj.add_performer_row(performer);
            }

            for ensemble_performer in recording.ensembles.clone() {
                obj.add_ensemble_row(ensemble_performer);
            }

            for tag_value in recording.tags.clone() {
                obj.add_tag_row(tag_value);
            }
        }

        obj
    }

    pub fn prefill(&self, prefill: &RecordingPrefill) {
        if let RecordingWork::Work(work) = &prefill.work {
            self.set_work(work.to_owned());
        }
    }

    pub fn connect_created<F: Fn(&Self, Recording) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("created", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let recording = values[1]
                .get::<glib::BoxedAnyObject>()
                .unwrap()
                .borrow::<Recording>()
                .clone();
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
        self.imp().work_row.set_title(work.name.get());
        self.imp().work_row.set_subtitle(
            &work
                .composers_string()
                .unwrap_or_else(|| gettext("No composers")),
        );

        self.imp()
            .enable_updates_row
            .set_active(work.enable_updates);

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
        let row = PerformerRow::new(
            &self.navigation(),
            &self.library(),
            performer,
            &gettext("Performer"),
        );

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
    fn add_tag(&self) {
        self.imp().tags_popover.get().unwrap().popup();
    }

    fn add_tag_row(&self, tag_value: TagValue) {
        let row = TagRow::new(tag_value);

        row.connect_move(clone!(
            #[weak(rename_to = this)]
            self,
            move |target, source| {
                let mut tag_rows = this.imp().tag_rows.borrow_mut();
                if let Some(index) = tag_rows.iter().position(|p| p == target) {
                    this.imp().tag_list.remove(&source);
                    tag_rows.retain(|p| p != &source);
                    this.imp().tag_list.insert(&source, index as i32);
                    tag_rows.insert(index, source);
                }
            }
        ));

        row.connect_remove(clone!(
            #[weak(rename_to = this)]
            self,
            move |row| {
                this.imp().tag_list.remove(row);
                this.imp().tag_rows.borrow_mut().retain(|p| p != row);
            }
        ));

        self.imp()
            .tag_list
            .insert(&row, self.imp().tag_rows.borrow().len() as i32);

        self.imp().tag_rows.borrow_mut().push(row);
    }

    #[template_callback]
    fn save(&self) {
        if let Some(work) = &*self.imp().work.borrow() {
            let library = self.imp().library.get().unwrap();

            let work = work.to_owned();

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

            let tags = self
                .imp()
                .tag_rows
                .borrow()
                .iter()
                .map(|r| r.tag_value())
                .collect::<Vec<TagValue>>();

            let enable_updates = self.imp().enable_updates_row.is_active();

            let created = if let Some(recording_id) = self.imp().recording_id.get() {
                if crate::editor::handle_save(
                    self,
                    library.update_recording(
                        recording_id,
                        work,
                        performers,
                        ensembles,
                        tags,
                        enable_updates,
                    ),
                )
                .is_none()
                {
                    return;
                }

                None
            } else {
                let Some(recording) = crate::editor::handle_save(
                    self,
                    library.create_recording(work, performers, ensembles, tags, enable_updates),
                ) else {
                    return;
                };

                Some(glib::BoxedAnyObject::new(recording))
            };

            // Popping before emitting "created" lets the next step of a guided
            // creation push its editor right away, instead of having to defer
            // until this editor has left the navigation stack.
            self.imp().navigation.get().unwrap().pop();

            if let Some(item) = created {
                self.emit_by_name::<()>("created", &[&item]);
            }
        }
    }
}
