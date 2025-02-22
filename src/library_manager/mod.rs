pub mod albums_page;

use crate::{
    db::{
        models::{Album, Ensemble, Instrument, Person, Recording, Role, Track, Work},
        tables::Medium,
    },
    library::MusicusLibrary,
    window::MusicusWindow,
};

use adw::{prelude::*, subclass::prelude::*};
use albums_page::AlbumsPage;
use gettextrs::gettext;
use gtk::glib;

use std::{
    cell::{OnceCell, RefCell},
    ffi::OsStr,
    path::Path,
};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/library_manager.blp")]
    pub struct LibraryManager {
        pub navigation: OnceCell<adw::NavigationView>,
        pub library: OnceCell<MusicusLibrary>,

        pub persons: RefCell<Vec<Person>>,
        pub roles: RefCell<Vec<Role>>,
        pub instruments: RefCell<Vec<Instrument>>,
        pub works: RefCell<Vec<Work>>,
        pub ensembles: RefCell<Vec<Ensemble>>,
        pub recordings: RefCell<Vec<Recording>>,
        pub tracks: RefCell<Vec<Track>>,
        pub mediums: RefCell<Vec<Medium>>,
        pub albums: RefCell<Vec<Album>>,

        #[template_child]
        pub library_path_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub n_persons_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub n_roles_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub n_instruments_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub n_works_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub n_ensembles_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub n_recordings_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub n_tracks_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub n_mediums_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub n_albums_label: TemplateChild<gtk::Label>,
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

    impl NavigationPageImpl for LibraryManager {
        fn showing(&self) {
            self.parent_showing();
            self.obj().update();
        }
    }
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
    async fn open_library(&self) {
        let dialog = gtk::FileDialog::builder()
            .title(gettext("Select music library folder"))
            .modal(true)
            .build();

        let root = self.root();
        let window = root
            .as_ref()
            .and_then(|r| r.downcast_ref::<gtk::Window>())
            .and_then(|w| w.downcast_ref::<MusicusWindow>())
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

    #[template_callback]
    fn show_persons(&self) {}

    #[template_callback]
    fn show_roles(&self) {}

    #[template_callback]
    fn show_instruments(&self) {}

    #[template_callback]
    fn show_works(&self) {}

    #[template_callback]
    fn show_ensembles(&self) {}

    #[template_callback]
    fn show_recordings(&self) {}

    #[template_callback]
    fn show_tracks(&self) {}

    #[template_callback]
    fn show_mediums(&self) {}

    #[template_callback]
    fn show_albums(&self) {
        let navigation = self.imp().navigation.get().unwrap();
        let library = self.imp().library.get().unwrap();
        navigation.push(&AlbumsPage::new(navigation, library));
    }

    // TODO: Make this async.
    fn update(&self) {
        let library = self.imp().library.get().unwrap();

        if let Some(Some(filename)) = Path::new(&library.folder()).file_name().map(OsStr::to_str) {
            self.imp().library_path_row.set_subtitle(filename);
        }

        let persons = library.all_persons().unwrap();
        self.imp()
            .n_persons_label
            .set_label(&persons.len().to_string());
        self.imp().persons.replace(persons);

        let roles = library.all_roles().unwrap();
        self.imp().n_roles_label.set_label(&roles.len().to_string());
        self.imp().roles.replace(roles);

        let instruments = library.all_instruments().unwrap();
        self.imp()
            .n_instruments_label
            .set_label(&instruments.len().to_string());
        self.imp().instruments.replace(instruments);

        let works = library.all_works().unwrap();
        self.imp().n_works_label.set_label(&works.len().to_string());
        self.imp().works.replace(works);

        let ensembles = library.all_ensembles().unwrap();
        self.imp()
            .n_ensembles_label
            .set_label(&ensembles.len().to_string());
        self.imp().ensembles.replace(ensembles);

        let recordings = library.all_recordings().unwrap();
        self.imp()
            .n_recordings_label
            .set_label(&recordings.len().to_string());
        self.imp().recordings.replace(recordings);

        let tracks = library.all_tracks().unwrap();
        self.imp()
            .n_tracks_label
            .set_label(&tracks.len().to_string());
        self.imp().tracks.replace(tracks);

        let mediums = library.all_mediums().unwrap();
        self.imp()
            .n_mediums_label
            .set_label(&mediums.len().to_string());
        self.imp().mediums.replace(mediums);

        let albums = library.all_albums().unwrap();
        self.imp()
            .n_albums_label
            .set_label(&albums.len().to_string());
        self.imp().albums.replace(albums);
    }
}
