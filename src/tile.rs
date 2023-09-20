use gtk::{glib, subclass::prelude::*};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/de/johrpan/musicus/tile.ui")]
    pub struct MusicusTile {}

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

    impl ObjectImpl for MusicusTile {}
    impl WidgetImpl for MusicusTile {}
    impl FlowBoxChildImpl for MusicusTile {}
}

glib::wrapper! {
    pub struct MusicusTile(ObjectSubclass<imp::MusicusTile>)
        @extends gtk::Widget, gtk::FlowBoxChild;
}

impl MusicusTile {
    pub fn new() -> Self {
        glib::Object::new()
    }
}
