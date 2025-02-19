use adw::{prelude::*, subclass::prelude::*};
use gettextrs::gettext;
use gtk::{gio, glib};

use crate::{config, MusicusWindow};

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct MusicusApplication {}

    #[glib::object_subclass]
    impl ObjectSubclass for MusicusApplication {
        const NAME: &'static str = "MusicusApplication";
        type Type = super::MusicusApplication;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for MusicusApplication {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_gactions();
            obj.set_accels_for_action("app.quit", &["<primary>q"]);
        }
    }

    impl ApplicationImpl for MusicusApplication {
        fn activate(&self) {
            let application = self.obj();

            let window = if let Some(window) = application.active_window() {
                window
            } else {
                let window = MusicusWindow::new(&*application);
                window.upcast()
            };

            window.present();
        }
    }

    impl GtkApplicationImpl for MusicusApplication {}
    impl AdwApplicationImpl for MusicusApplication {}
}

glib::wrapper! {
    pub struct MusicusApplication(ObjectSubclass<imp::MusicusApplication>)
        @extends gio::Application, gtk::Application, adw::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl MusicusApplication {
    pub fn new() -> Self {
        glib::Object::builder()
            .property("application-id", config::APP_ID)
            .property("resource-base-path", config::PATH_ID)
            .build()
    }

    fn setup_gactions(&self) {
        let quit_action = gio::ActionEntry::builder("quit")
            .activate(move |app: &Self, _, _| app.quit())
            .build();

        let about_action = gio::ActionEntry::builder("about")
            .activate(move |app: &Self, _, _| app.show_about())
            .build();

        self.add_action_entries([quit_action, about_action]);
    }

    fn show_about(&self) {
        let window = self.active_window().unwrap();
        let about = adw::AboutDialog::builder()
            .application_name(gettext(config::NAME))
            .application_icon(config::APP_ID)
            .developer_name("Elias Projahn")
            .version(config::VERSION)
            .website("https://github.com/johrpan/musicus")
            .developers(vec!["Elias Projahn <elias@johrpan.de>"])
            .copyright("© 2025 Elias Projahn")
            .license_type(gtk::License::Gpl30)
            .build();

        about.present(Some(&window));
    }
}
