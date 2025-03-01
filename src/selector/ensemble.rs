use std::cell::{OnceCell, RefCell};

use gettextrs::gettext;
use gtk::{
    glib::{self, subclass::Signal, Properties},
    prelude::*,
    subclass::prelude::*,
};
use once_cell::sync::Lazy;

use crate::{activatable_row::ActivatableRow, db::models::Ensemble, library::Library};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, Properties)]
    #[properties(wrapper_type = super::EnsembleSelectorPopover)]
    #[template(file = "data/ui/selector/ensemble.blp")]
    pub struct EnsembleSelectorPopover {
        #[property(get, construct_only)]
        pub library: OnceCell<Library>,

        pub ensembles: RefCell<Vec<Ensemble>>,

        #[template_child]
        pub search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub scrolled_window: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub list_box: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EnsembleSelectorPopover {
        const NAME: &'static str = "MusicusEnsembleSelectorPopover";
        type Type = super::EnsembleSelectorPopover;
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
    impl ObjectImpl for EnsembleSelectorPopover {
        fn constructed(&self) {
            self.parent_constructed();

            self.obj().connect_visible_notify(|obj| {
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
                vec![
                    Signal::builder("ensemble-selected")
                        .param_types([Ensemble::static_type()])
                        .build(),
                    Signal::builder("create").build(),
                ]
            });

            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for EnsembleSelectorPopover {
        // TODO: Fix focus.
        fn focus(&self, direction_type: gtk::DirectionType) -> bool {
            if direction_type == gtk::DirectionType::Down {
                self.list_box.child_focus(direction_type)
            } else {
                self.parent_focus(direction_type)
            }
        }
    }

    impl PopoverImpl for EnsembleSelectorPopover {}
}

glib::wrapper! {
    pub struct EnsembleSelectorPopover(ObjectSubclass<imp::EnsembleSelectorPopover>)
        @extends gtk::Widget, gtk::Popover;
}

#[gtk::template_callbacks]
impl EnsembleSelectorPopover {
    pub fn new(library: &Library) -> Self {
        glib::Object::builder().property("library", library).build()
    }

    pub fn connect_ensemble_selected<F: Fn(&Self, Ensemble) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("ensemble-selected", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let ensemble = values[1].get::<Ensemble>().unwrap();
            f(&obj, ensemble);
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
    fn search_changed(&self, entry: &gtk::SearchEntry) {
        self.search(&entry.text());
    }

    #[template_callback]
    fn activate(&self, _: &gtk::SearchEntry) {
        if let Some(ensemble) = self.imp().ensembles.borrow().first() {
            self.select(ensemble.clone());
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

        let ensembles = imp.library.get().unwrap().search_ensembles(search).unwrap();

        imp.list_box.remove_all();

        for ensemble in &ensembles {
            let row = ActivatableRow::new(
                &gtk::Label::builder()
                    .label(ensemble.to_string())
                    .halign(gtk::Align::Start)
                    .build(),
            );

            row.set_tooltip_text(Some(&ensemble.to_string()));

            let ensemble = ensemble.clone();
            let obj = self.clone();
            row.connect_activated(move |_: &ActivatableRow| {
                obj.select(ensemble.clone());
            });

            imp.list_box.append(&row);
        }

        let create_box = gtk::Box::builder().spacing(12).build();
        create_box.append(&gtk::Image::builder().icon_name("list-add-symbolic").build());
        create_box.append(
            &gtk::Label::builder()
                .label(gettext("Create new ensemble"))
                .halign(gtk::Align::Start)
                .build(),
        );

        let create_row = ActivatableRow::new(&create_box);
        let obj = self.clone();
        create_row.connect_activated(move |_: &ActivatableRow| {
            obj.create();
        });

        imp.list_box.append(&create_row);

        imp.ensembles.replace(ensembles);
    }

    fn select(&self, ensemble: Ensemble) {
        self.emit_by_name::<()>("ensemble-selected", &[&ensemble]);
        self.popdown();
    }

    fn create(&self) {
        self.emit_by_name::<()>("create", &[]);
        self.popdown();
    }
}
