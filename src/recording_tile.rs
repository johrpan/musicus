use gtk::{gio, glib, prelude::*, subclass::prelude::*};
use std::cell::OnceCell;

use crate::{
    db::models::Recording, editor::recording_editor::MusicusRecordingEditor,
    library::MusicusLibrary,
};

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

        pub navigation: OnceCell<adw::NavigationView>,
        pub library: OnceCell<MusicusLibrary>,
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

    impl ObjectImpl for MusicusRecordingTile {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj().to_owned();
            let edit_action = gio::ActionEntry::builder("edit")
                .activate(move |_, _, _| {
                    obj.imp()
                        .navigation
                        .get()
                        .unwrap()
                        .push(&MusicusRecordingEditor::new(
                            obj.imp().navigation.get().unwrap(),
                            obj.imp().library.get().unwrap(),
                            Some(&obj.imp().recording.get().unwrap()),
                        ));
                })
                .build();

            let actions = gio::SimpleActionGroup::new();
            actions.add_action_entries([edit_action]);
            self.obj().insert_action_group("recording", Some(&actions));
        }
    }

    impl WidgetImpl for MusicusRecordingTile {}
    impl FlowBoxChildImpl for MusicusRecordingTile {}
}

glib::wrapper! {
    pub struct MusicusRecordingTile(ObjectSubclass<imp::MusicusRecordingTile>)
        @extends gtk::Widget, gtk::FlowBoxChild;
}

impl MusicusRecordingTile {
    pub fn new(
        navigation: &adw::NavigationView,
        library: &MusicusLibrary,
        recording: &Recording,
    ) -> Self {
        let obj: Self = glib::Object::new();
        let imp = obj.imp();

        imp.work_label.set_label(&recording.work.name.get());
        imp.composer_label
            .set_label(&recording.work.composers_string());
        imp.performances_label
            .set_label(&recording.performers_string());

        imp.navigation.set(navigation.to_owned()).unwrap();
        imp.library.set(library.to_owned()).unwrap();
        imp.recording.set(recording.to_owned()).unwrap();

        obj
    }

    pub fn recording(&self) -> &Recording {
        self.imp().recording.get().unwrap()
    }
}
