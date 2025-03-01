use std::cell::OnceCell;

use adw::{glib, glib::subclass::Signal, prelude::*, subclass::prelude::*};
use once_cell::sync::Lazy;

use crate::db::models::{Ensemble, Instrument, Person, Work};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/search_tag.blp")]
    pub struct SearchTag {
        #[template_child]
        pub label: TemplateChild<gtk::Label>,
        pub tag: OnceCell<Tag>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SearchTag {
        const NAME: &'static str = "MusicusSearchTag";
        type Type = super::SearchTag;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SearchTag {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> =
                Lazy::new(|| vec![Signal::builder("remove").build()]);

            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for SearchTag {}
    impl BoxImpl for SearchTag {}
}

glib::wrapper! {
    pub struct SearchTag(ObjectSubclass<imp::SearchTag>)
        @extends gtk::Widget;
}

#[gtk::template_callbacks]
impl SearchTag {
    pub fn new(tag: Tag) -> Self {
        let obj: SearchTag = glib::Object::new();

        let label = match &tag {
            Tag::Composer(person) => person.name.get(),
            Tag::Performer(person) => person.name.get(),
            Tag::Ensemble(ensemble) => ensemble.name.get(),
            Tag::Instrument(instrument) => instrument.name.get(),
            Tag::Work(work) => work.name.get(),
        };

        obj.imp().label.set_label(label);
        obj.set_tooltip_text(Some(label));
        obj.imp().tag.set(tag).unwrap();

        obj
    }

    pub fn connect_remove<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("remove", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            f(&obj);
            None
        })
    }

    pub fn tag(&self) -> &Tag {
        self.imp().tag.get().unwrap()
    }

    #[template_callback]
    fn remove(&self) {
        self.emit_by_name::<()>("remove", &[]);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tag {
    Composer(Person),
    Performer(Person),
    Ensemble(Ensemble),
    Instrument(Instrument),
    Work(Work),
}
