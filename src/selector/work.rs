use std::cell::{OnceCell, RefCell};

use gettextrs::gettext;
use gtk::{
    glib::{self, subclass::Signal, Properties},
    pango,
    prelude::*,
    subclass::prelude::*,
};
use once_cell::sync::Lazy;

use crate::{
    db::models::{Person, Work},
    library::Library,
    util::activatable_row::ActivatableRow,
};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, Properties)]
    #[properties(wrapper_type = super::WorkSelectorPopover)]
    #[template(file = "data/ui/selector/work.blp")]
    pub struct WorkSelectorPopover {
        #[property(get, construct_only)]
        pub library: OnceCell<Library>,

        pub composers: RefCell<Vec<Person>>,
        pub composer: RefCell<Option<Person>>,
        pub works: RefCell<Vec<Work>>,

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
    }

    #[glib::object_subclass]
    impl ObjectSubclass for WorkSelectorPopover {
        const NAME: &'static str = "MusicusWorkSelectorPopover";
        type Type = super::WorkSelectorPopover;
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
    impl ObjectImpl for WorkSelectorPopover {
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

            self.obj().search_composers("");
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("selected")
                        .param_types([Work::static_type()])
                        .build(),
                    Signal::builder("create").build(),
                ]
            });

            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for WorkSelectorPopover {
        // TODO: Fix focus.
        fn focus(&self, direction_type: gtk::DirectionType) -> bool {
            if direction_type == gtk::DirectionType::Down {
                if self.stack.visible_child() == Some(self.composer_list.get().upcast()) {
                    self.composer_list.child_focus(direction_type)
                } else {
                    self.work_list.child_focus(direction_type)
                }
            } else {
                self.parent_focus(direction_type)
            }
        }
    }

    impl PopoverImpl for WorkSelectorPopover {}
}

glib::wrapper! {
    pub struct WorkSelectorPopover(ObjectSubclass<imp::WorkSelectorPopover>)
        @extends gtk::Widget, gtk::Popover;
}

#[gtk::template_callbacks]
impl WorkSelectorPopover {
    pub fn new(library: &Library) -> Self {
        glib::Object::builder().property("library", library).build()
    }

    pub fn connect_selected<F: Fn(&Self, Work) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("selected", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let work = values[1].get::<Work>().unwrap();
            f(&obj, work);
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
    fn back_button_clicked(&self) {
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
            self.select(work.clone());
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
            let row = ActivatableRow::new(
                &gtk::Label::builder()
                    .label(person.to_string())
                    .halign(gtk::Align::Start)
                    .ellipsize(pango::EllipsizeMode::Middle)
                    .build(),
            );

            row.set_tooltip_text(Some(&person.to_string()));

            let person = person.clone();
            let obj = self.clone();
            row.connect_activated(move |_: &ActivatableRow| {
                obj.select_composer(person.clone());
            });

            imp.composer_list.append(&row);
        }

        let create_box = gtk::Box::builder().spacing(12).build();
        create_box.append(&gtk::Image::builder().icon_name("list-add-symbolic").build());
        create_box.append(
            &gtk::Label::builder()
                .label(gettext("Create new work"))
                .halign(gtk::Align::Start)
                .build(),
        );

        let create_row = ActivatableRow::new(&create_box);
        let obj = self.clone();
        create_row.connect_activated(move |_: &ActivatableRow| {
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
            let row = ActivatableRow::new(
                &gtk::Label::builder()
                    .label(work.name.get())
                    .halign(gtk::Align::Start)
                    .ellipsize(pango::EllipsizeMode::Middle)
                    .build(),
            );

            row.set_tooltip_text(Some(work.name.get()));

            let work = work.clone();
            let obj = self.clone();
            row.connect_activated(move |_: &ActivatableRow| {
                obj.select(work.clone());
            });

            imp.work_list.append(&row);
        }

        let create_box = gtk::Box::builder().spacing(12).build();
        create_box.append(&gtk::Image::builder().icon_name("list-add-symbolic").build());
        create_box.append(
            &gtk::Label::builder()
                .label(gettext("Create new work"))
                .halign(gtk::Align::Start)
                .build(),
        );

        let create_row = ActivatableRow::new(&create_box);
        let obj = self.clone();
        create_row.connect_activated(move |_: &ActivatableRow| {
            obj.create();
        });

        imp.work_list.append(&create_row);

        imp.works.replace(works);
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

    fn select(&self, work: Work) {
        self.emit_by_name::<()>("selected", &[&work]);
        self.popdown();
    }

    fn create(&self) {
        self.emit_by_name::<()>("create", &[]);
        self.popdown();
    }
}
