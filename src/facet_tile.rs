use std::cell::OnceCell;

use gtk::{glib, prelude::*, subclass::prelude::*};

use crate::library::Facet;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/facet_tile.blp")]
    pub struct FacetTile {
        #[template_child]
        pub title_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub subtitle_label: TemplateChild<gtk::Label>,

        pub facet: OnceCell<Facet>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FacetTile {
        const NAME: &'static str = "MusicusFacetTile";
        type Type = super::FacetTile;
        type ParentType = gtk::FlowBoxChild;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for FacetTile {}
    impl WidgetImpl for FacetTile {}
    impl FlowBoxChildImpl for FacetTile {}
}

glib::wrapper! {
    pub struct FacetTile(ObjectSubclass<imp::FacetTile>)
        @extends gtk::FlowBoxChild, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl FacetTile {
    pub fn new(facet: Facet) -> Self {
        let obj: Self = glib::Object::new();
        let imp = obj.imp();

        match &facet {
            Facet::Composer(person) | Facet::Performer(person) => {
                imp.title_label.set_label(person.name.get());
            }
            Facet::Ensemble(ensemble) => {
                imp.title_label.set_label(ensemble.name.get());
                if let Some(members) = ensemble.members_string() {
                    imp.subtitle_label.set_label(&members);
                    imp.subtitle_label.set_visible(true);
                } else {
                    imp.subtitle_label.set_visible(false);
                }
            }
            Facet::Instrument(instrument) => {
                imp.title_label.set_label(instrument.name.get());
            }
            Facet::Tag(tag_value) => match &tag_value.value {
                Some(value) => {
                    imp.title_label.set_label(value);
                    imp.subtitle_label.set_label(tag_value.tag.name.get());
                    imp.subtitle_label.set_visible(true);
                }
                None => {
                    imp.title_label.set_label(tag_value.tag.name.get());
                    imp.subtitle_label.set_visible(false);
                }
            },
            Facet::Work(work) => {
                imp.title_label.set_label(work.name.get());
                if let Some(composers) = work.composers_string() {
                    imp.subtitle_label.set_label(&composers);
                    imp.subtitle_label.set_visible(true);
                } else {
                    imp.subtitle_label.set_visible(false);
                }
            }
        }

        imp.facet.set(facet).unwrap();

        obj
    }

    pub fn facet(&self) -> &Facet {
        self.imp().facet.get().unwrap()
    }
}
