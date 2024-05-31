use crate::{db::models::Person, library::MusicusLibrary};

use gettextrs::gettext;
use gtk::{
    glib::{self, subclass::Signal, Properties},
    prelude::*,
    subclass::prelude::*,
};
use once_cell::sync::Lazy;

use std::cell::{OnceCell, RefCell};

use super::activatable_row::MusicusActivatableRow;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, Properties)]
    #[properties(wrapper_type = super::MusicusPersonSelectorPopover)]
    #[template(file = "data/ui/person_selector_popover.blp")]
    pub struct MusicusPersonSelectorPopover {
        #[property(get, construct_only)]
        pub library: OnceCell<MusicusLibrary>,

        pub persons: RefCell<Vec<Person>>,

        #[template_child]
        pub search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub scrolled_window: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub list_box: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MusicusPersonSelectorPopover {
        const NAME: &'static str = "MusicusPersonSelectorPopover";
        type Type = super::MusicusPersonSelectorPopover;
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
    impl ObjectImpl for MusicusPersonSelectorPopover {
        fn constructed(&self) {
            self.parent_constructed();

            self.obj()
                .connect_visible_notify(|obj: &super::MusicusPersonSelectorPopover| {
                    if obj.is_visible() {
                        obj.imp().search_entry.set_text("");
                        obj.imp().search_entry.grab_focus();
                        obj.imp().scrolled_window.vadjustment().set_value(0.0);
                    }
                });

            self.obj().search("");
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("person-selected")
                    .param_types([Person::static_type()])
                    .build()]
            });

            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for MusicusPersonSelectorPopover {
        // TODO: Fix focus.
        fn focus(&self, direction_type: gtk::DirectionType) -> bool {
            if direction_type == gtk::DirectionType::Down {
                self.list_box.child_focus(direction_type)
            } else {
                self.parent_focus(direction_type)
            }
        }
    }

    impl PopoverImpl for MusicusPersonSelectorPopover {}
}

glib::wrapper! {
    pub struct MusicusPersonSelectorPopover(ObjectSubclass<imp::MusicusPersonSelectorPopover>)
        @extends gtk::Widget, gtk::Popover;
}

#[gtk::template_callbacks]
impl MusicusPersonSelectorPopover {
    pub fn new(library: &MusicusLibrary) -> Self {
        glib::Object::builder().property("library", library).build()
    }

    pub fn connect_person_selected<F: Fn(&Self, Person) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("person-selected", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let person = values[1].get::<Person>().unwrap();
            f(&obj, person);
            None
        })
    }

    #[template_callback]
    fn search_changed(&self, entry: &gtk::SearchEntry) {
        self.search(&entry.text());
    }

    #[template_callback]
    fn activate(&self, _: &gtk::SearchEntry) {
        if let Some(person) = self.imp().persons.borrow().first() {
            self.select(person.clone());
        } else {
            self.create();
        }
    }

    #[template_callback]
    fn stop_search(&self, _: &gtk::SearchEntry) {
        self.popdown();
    }

    fn search(&self, search: &str) {
        let imp = self.imp();

        let persons = imp.library.get().unwrap().search_persons(search).unwrap();

        imp.list_box.remove_all();

        for person in &persons {
            let row = MusicusActivatableRow::new(
                &gtk::Label::builder()
                    .label(person.to_string())
                    .halign(gtk::Align::Start)
                    .build(),
            );

            let person = person.clone();
            let obj = self.clone();
            row.connect_activated(move |_: &MusicusActivatableRow| {
                obj.select(person.clone());
            });

            imp.list_box.append(&row);
        }

        let create_box = gtk::Box::builder().spacing(12).build();
        create_box.append(&gtk::Image::builder().icon_name("list-add-symbolic").build());
        create_box.append(
            &gtk::Label::builder()
                .label(gettext("Create new person"))
                .halign(gtk::Align::Start)
                .build(),
        );

        let create_row = MusicusActivatableRow::new(&create_box);
        let obj = self.clone();
        create_row.connect_activated(move |_: &MusicusActivatableRow| {
            obj.create();
        });

        imp.list_box.append(&create_row);

        imp.persons.replace(persons);
    }

    fn select(&self, person: Person) {
        self.emit_by_name::<()>("person-selected", &[&person]);
        self.popdown();
    }

    fn create(&self) {
        log::info!("Create person!");
        self.popdown();
    }
}
