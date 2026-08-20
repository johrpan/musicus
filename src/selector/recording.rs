use std::cell::{OnceCell, RefCell};

use gettextrs::gettext;
use gtk::{
    glib::{self, subclass::Signal, Properties},
    prelude::*,
    subclass::prelude::*,
};
use once_cell::sync::Lazy;

use musicus_library::db::models::{Person, Recording, Work};

use super::{
    connect_keynav, create_row, ComposerPrefill, RecordingPrefill, RecordingWork, WorkPrefill,
};
use crate::{
    library::{Library, SearchItem},
    util::activatable_row::ActivatableRow,
};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, Properties)]
    #[properties(wrapper_type = super::RecordingSelectorPopover)]
    #[template(file = "data/ui/selector/recording.blp")]
    pub struct RecordingSelectorPopover {
        #[property(get, construct_only)]
        pub library: OnceCell<Library>,

        pub composers: RefCell<Vec<SearchItem<Person>>>,
        pub works: RefCell<Vec<SearchItem<Work>>>,
        pub recordings: RefCell<Vec<SearchItem<Recording>>>,

        pub composer: RefCell<Option<Person>>,
        pub work: RefCell<Option<Work>>,

        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub composer_view: TemplateChild<adw::ToolbarView>,
        #[template_child]
        pub composer_search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub composer_scrolled_window: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub composer_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub work_view: TemplateChild<adw::ToolbarView>,
        #[template_child]
        pub composer_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub work_search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub work_scrolled_window: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub work_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub recording_view: TemplateChild<adw::ToolbarView>,
        #[template_child]
        pub work_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub recording_search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub recording_scrolled_window: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub recording_list: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RecordingSelectorPopover {
        const NAME: &'static str = "MusicusRecordingSelectorPopover";
        type Type = super::RecordingSelectorPopover;
        type ParentType = gtk::Popover;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for RecordingSelectorPopover {
        fn constructed(&self) {
            self.parent_constructed();

            self.obj().connect_visible_notify(|obj| {
                if obj.is_visible() {
                    obj.imp().stack.set_visible_child(&*obj.imp().composer_view);
                    obj.imp().composer_search_entry.set_text("");
                    obj.imp().composer_search_entry.grab_focus();
                    obj.imp()
                        .composer_scrolled_window
                        .vadjustment()
                        .set_value(0.0);
                }
            });

            connect_keynav(&self.composer_search_entry, &self.composer_list);
            connect_keynav(&self.work_search_entry, &self.work_list);
            connect_keynav(&self.recording_search_entry, &self.recording_list);

            self.obj().search_composers("");
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("selected")
                        .param_types([glib::BoxedAnyObject::static_type()])
                        .build(),
                    Signal::builder("create")
                        .param_types([glib::BoxedAnyObject::static_type()])
                        .build(),
                ]
            });

            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for RecordingSelectorPopover {}
    impl PopoverImpl for RecordingSelectorPopover {}
}

glib::wrapper! {
    pub struct RecordingSelectorPopover(ObjectSubclass<imp::RecordingSelectorPopover>)
        @extends gtk::Popover, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native, gtk::ShortcutManager;
}

#[gtk::template_callbacks]
impl RecordingSelectorPopover {
    pub fn new(library: &Library) -> Self {
        glib::Object::builder().property("library", library).build()
    }

    pub fn connect_selected<F: Fn(&Self, Recording) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("selected", true, move |values| {
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

    pub fn connect_create<F: Fn(&Self, RecordingPrefill) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("create", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let prefill = values[1]
                .get::<glib::BoxedAnyObject>()
                .unwrap()
                .borrow::<RecordingPrefill>()
                .clone();
            f(&obj, prefill);
            None
        })
    }

    #[template_callback]
    fn composer_search_changed(&self, entry: &gtk::SearchEntry) {
        self.search_composers(&entry.text());
    }

    #[template_callback]
    fn composer_activate(&self, _: &gtk::SearchEntry) {
        if let Some(item) = self.imp().composers.borrow().first() {
            self.select_composer(item.to_owned());
        } else {
            // There is no composer matching what the user typed, so it is the
            // name of a composer that does not exist yet.
            self.create(self.composer_prefill());
        }
    }

    #[template_callback]
    fn back_to_composer(&self) {
        self.imp()
            .stack
            .set_visible_child(&*self.imp().composer_view);
        self.imp().composer_search_entry.grab_focus();
    }

    #[template_callback]
    fn work_search_changed(&self, entry: &gtk::SearchEntry) {
        self.search_works(&entry.text());
    }

    #[template_callback]
    fn work_activate(&self, _: &gtk::SearchEntry) {
        if let Some(item) = self.imp().works.borrow().first() {
            self.select_work(item.to_owned());
        } else {
            // There is no work matching what the user typed, so it is the name
            // of a work that does not exist yet.
            self.create(self.work_prefill());
        }
    }

    #[template_callback]
    fn back_to_work(&self) {
        self.imp().stack.set_visible_child(&*self.imp().work_view);
        self.imp().work_search_entry.grab_focus();
    }

    #[template_callback]
    fn recording_search_changed(&self, entry: &gtk::SearchEntry) {
        self.search_recordings(&entry.text());
    }

    #[template_callback]
    fn recording_activate(&self, _: &gtk::SearchEntry) {
        if let Some(item) = self.imp().recordings.borrow().first() {
            self.select(item.to_owned());
        } else {
            self.create(self.recording_prefill());
        }
    }

    #[template_callback]
    fn stop_search(&self, _: &gtk::SearchEntry) {
        self.popdown();
    }

    fn search_composers(&self, search: &str) {
        let imp = self.imp();

        let persons = imp.library.get().unwrap().search_persons(search).unwrap();

        imp.composer_list.remove_all();

        for result in &persons {
            let text = result.item.to_string();
            let row = ActivatableRow::new(&super::item_row_child(&text, result.in_library, 0));

            let item = result.clone();
            let obj = self.clone();
            row.connect_activated(move |_: &ActivatableRow| {
                obj.select_composer(item.clone());
            });

            imp.composer_list.append(&row);
        }

        let obj = self.clone();
        imp.composer_list
            .append(&create_row(&gettext("Create new person"), move || {
                obj.create(obj.composer_prefill());
            }));

        let obj = self.clone();
        imp.composer_list
            .append(&create_row(&gettext("Create new recording"), move || {
                obj.create(RecordingPrefill::default());
            }));

        imp.composers.replace(persons);
    }

    fn search_works(&self, search: &str) {
        let imp = self.imp();

        let works = imp
            .library
            .get()
            .unwrap()
            .search_works(imp.composer.borrow().as_ref().unwrap(), search)
            .unwrap();

        imp.work_list.remove_all();

        for result in &works {
            let text = result.item.name.get().to_owned();
            let row = ActivatableRow::new(&super::item_row_child(&text, result.in_library, 0));

            let item = result.clone();
            let obj = self.clone();
            row.connect_activated(move |_: &ActivatableRow| {
                obj.select_work(item.clone());
            });

            imp.work_list.append(&row);
        }

        let obj = self.clone();
        imp.work_list
            .append(&create_row(&gettext("Create new work"), move || {
                obj.create(obj.work_prefill());
            }));

        let obj = self.clone();
        imp.work_list
            .append(&create_row(&gettext("Create new recording"), move || {
                obj.create(RecordingPrefill::default());
            }));

        imp.works.replace(works);
    }

    fn search_recordings(&self, search: &str) {
        let imp = self.imp();

        let recordings = imp
            .library
            .get()
            .unwrap()
            .search_recordings(imp.work.borrow().as_ref().unwrap(), search)
            .unwrap();

        imp.recording_list.remove_all();

        for result in &recordings {
            let mut text = result.item.performers_string();

            if let Some(comment) = &result.item.comment {
                text.push_str(&format!(" ({comment})"));
            }

            let row = ActivatableRow::new(&super::item_row_child(&text, result.in_library, 0));

            let item = result.clone();
            let obj = self.clone();
            row.connect_activated(move |_: &ActivatableRow| {
                obj.select(item.clone());
            });

            imp.recording_list.append(&row);
        }

        let obj = self.clone();
        imp.recording_list
            .append(&create_row(&gettext("Create new recording"), move || {
                obj.create(obj.recording_prefill());
            }));

        imp.recordings.replace(recordings);
    }

    fn select_composer(&self, item: SearchItem<Person>) {
        let person = if item.in_library {
            item.item
        } else {
            match self
                .imp()
                .library
                .get()
                .unwrap()
                .import_metadata_person(&item.item.person_id)
            {
                Ok(person) => person,
                Err(err) => {
                    log::error!("Failed to import person from metadata database: {err:?}");
                    return;
                }
            }
        };

        self.imp().composer_label.set_text(person.name.get());
        self.imp().work_search_entry.set_text("");
        self.imp().work_search_entry.grab_focus();
        self.imp().work_scrolled_window.vadjustment().set_value(0.0);
        self.imp().stack.set_visible_child(&*self.imp().work_view);

        self.imp().composer.replace(Some(person.clone()));
        self.search_works("");
    }

    fn select_work(&self, item: SearchItem<Work>) {
        let work = if item.in_library {
            item.item
        } else {
            match self
                .imp()
                .library
                .get()
                .unwrap()
                .import_metadata_work(&item.item.work_id)
            {
                Ok(work) => work,
                Err(err) => {
                    log::error!("Failed to import work from metadata database: {err:?}");
                    return;
                }
            }
        };

        self.imp().work_label.set_text(work.name.get());
        self.imp().recording_search_entry.set_text("");
        self.imp().recording_search_entry.grab_focus();
        self.imp()
            .recording_scrolled_window
            .vadjustment()
            .set_value(0.0);
        self.imp()
            .stack
            .set_visible_child(&*self.imp().recording_view);

        self.imp().work.replace(Some(work.clone()));
        self.search_recordings("");
    }

    fn select(&self, item: SearchItem<Recording>) {
        let recording = if item.in_library {
            item.item
        } else {
            match self
                .imp()
                .library
                .get()
                .unwrap()
                .import_metadata_recording(&item.item.recording_id)
            {
                Ok(recording) => recording,
                Err(err) => {
                    log::error!("Failed to import recording from metadata database: {err:?}");
                    return;
                }
            }
        };

        self.emit_by_name::<()>("selected", &[&glib::BoxedAnyObject::new(recording.clone())]);
        self.popdown();
    }

    /// What is known about a new recording when creating its work and that work's composer
    /// first from within the composer pane.
    fn composer_prefill(&self) -> RecordingPrefill {
        RecordingPrefill {
            work: RecordingWork::New(WorkPrefill {
                composer: ComposerPrefill::New(self.imp().composer_search_entry.text().to_string()),
                name: String::new(),
            }),
        }
    }

    /// What is known about a new recording when creating its work first from within the
    /// work pane.
    fn work_prefill(&self) -> RecordingPrefill {
        RecordingPrefill {
            work: RecordingWork::New(WorkPrefill {
                composer: match self.imp().composer.borrow().clone() {
                    Some(composer) => ComposerPrefill::Person(composer),
                    None => ComposerPrefill::Unknown,
                },
                name: self.imp().work_search_entry.text().to_string(),
            }),
        }
    }

    /// What is known about a new recording when creating it from within the recording pane.
    fn recording_prefill(&self) -> RecordingPrefill {
        RecordingPrefill {
            work: match self.imp().work.borrow().clone() {
                Some(work) => RecordingWork::Work(work),
                None => RecordingWork::Unknown,
            },
        }
    }

    fn create(&self, prefill: RecordingPrefill) {
        self.emit_by_name::<()>("create", &[&glib::BoxedAnyObject::new(prefill)]);
        self.popdown();
    }
}
