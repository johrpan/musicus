use crate::search_tag::Tag;
use gtk::{glib, prelude::*, subclass::prelude::*};
use std::cell::OnceCell;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/tag_tile.blp")]
    pub struct MusicusTagTile {
        #[template_child]
        pub title_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub subtitle_label: TemplateChild<gtk::Label>,

        pub tag: OnceCell<Tag>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MusicusTagTile {
        const NAME: &'static str = "MusicusTagTile";
        type Type = super::MusicusTagTile;
        type ParentType = gtk::FlowBoxChild;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for MusicusTagTile {}
    impl WidgetImpl for MusicusTagTile {}
    impl FlowBoxChildImpl for MusicusTagTile {}
}

glib::wrapper! {
    pub struct MusicusTagTile(ObjectSubclass<imp::MusicusTagTile>)
        @extends gtk::Widget, gtk::FlowBoxChild;
}

impl MusicusTagTile {
    pub fn new(tag: Tag) -> Self {
        let obj: Self = glib::Object::new();
        let imp = obj.imp();

        match &tag {
            Tag::Composer(person) | Tag::Performer(person) => {
                imp.title_label.set_label(&person.name_fl());
            }
            Tag::Ensemble(ensemble) => {
                imp.title_label.set_label(&ensemble.name);
            }
            Tag::Work(work) => {
                imp.title_label.set_label(&work.title);
                imp.subtitle_label.set_label(&work.composer.name_fl());
                imp.subtitle_label.set_visible(true);
            }
        }

        imp.tag.set(tag).unwrap();

        obj
    }

    pub fn tag(&self) -> &Tag {
        self.imp().tag.get().unwrap()
    }
}
