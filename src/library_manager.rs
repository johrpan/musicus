use adw::subclass::prelude::*;
use gtk::glib;
use std::cell::OnceCell;

use crate::{
    editor::{
        ensemble_editor::MusicusEnsembleEditor, instrument_editor::MusicusInstrumentEditor,
        person_editor::MusicusPersonEditor, recording_editor::MusicusRecordingEditor,
        role_editor::MusicusRoleEditor, work_editor::MusicusWorkEditor,
    },
    library::MusicusLibrary,
};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/library_manager.blp")]
    pub struct LibraryManager {
        pub navigation: OnceCell<adw::NavigationView>,
        pub library: OnceCell<MusicusLibrary>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LibraryManager {
        const NAME: &'static str = "MusicusLibraryManager";
        type Type = super::LibraryManager;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for LibraryManager {}
    impl WidgetImpl for LibraryManager {}
    impl NavigationPageImpl for LibraryManager {}
}

glib::wrapper! {
    pub struct LibraryManager(ObjectSubclass<imp::LibraryManager>)
        @extends gtk::Widget, adw::NavigationPage;
}

#[gtk::template_callbacks]
impl LibraryManager {
    pub fn new(navigation: &adw::NavigationView, library: &MusicusLibrary) -> Self {
        let obj: Self = glib::Object::new();
        let imp = obj.imp();

        imp.navigation.set(navigation.to_owned()).unwrap();
        imp.library.set(library.to_owned()).unwrap();

        obj
    }

    #[template_callback]
    fn add_person(&self, _: &gtk::Button) {
        self.imp()
            .navigation
            .get()
            .unwrap()
            .push(&MusicusPersonEditor::new(
                &self.imp().navigation.get().unwrap(),
                &self.imp().library.get().unwrap(),
                None,
            ));
    }

    #[template_callback]
    fn add_role(&self, _: &gtk::Button) {
        self.imp()
            .navigation
            .get()
            .unwrap()
            .push(&MusicusRoleEditor::new(
                &self.imp().navigation.get().unwrap(),
                &self.imp().library.get().unwrap(),
                None,
            ));
    }

    #[template_callback]
    fn add_instrument(&self, _: &gtk::Button) {
        self.imp()
            .navigation
            .get()
            .unwrap()
            .push(&MusicusInstrumentEditor::new(
                &self.imp().navigation.get().unwrap(),
                &self.imp().library.get().unwrap(),
                None,
            ));
    }

    #[template_callback]
    fn add_work(&self, _: &gtk::Button) {
        self.imp()
            .navigation
            .get()
            .unwrap()
            .push(&MusicusWorkEditor::new(
                &self.imp().navigation.get().unwrap(),
                &self.imp().library.get().unwrap(),
                None,
            ));
    }

    #[template_callback]
    fn add_ensemble(&self, _: &gtk::Button) {
        self.imp()
            .navigation
            .get()
            .unwrap()
            .push(&MusicusEnsembleEditor::new(
                &self.imp().navigation.get().unwrap(),
                &self.imp().library.get().unwrap(),
                None,
            ));
    }

    #[template_callback]
    fn add_recording(&self, _: &gtk::Button) {
        self.imp()
            .navigation
            .get()
            .unwrap()
            .push(&MusicusRecordingEditor::new(
                &self.imp().navigation.get().unwrap(),
                &self.imp().library.get().unwrap(),
                None,
            ));
    }

    #[template_callback]
    fn add_medium(&self, _: &gtk::Button) {
        todo!("Medium import");
    }

    #[template_callback]
    fn add_album(&self, _: &gtk::Button) {
        todo!("Album editor");
        // self.imp()
        //     .navigation
        //     .get()
        //     .unwrap()
        //     .push(&MusicusAlbumEditor::new(
        //         &self.imp().navigation.get().unwrap(),
        //         &self.imp().library.get().unwrap(),
        //         None,
        //     ));
    }
}
