use crate::library::{Performance, Recording};
use gtk::{glib, subclass::prelude::*};
use std::cell::OnceCell;

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
    pub fn new(recording: &Recording, performances: Vec<Performance>) -> Self {
        let obj: Self = glib::Object::new();
        let imp = obj.imp();

        imp.work_label.set_label(&recording.work.title);
        imp.composer_label
            .set_label(&recording.work.composer.name_fl());

        imp.performances_label.set_label(
            &performances
                .into_iter()
                .map(|performance| match performance {
                    Performance::Person(person, role) => {
                        let mut result = person.name_fl();
                        if let Some(role) = role {
                            result.push_str(&format!(" ({})", role.name));
                        }
                        result
                    }
                    Performance::Ensemble(ensemble, role) => {
                        let mut result = ensemble.name;
                        if let Some(role) = role {
                            result.push_str(&format!(" ({})", role.name));
                        }
                        result
                    }
                })
                .collect::<Vec<String>>()
                .join(", "),
        );

        imp.recording.set(recording.clone()).unwrap();

        obj
    }

    pub fn recording(&self) -> &Recording {
        self.imp().recording.get().unwrap()
    }
}
