use std::{cell::OnceCell, ffi::OsStr, path::Path};

use adw::{prelude::*, subclass::prelude::*};
use gettextrs::gettext;
use gtk::glib;

use crate::{library::Library, window::Window};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/library_manager.blp")]
    pub struct LibraryManager {
        pub navigation: OnceCell<adw::NavigationView>,
        pub library: OnceCell<Library>,

        #[template_child]
        pub library_path_row: TemplateChild<adw::ActionRow>,
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
    pub fn new(navigation: &adw::NavigationView, library: &Library) -> Self {
        let obj: Self = glib::Object::new();

        obj.imp().navigation.set(navigation.to_owned()).unwrap();
        obj.imp().library.set(library.to_owned()).unwrap();

        if let Some(Some(filename)) = Path::new(&library.folder()).file_name().map(OsStr::to_str) {
            obj.imp().library_path_row.set_subtitle(filename);
        }

        obj
    }

    #[template_callback]
    async fn open_library(&self) {
        let dialog = gtk::FileDialog::builder()
            .title(gettext("Select music library folder"))
            .modal(true)
            .build();

        let root = self.root();
        let window = root
            .as_ref()
            .and_then(|r| r.downcast_ref::<gtk::Window>())
            .and_then(|w| w.downcast_ref::<Window>())
            .unwrap();

        match dialog.select_folder_future(Some(window)).await {
            Err(err) => {
                if !err.matches(gtk::DialogError::Dismissed) {
                    log::error!("Folder selection failed: {err}");
                }
            }
            Ok(folder) => window.set_library_folder(&folder),
        }
    }

    #[template_callback]
    fn import_archive(&self) {}

    #[template_callback]
    fn export_archive(&self) {}
}
