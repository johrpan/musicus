use super::{LoginDialog, ServerDialog};
use crate::backend::Backend;
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use libhandy::prelude::*;
use std::rc::Rc;

/// A dialog for configuring the app.
pub struct Preferences {
    backend: Rc<Backend>,
    window: libhandy::Window,
    music_library_path_row: libhandy::ActionRow,
    url_row: libhandy::ActionRow,
    login_row: libhandy::ActionRow,
}

impl Preferences {
    /// Create a new preferences dialog.
    pub fn new<P: IsA<gtk::Window>>(backend: Rc<Backend>, parent: &P) -> Rc<Self> {
        // Create UI
        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/preferences.ui");

        get_widget!(builder, libhandy::Window, window);
        get_widget!(builder, libhandy::ActionRow, music_library_path_row);
        get_widget!(builder, gtk::Button, select_music_library_path_button);
        get_widget!(builder, libhandy::ActionRow, url_row);
        get_widget!(builder, gtk::Button, url_button);
        get_widget!(builder, libhandy::ActionRow, login_row);
        get_widget!(builder, gtk::Button, login_button);

        window.set_transient_for(Some(parent));

        let this = Rc::new(Self {
            backend,
            window,
            music_library_path_row,
            url_row,
            login_row,
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

            dialog.connect_response(clone!(@strong this => move |dialog, response| {
                if let gtk::ResponseType::Accept = response {
                    if let Some(file) = dialog.get_file() {
                        if let Some(path) = file.get_path() {
                            this.music_library_path_row.set_subtitle(Some(path.to_str().unwrap()));

                            let context = glib::MainContext::default();
                            let backend = this.backend.clone();
                            context.spawn_local(async move {
                                backend.set_music_library_path(path).await.unwrap();
                            });
                        }
                    }
                }
            }));

            dialog.show();
        }));

        url_button.connect_clicked(clone!(@strong this => move |_| {
            let dialog = ServerDialog::new(this.backend.clone(), &this.window);

            dialog.set_selected_cb(clone!(@strong this => move |url| {
                this.url_row.set_subtitle(Some(&url));
            }));

            dialog.show();
        }));

        login_button.connect_clicked(clone!(@strong this => move |_| {
            let dialog = LoginDialog::new(this.backend.clone(), &this.window);

            dialog.set_selected_cb(clone!(@strong this => move |data| {
                this.login_row.set_subtitle(Some(&data.username));
            }));

            dialog.show();
        }));

        // Initialize

        if let Some(path) = this.backend.get_music_library_path() {
            this.music_library_path_row
                .set_subtitle(Some(path.to_str().unwrap()));
        }

        if let Some(url) = this.backend.get_server_url() {
            this.url_row.set_subtitle(Some(&url));
        }

        if let Some(data) = this.backend.get_login_data() {
            this.login_row.set_subtitle(Some(&data.username));
        }

        this
    }

    /// Show the preferences dialog.
    pub fn show(&self) {
        self.window.show();
    }
}
