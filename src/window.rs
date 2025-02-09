use std::path::Path;

use adw::subclass::prelude::*;
use gtk::{gio, glib, glib::clone, prelude::*};

use crate::{
    config, home_page::MusicusHomePage, library::MusicusLibrary, library_manager::LibraryManager,
    player::MusicusPlayer, player_bar::PlayerBar, playlist_page::MusicusPlaylistPage,
    welcome_page::MusicusWelcomePage,
};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/window.blp")]
    pub struct MusicusWindow {
        pub player: MusicusPlayer,

        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub navigation_view: TemplateChild<adw::NavigationView>,
        #[template_child]
        pub player_bar_revealer: TemplateChild<gtk::Revealer>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MusicusWindow {
        const NAME: &'static str = "MusicusWindow";
        type Type = super::MusicusWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
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

            if config::PROFILE == "development" {
                self.obj().add_css_class("devel");
            }

            let navigation_view = self.navigation_view.get().to_owned();
            let library_action = gio::ActionEntry::builder("library")
                .activate(move |_: &super::MusicusWindow, _, _| {
                    navigation_view.push_by_tag("library")
                })
                .build();

            self.obj().add_action_entries([library_action]);

            let player_bar = PlayerBar::new(&self.player);
            self.player_bar_revealer.set_child(Some(&player_bar));

            let playlist_page = MusicusPlaylistPage::new(&self.player);
            self.stack.add_named(&playlist_page, Some("playlist"));

            let stack = self.stack.get();
            playlist_page.connect_close(clone!(@weak player_bar, @weak stack => move |_| {
                stack.set_visible_child_name("navigation");
                player_bar.playlist_hidden();
            }));

            player_bar.connect_show_playlist(
                clone!(@weak playlist_page, @weak stack => move |_, show| {
                    if show {
                        playlist_page.scroll_to_current();
                        stack.set_visible_child_name("playlist");
                    } else {
                        stack.set_visible_child_name("navigation");
                    };
                }),
            );

            self.player
                .bind_property("active", &self.player_bar_revealer.get(), "reveal-child")
                .sync_create()
                .build();

            let obj = self.obj().to_owned();
            self.player.connect_raise(move |_| obj.present());

            let settings = gio::Settings::new(config::APP_ID);
            let library_path = settings.string("library-path").to_string();
            if !library_path.is_empty() {
                self.obj().load_library(&library_path);
            }
        }
    }

    impl WidgetImpl for MusicusWindow {}

    impl WindowImpl for MusicusWindow {
        fn close_request(&self) -> glib::signal::Propagation {
            if let Err(err) = self.obj().save_window_state() {
                log::warn!("Failed to save window state: {err}");
            }

            glib::signal::Propagation::Proceed
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
    pub fn new<P: IsA<gtk::Application>>(application: &P) -> Self {
        glib::Object::builder()
            .property("application", application)
            .build()
    }

    pub fn load_window_state(&self) {
        let settings = gio::Settings::new(config::APP_ID);
        self.set_default_size(settings.int("window-width"), settings.int("window-height"));
        self.set_property("maximized", settings.boolean("is-maximized"));
    }

    pub fn save_window_state(&self) -> Result<(), glib::BoolError> {
        let settings = gio::Settings::new(config::APP_ID);

        let size = self.default_size();
        settings.set_int("window-width", size.0)?;
        settings.set_int("window-height", size.1)?;
        settings.set_boolean("is-maximized", self.is_maximized())?;

        Ok(())
    }

    #[template_callback]
    fn set_library_folder(&self, folder: &gio::File) {
        let path = folder.path().unwrap();

        let settings = gio::Settings::new(config::APP_ID);
        settings
            .set_string("library-path", path.to_str().unwrap())
            .unwrap();

        self.load_library(path);
    }

    fn load_library(&self, path: impl AsRef<Path>) {
        let library = MusicusLibrary::new(path);
        self.imp().player.set_library(&library);

        let navigation = self.imp().navigation_view.get();
        navigation
            .replace(&[MusicusHomePage::new(&navigation, &library, &self.imp().player).into()]);
        navigation.add(&LibraryManager::new(&library));
    }
}
