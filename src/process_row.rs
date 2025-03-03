use std::cell::OnceCell;

use formatx::formatx;
use gettextrs::gettext;
use gtk::{
    glib::{self, subclass::Signal, Properties},
    prelude::*,
    subclass::prelude::*,
};
use once_cell::sync::Lazy;

use crate::process::Process;

mod imp {
    use super::*;

    #[derive(Properties, Debug, Default, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::ProcessRow)]
    #[template(file = "data/ui/process_row.blp")]
    pub struct ProcessRow {
        #[property(get, construct_only)]
        pub process: OnceCell<Process>,

        #[template_child]
        pub description_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub success_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub error_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub remove_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub progress_bar: TemplateChild<gtk::ProgressBar>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProcessRow {
        const NAME: &'static str = "MusicusProcessRow";
        type Type = super::ProcessRow;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for ProcessRow {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> =
                Lazy::new(|| vec![Signal::builder("remove").build()]);

            SIGNALS.as_ref()
        }

        fn constructed(&self) {
            self.parent_constructed();

            self.description_label
                .set_label(&self.obj().process().description());

            self.obj()
                .process()
                .bind_property("progress", &*self.progress_bar, "fraction")
                .build();

            let obj = self.obj().to_owned();
            self.obj().process().connect_finished_notify(move |_| {
                obj.update();
            });

            self.obj().update();
        }
    }

    impl WidgetImpl for ProcessRow {}
    impl ListBoxRowImpl for ProcessRow {}
}

glib::wrapper! {
    pub struct ProcessRow(ObjectSubclass<imp::ProcessRow>)
        @extends gtk::Widget, gtk::ListBoxRow;
}

#[gtk::template_callbacks]
impl ProcessRow {
    pub fn new(process: &Process) -> Self {
        glib::Object::builder().property("process", process).build()
    }

    pub fn connect_remove<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("remove", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            f(&obj);
            None
        })
    }

    #[template_callback]
    fn remove(&self) {
        self.emit_by_name::<()>("remove", &[]);
    }

    fn update(&self) {
        if !self.process().finished() {
            self.imp()
                .progress_bar
                .set_fraction(self.process().progress());
        } else {
            self.imp().progress_bar.set_visible(false);
            self.imp().remove_button.set_visible(true);

            if let Some(error) = self.process().error() {
                self.imp()
                    .error_label
                    .set_label(&formatx!(gettext("Process failed: {}"), error).unwrap());
                self.imp().error_label.set_visible(true);
            } else {
                self.imp().success_label.set_visible(true);
            }
        }
    }
}
