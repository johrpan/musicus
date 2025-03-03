use std::{cell::OnceCell, ffi::OsStr, path::Path};

use adw::{prelude::*, subclass::prelude::*};
use formatx::formatx;
use gettextrs::gettext;
use gtk::glib::{self, clone};

use crate::{
    library::Library, process::Process, process_manager::ProcessManager, process_row::ProcessRow,
    window::Window,
};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/library_manager.blp")]
    pub struct LibraryManager {
        pub navigation: OnceCell<adw::NavigationView>,
        pub library: OnceCell<Library>,
        pub process_manager: OnceCell<ProcessManager>,

        #[template_child]
        pub library_path_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub process_list: TemplateChild<gtk::ListBox>,
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
    pub fn new(
        navigation: &adw::NavigationView,
        library: &Library,
        process_manager: &ProcessManager,
    ) -> Self {
        let obj: Self = glib::Object::new();

        for process in process_manager.processes() {
            obj.add_process(&process);
        }

        if let Some(Some(filename)) = Path::new(&library.folder()).file_name().map(OsStr::to_str) {
            obj.imp().library_path_row.set_subtitle(filename);
        }

        obj.imp().navigation.set(navigation.to_owned()).unwrap();
        obj.imp().library.set(library.to_owned()).unwrap();
        obj.imp()
            .process_manager
            .set(process_manager.to_owned())
            .unwrap();

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
    async fn import_archive(&self) {
        let dialog = gtk::FileDialog::builder()
            .title(gettext("Import from library archive"))
            .modal(true)
            .build();

        let root = self.root();
        let window = root
            .as_ref()
            .and_then(|r| r.downcast_ref::<gtk::Window>())
            .and_then(|w| w.downcast_ref::<Window>())
            .unwrap();

        match dialog.open_future(Some(window)).await {
            Err(err) => {
                if !err.matches(gtk::DialogError::Dismissed) {
                    log::error!("File selection failed: {err}");
                }
            }
            Ok(path) => {
                if let Some(path) = path.path() {
                    match self.imp().library.get().unwrap().import(&path) {
                        Ok(receiver) => {
                            let process = Process::new(
                                &formatx!(
                                    gettext("Importing music library from {}"),
                                    path.file_name()
                                        .map(|f| f.to_string_lossy().into_owned())
                                        .unwrap_or(gettext("archive"))
                                )
                                .unwrap(),
                                receiver,
                            );

                            process.connect_finished_notify(clone!(
                                #[weak(rename_to = obj)]
                                self,
                                move |_| {
                                    obj.imp().library.get().unwrap().changed();
                                }
                            ));

                            self.imp()
                                .process_manager
                                .get()
                                .unwrap()
                                .add_process(&process);

                            self.add_process(&process);
                        }
                        Err(err) => log::error!("Failed to import library: {err}"),
                    }
                }
            }
        }
    }

    #[template_callback]
    async fn export_archive(&self) {
        let dialog = gtk::FileDialog::builder()
            .title(gettext("Export library"))
            .modal(true)
            .build();

        let root = self.root();
        let window = root
            .as_ref()
            .and_then(|r| r.downcast_ref::<gtk::Window>())
            .and_then(|w| w.downcast_ref::<Window>())
            .unwrap();

        match dialog.save_future(Some(window)).await {
            Err(err) => {
                if !err.matches(gtk::DialogError::Dismissed) {
                    log::error!("File selection failed: {err}");
                }
            }
            Ok(path) => {
                if let Some(path) = path.path() {
                    match self.imp().library.get().unwrap().export(&path) {
                        Ok(receiver) => {
                            let process = Process::new(
                                &formatx!(
                                    gettext("Exporting music library to {}"),
                                    path.file_name()
                                        .map(|f| f.to_string_lossy().into_owned())
                                        .unwrap_or(gettext("archive"))
                                )
                                .unwrap(),
                                receiver,
                            );

                            self.imp()
                                .process_manager
                                .get()
                                .unwrap()
                                .add_process(&process);

                            self.add_process(&process);
                        }
                        Err(err) => log::error!("Failed to export library: {err}"),
                    }
                }
            }
        }
    }

    fn add_process(&self, process: &Process) {
        let row = ProcessRow::new(process);

        row.connect_remove(clone!(
            #[weak(rename_to = obj)]
            self,
            move |row| {
                obj.imp()
                    .process_manager
                    .get()
                    .unwrap()
                    .remove_process(&row.process());

                obj.imp().process_list.remove(row);

                if obj.imp().process_list.first_child().is_none() {
                    obj.imp().process_list.set_visible(false);
                }
            }
        ));

        self.imp().process_list.append(&row);
        self.imp().process_list.set_visible(true);
    }
}
