use adw::prelude::*;
use gettextrs::gettext;
use glib::clone;
use gtk_macros::get_widget;
use musicus_backend::Backend;
use std::rc::Rc;

/// A dialog for configuring the app.
pub struct Preferences {
    backend: Rc<Backend>,
    window: adw::Window,
    music_library_path_row: adw::ActionRow,
}

impl Preferences {
    /// Create a new preferences dialog.
    pub fn new<P: IsA<gtk::Window>>(backend: Rc<Backend>, parent: &P) -> Rc<Self> {
        // Create UI
        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/preferences.ui");

        get_widget!(builder, adw::Window, window);
        get_widget!(builder, adw::ActionRow, music_library_path_row);
        get_widget!(builder, gtk::Button, select_music_library_path_button);

        window.set_transient_for(Some(parent));

        let this = Rc::new(Self {
            backend,
            window,
            music_library_path_row,
        });

        // Connect signals and callbacks

        select_music_library_path_button.connect_clicked(clone!(@strong this => move |_| {
            let dialog = gtk::FileChooserDialog::new(
                Some(&gettext("Select music library folder")),
                Some(&this.window),
                gtk::FileChooserAction::SelectFolder,
                &[
                    (&gettext("Cancel"), gtk::ResponseType::Cancel),
                    (&gettext("Select"), gtk::ResponseType::Accept),
                ]);

            dialog.set_modal(true);

            dialog.connect_response(clone!(@strong this => move |dialog, response| {
                if let gtk::ResponseType::Accept = response {
                    if let Some(file) = dialog.file() {
                        if let Some(path) = file.path() {
                            this.music_library_path_row.set_subtitle(path.to_str().unwrap());

                            spawn!(@clone this, async move {
                                this.backend.set_music_library_path(path).await.unwrap();
                            });
                        }
                    }
                }

                dialog.hide();
            }));

            dialog.show();
        }));

        // Initialize

        if let Some(path) = this.backend.get_music_library_path() {
            this.music_library_path_row
                .set_subtitle(path.to_str().unwrap());
        }

        this
    }

    /// Show the preferences dialog.
    pub fn show(&self) {
        self.window.show();
    }
}
