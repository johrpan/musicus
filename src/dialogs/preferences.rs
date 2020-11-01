use crate::backend::Backend;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use libhandy::prelude::*;
use std::rc::Rc;

pub struct Preferences {
    window: gtk::Window,
}

impl Preferences {
    pub fn new<P: IsA<gtk::Window>>(backend: Rc<Backend>, parent: &P) -> Self {
        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/preferences.ui");

        get_widget!(builder, gtk::Window, window);
        get_widget!(builder, libhandy::ActionRow, music_library_path_row);
        get_widget!(builder, gtk::Button, select_music_library_path_button);

        window.set_transient_for(Some(parent));

        if let Some(path) = backend.get_music_library_path() {
            music_library_path_row.set_subtitle(Some(path.to_str().unwrap()));
        }

        select_music_library_path_button.connect_clicked(clone!(@strong window, @strong backend, @strong music_library_path_row => move |_| {
            let dialog = gtk::FileChooserNative::new(Some("Select music library folder"), Some(&window), gtk::FileChooserAction::SelectFolder, None, None);

            if let gtk::ResponseType::Accept = dialog.run() {
                if let Some(path) = dialog.get_filename() {
                    music_library_path_row.set_subtitle(Some(path.to_str().unwrap()));
                    
                    let context = glib::MainContext::default();
                    let backend = backend.clone();
                    context.spawn_local(async move {
                        backend.set_music_library_path(path).await.unwrap();
                    });
                }
            }
        }));

        Self { window }
    }

    pub fn show(&self) {
        self.window.show();
    }
}
