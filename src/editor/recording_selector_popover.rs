use crate::{
    db::models::{Person, Recording, Work},
    library::MusicusLibrary,
};

use gettextrs::gettext;
use gtk::{
    glib::{self, subclass::Signal, Properties},
    pango,
    prelude::*,
    subclass::prelude::*,
};
use once_cell::sync::Lazy;

use std::cell::{OnceCell, RefCell};

use super::activatable_row::MusicusActivatableRow;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, Properties)]
    #[properties(wrapper_type = super::RecordingSelectorPopover)]
    #[template(file = "data/ui/recording_selector_popover.blp")]
    pub struct RecordingSelectorPopover {
        #[property(get, construct_only)]
        pub library: OnceCell<MusicusLibrary>,

        pub composers: RefCell<Vec<Person>>,
        pub works: RefCell<Vec<Work>>,
        pub recordings: RefCell<Vec<Recording>>,

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

            self.obj()
                .connect_visible_notify(|obj: &super::RecordingSelectorPopover| {
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

            self.obj().search_composers("");
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("selected")
                        .param_types([Recording::static_type()])
                        .build(),
                    Signal::builder("create").build(),
                ]
            });

            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for RecordingSelectorPopover {
        // TODO: Fix focus.
        fn focus(&self, direction_type: gtk::DirectionType) -> bool {
            if direction_type == gtk::DirectionType::Down {
                if self.stack.visible_child() == Some(self.composer_list.get().upcast()) {
                    self.composer_list.child_focus(direction_type)
                } else if self.stack.visible_child() == Some(self.work_list.get().upcast()) {
                    self.work_list.child_focus(direction_type)
                } else {
                    self.recording_list.child_focus(direction_type)
                }
            } else {
                self.parent_focus(direction_type)
            }
        }
    }

    impl PopoverImpl for RecordingSelectorPopover {}
}

glib::wrapper! {
    pub struct RecordingSelectorPopover(ObjectSubclass<imp::RecordingSelectorPopover>)
        @extends gtk::Widget, gtk::Popover;
}

#[gtk::template_callbacks]
impl RecordingSelectorPopover {
    pub fn new(library: &MusicusLibrary) -> Self {
        glib::Object::builder().property("library", library).build()
    }

    pub fn connect_selected<F: Fn(&Self, Recording) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("selected", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let recording = values[1].get::<Recording>().unwrap();
            f(&obj, recording);
            None
        })
    }

    pub fn connect_create<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("create", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            f(&obj);
            None
        })
    }

    #[template_callback]
    fn composer_search_changed(&self, entry: &gtk::SearchEntry) {
        self.search_composers(&entry.text());
    }

    #[template_callback]
    fn composer_activate(&self, _: &gtk::SearchEntry) {
        if let Some(composer) = self.imp().composers.borrow().first() {
            self.select_composer(composer.to_owned());
        } else {
            self.create();
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
        if let Some(work) = self.imp().works.borrow().first() {
            self.select_work(work.to_owned());
        } else {
            self.create();
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
        if let Some(recording) = self.imp().recordings.borrow().first() {
            self.select(recording.to_owned());
        } else {
            self.create();
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

        for person in &persons {
            let row = MusicusActivatableRow::new(
                &gtk::Label::builder()
                    .label(person.to_string())
                    .halign(gtk::Align::Start)
                    .ellipsize(pango::EllipsizeMode::Middle)
                    .build(),
            );

            row.set_tooltip_text(Some(&person.to_string()));

            let person = person.clone();
            let obj = self.clone();
            row.connect_activated(move |_: &MusicusActivatableRow| {
                obj.select_composer(person.clone());
            });

            imp.composer_list.append(&row);
        }

        let create_box = gtk::Box::builder().spacing(12).build();
        create_box.append(&gtk::Image::builder().icon_name("list-add-symbolic").build());
        create_box.append(
            &gtk::Label::builder()
                .label(gettext("Create new recording"))
                .halign(gtk::Align::Start)
                .build(),
        );

        let create_row = MusicusActivatableRow::new(&create_box);
        let obj = self.clone();
        create_row.connect_activated(move |_: &MusicusActivatableRow| {
            obj.create();
        });

        imp.composer_list.append(&create_row);

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

        for work in &works {
            let row = MusicusActivatableRow::new(
                &gtk::Label::builder()
                    .label(work.name.get())
                    .halign(gtk::Align::Start)
                    .ellipsize(pango::EllipsizeMode::Middle)
                    .build(),
            );

            row.set_tooltip_text(Some(&work.name.get()));

            let work = work.clone();
            let obj = self.clone();
            row.connect_activated(move |_: &MusicusActivatableRow| {
                obj.select_work(work.clone());
            });

            imp.work_list.append(&row);
        }

        let create_box = gtk::Box::builder().spacing(12).build();
        create_box.append(&gtk::Image::builder().icon_name("list-add-symbolic").build());
        create_box.append(
            &gtk::Label::builder()
                .label(gettext("Create new recording"))
                .halign(gtk::Align::Start)
                .build(),
        );

        let create_row = MusicusActivatableRow::new(&create_box);
        let obj = self.clone();
        create_row.connect_activated(move |_: &MusicusActivatableRow| {
            obj.create();
        });

        imp.work_list.append(&create_row);

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

        for recording in &recordings {
            let mut label = recording.performers_string();

            if let Some(year) = recording.year {
                label.push_str(&format!(" ({year})"));
            }

            let row = MusicusActivatableRow::new(
                &gtk::Label::builder()
                    .label(&label)
                    .halign(gtk::Align::Start)
                    .ellipsize(pango::EllipsizeMode::Middle)
                    .build(),
            );

            row.set_tooltip_text(Some(&label));

            let recording = recording.clone();
            let obj = self.clone();
            row.connect_activated(move |_: &MusicusActivatableRow| {
                obj.select(recording.clone());
            });

            imp.recording_list.append(&row);
        }

        let create_box = gtk::Box::builder().spacing(12).build();
        create_box.append(&gtk::Image::builder().icon_name("list-add-symbolic").build());
        create_box.append(
            &gtk::Label::builder()
                .label(gettext("Create new recording"))
                .halign(gtk::Align::Start)
                .build(),
        );

        let create_row = MusicusActivatableRow::new(&create_box);
        let obj = self.clone();
        create_row.connect_activated(move |_: &MusicusActivatableRow| {
            obj.create();
        });

        imp.recording_list.append(&create_row);

        imp.recordings.replace(recordings);
    }

    fn select_composer(&self, person: Person) {
        self.imp().composer_label.set_text(person.name.get());
        self.imp().work_search_entry.set_text("");
        self.imp().work_search_entry.grab_focus();
        self.imp().work_scrolled_window.vadjustment().set_value(0.0);
        self.imp().stack.set_visible_child(&*self.imp().work_view);

        self.imp().composer.replace(Some(person.clone()));
        self.search_works("");
    }

    fn select_work(&self, work: Work) {
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

    fn select(&self, recording: Recording) {
        self.emit_by_name::<()>("selected", &[&recording]);
        self.popdown();
    }

    fn create(&self) {
        self.emit_by_name::<()>("create", &[]);
        self.popdown();
    }
}
