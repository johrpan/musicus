use std::cell::OnceCell;

use gtk::{glib, prelude::*, subclass::prelude::*};

use crate::search_tag::Tag;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/tag_tile.blp")]
    pub struct TagTile {
        #[template_child]
        pub title_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub subtitle_label: TemplateChild<gtk::Label>,

        pub tag: OnceCell<Tag>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TagTile {
        const NAME: &'static str = "MusicusTagTile";
        type Type = super::TagTile;
        type ParentType = gtk::FlowBoxChild;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for TagTile {}
    impl WidgetImpl for TagTile {}
    impl FlowBoxChildImpl for TagTile {}
}

glib::wrapper! {
    pub struct TagTile(ObjectSubclass<imp::TagTile>)
        @extends gtk::Widget, gtk::FlowBoxChild;
}

impl TagTile {
    pub fn new(tag: Tag) -> Self {
        let obj: Self = glib::Object::new();
        let imp = obj.imp();

        match &tag {
            Tag::Composer(person) | Tag::Performer(person) => {
                imp.title_label.set_label(person.name.get());
            }
            Tag::Ensemble(ensemble) => {
                imp.title_label.set_label(ensemble.name.get());
            }
            Tag::Instrument(instrument) => {
                imp.title_label.set_label(instrument.name.get());
            }
            Tag::Work(work) => {
                imp.title_label.set_label(work.name.get());
                if let Some(composers) = work.composers_string() {
                    imp.subtitle_label.set_label(&composers);
                    imp.subtitle_label.set_visible(true);
                } else {
                    imp.subtitle_label.set_visible(false);
                }
            }
        }

        imp.tag.set(tag).unwrap();

        obj
    }

    pub fn tag(&self) -> &Tag {
        self.imp().tag.get().unwrap()
    }
}
