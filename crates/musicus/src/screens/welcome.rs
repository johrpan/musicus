use crate::navigator::{NavigationHandle, Screen};
use crate::widgets::Widget;
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use std::rc::Rc;

/// A screen displaying a welcome message and the necessary means to set up the application. This
/// screen doesn't access the backend except for setting the initial values and is safe to be used
/// while the backend is loading.
pub struct WelcomeScreen {
    handle: NavigationHandle<()>,
    widget: gtk::Box,
}

impl Screen<(), ()> for WelcomeScreen {
    fn new(_: (), handle: NavigationHandle<()>) -> Rc<Self> {
        let widget = gtk::BoxBuilder::new()
            .orientation(gtk::Orientation::Vertical)
            .build();

        let header = libadwaita::HeaderBarBuilder::new()
            .title_widget(&libadwaita::WindowTitle::new(Some("Musicus"), None))
            .build();

        let button = gtk::ButtonBuilder::new()
            .halign(gtk::Align::Center)
            .label(&gettext("Select folder"))
            .build();

        let welcome = libadwaita::StatusPageBuilder::new()
            .icon_name("folder-music-symbolic")
            .title(&gettext("Welcome to Musicus!"))
            .description(&gettext("Get startet by selecting the folder containing your music \
                files! Musicus will create a new database there or open one that already exists."))
            .child(&button)
            .build();

        button.add_css_class("suggested-action");

        widget.append(&header);
        widget.append(&welcome);

        let this = Rc::new(Self {
            handle,
            widget,
        });

        button.connect_clicked(clone!(@weak this => move |_| {
            let dialog = gtk::FileChooserDialog::new(
                Some(&gettext("Select music library folder")),
                Some(&this.handle.window),
                gtk::FileChooserAction::SelectFolder,
                &[
                    (&gettext("Cancel"), gtk::ResponseType::Cancel),
                    (&gettext("Select"), gtk::ResponseType::Accept),
                ]);

            dialog.set_modal(true);

            dialog.connect_response(clone!(@weak this => move |dialog, response| {
                if let gtk::ResponseType::Accept = response {
                    if let Some(file) = dialog.get_file() {
                        if let Some(path) = file.get_path() {
                            spawn!(@clone this, async move {
                                this.handle.backend.set_music_library_path(path).await.unwrap();
                            });
                        }
                    }
                }

                dialog.hide();
            }));

            dialog.show();
        }));

        this
    }
}

impl Widget for WelcomeScreen {
    fn get_widget(&self) -> gtk::Widget {
        self.widget.clone().upcast()
    }
}
