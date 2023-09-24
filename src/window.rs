use crate::{
    home_page::MusicusHomePage, playlist_page::MusicusPlaylistPage,
    welcome_page::MusicusWelcomePage,
};

use adw::subclass::prelude::*;
use gtk::{gio, glib, prelude::*};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/de/johrpan/musicus/window.ui")]
    pub struct MusicusWindow {
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub navigation_view: TemplateChild<adw::NavigationView>,
        #[template_child]
        pub player_bar_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub playlist_button: TemplateChild<gtk::ToggleButton>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MusicusWindow {
        const NAME: &'static str = "MusicusWindow";
        type Type = super::MusicusWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            MusicusHomePage::static_type();
            MusicusPlaylistPage::static_type();
            MusicusWelcomePage::static_type();
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for MusicusWindow {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().load_window_state();
        }
    }

    impl WidgetImpl for MusicusWindow {}

    impl WindowImpl for MusicusWindow {
        fn close_request(&self) -> glib::signal::Inhibit {
            if let Err(err) = self.obj().save_window_state() {
                log::warn!("Failed to save window state: {err}");
            }

            glib::signal::Inhibit(false)
        }
    }

    impl ApplicationWindowImpl for MusicusWindow {}
    impl AdwApplicationWindowImpl for MusicusWindow {}
}

glib::wrapper! {
    pub struct MusicusWindow(ObjectSubclass<imp::MusicusWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionGroup, gio::ActionMap;
}

#[gtk::template_callbacks]
impl MusicusWindow {
    pub fn new<P: glib::IsA<gtk::Application>>(application: &P) -> Self {
        glib::Object::builder()
            .property("application", application)
            .build()
    }

    pub fn load_window_state(&self) {
        let settings = gio::Settings::new("de.johrpan.musicus");
        self.set_default_size(settings.int("window-width"), settings.int("window-height"));
        self.set_property("maximized", settings.boolean("is-maximized"));
    }

    pub fn save_window_state(&self) -> Result<(), glib::BoolError> {
        let settings = gio::Settings::new("de.johrpan.musicus");

        let size = self.default_size();
        settings.set_int("window-width", size.0)?;
        settings.set_int("window-height", size.1)?;
        settings.set_boolean("is-maximized", self.is_maximized())?;

        Ok(())
    }

    #[template_callback]
    fn set_library_folder(&self, folder: &gio::File) {
        let path = folder.path();
        log::info!("{path:?}");
        self.imp().navigation_view.replace_with_tags(&["home"]);
    }

    #[template_callback]
    fn show_playlist(&self, button: &gtk::ToggleButton) {
        self.imp()
            .stack
            .set_visible_child_name(if button.is_active() {
                "playlist"
            } else {
                "navigation"
            });
    }

    #[template_callback]
    fn hide_playlist(&self, _: &MusicusPlaylistPage) {
        self.imp().playlist_button.set_active(false);
    }
}
