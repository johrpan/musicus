use crate::library::{Ensemble, Person, Work};
use adw::{glib, glib::subclass::Signal, prelude::*, subclass::prelude::*};
use once_cell::sync::Lazy;
use std::cell::OnceCell;

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

        obj.imp().label.set_label(&match &tag {
            Tag::Composer(person) => person.name_fl(),
            Tag::Performer(person) => person.name_fl(),
            Tag::Ensemble(ensemble) => ensemble.name.clone(),
            Tag::Work(work) => work.title.clone(),
        });

        obj.imp().tag.set(tag).unwrap();

        obj
    }

    pub fn tag(&self) -> &Tag {
        self.imp().tag.get().unwrap()
    }

    #[template_callback]
    fn remove(&self, _: &gtk::Button) {
        self.emit_by_name::<()>("remove", &[]);
    }
}

#[derive(Debug, Clone)]
pub enum Tag {
    Composer(Person),
    Performer(Person),
    Ensemble(Ensemble),
    Work(Work),
}
