use gtk::{glib, subclass::prelude::*};
use std::cell::OnceCell;

use crate::db::models::Recording;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/recording_tile.blp")]
    pub struct MusicusRecordingTile {
        #[template_child]
        pub composer_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub work_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub performances_label: TemplateChild<gtk::Label>,

        pub recording: OnceCell<Recording>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MusicusRecordingTile {
        const NAME: &'static str = "MusicusRecordingTile";
        type Type = super::MusicusRecordingTile;
        type ParentType = gtk::FlowBoxChild;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for MusicusRecordingTile {}
    impl WidgetImpl for MusicusRecordingTile {}
    impl FlowBoxChildImpl for MusicusRecordingTile {}
}

glib::wrapper! {
    pub struct MusicusRecordingTile(ObjectSubclass<imp::MusicusRecordingTile>)
        @extends gtk::Widget, gtk::FlowBoxChild;
}

impl MusicusRecordingTile {
    pub fn new(recording: &Recording) -> Self {
        let obj: Self = glib::Object::new();
        let imp = obj.imp();

        imp.work_label.set_label(&recording.work.name.get());
        imp.composer_label.set_label(&recording.work.composers_string());
        imp.performances_label.set_label(&recording.performers_string());

        imp.recording.set(recording.clone()).unwrap();

        obj
    }

    pub fn recording(&self) -> &Recording {
        self.imp().recording.get().unwrap()
    }
}
