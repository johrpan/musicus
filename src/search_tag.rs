use adw::{glib, glib::subclass::Signal, prelude::*, subclass::prelude::*};
use once_cell::sync::Lazy;
use std::cell::OnceCell;

use crate::db::models::{Ensemble, Person, Work};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/search_tag.blp")]
    pub struct MusicusSearchTag {
        #[template_child]
        pub label: TemplateChild<gtk::Label>,
        pub tag: OnceCell<Tag>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MusicusSearchTag {
        const NAME: &'static str = "MusicusSearchTag";
        type Type = super::MusicusSearchTag;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for MusicusSearchTag {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> =
                Lazy::new(|| vec![Signal::builder("remove").build()]);

            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for MusicusSearchTag {}
    impl BoxImpl for MusicusSearchTag {}
}

glib::wrapper! {
    pub struct MusicusSearchTag(ObjectSubclass<imp::MusicusSearchTag>)
        @extends gtk::Widget;
}

#[gtk::template_callbacks]
impl MusicusSearchTag {
    pub fn new(tag: Tag) -> Self {
        let obj: MusicusSearchTag = glib::Object::new();

        obj.imp().label.set_label(match &tag {
            Tag::Composer(person) => person.name.get(),
            Tag::Performer(person) => person.name.get(),
            Tag::Ensemble(ensemble) => ensemble.name.get(),
            Tag::Work(work) => work.name.get(),
        });

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
    Work(Work),
}
