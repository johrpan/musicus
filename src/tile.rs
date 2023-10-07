use gtk::{glib, glib::Properties, prelude::*, subclass::prelude::*};
use std::cell::RefCell;

mod imp {
    use super::*;

    #[derive(Properties, Debug, Default, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::MusicusTile)]
    #[template(file = "data/ui/tile.blp")]
    pub struct MusicusTile {
        #[property(get, set)]
        pub title: RefCell<String>,

        #[property(get, set)]
        pub subtitle: RefCell<Option<String>>,

        #[template_child]
        pub title_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub subtitle_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MusicusTile {
        const NAME: &'static str = "MusicusTile";
        type Type = super::MusicusTile;
        type ParentType = gtk::FlowBoxChild;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for MusicusTile {
        fn constructed(&self) {
            self.parent_constructed();

            self.obj()
                .bind_property("title", &self.title_label.get(), "label")
                .sync_create()
                .build();

            self.obj()
                .bind_property("subtitle", &self.subtitle_label.get(), "visible")
                .sync_create()
                .transform_to(|_, s: Option<String>| Some(s.is_some()))
                .build();

            self.obj()
                .bind_property("subtitle", &self.subtitle_label.get(), "label")
                .sync_create()
                .build();
        }
    }

    impl WidgetImpl for MusicusTile {}
    impl FlowBoxChildImpl for MusicusTile {}
}

glib::wrapper! {
    pub struct MusicusTile(ObjectSubclass<imp::MusicusTile>)
        @extends gtk::Widget, gtk::FlowBoxChild;
}

impl MusicusTile {
    pub fn with_title(title: &str) -> Self {
        glib::Object::builder().property("title", title).build()
    }

    pub fn with_subtitle(title: &str, subtitle: &str) -> Self {
        glib::Object::builder()
            .property("title", title)
            .property("subtitle", subtitle)
            .build()
    }
}
