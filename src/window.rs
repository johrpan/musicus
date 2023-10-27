use crate::{
    home_page::MusicusHomePage, library::MusicusLibrary, player::MusicusPlayer,
    player_bar::PlayerBar, playlist_page::MusicusPlaylistPage, welcome_page::MusicusWelcomePage,
};
use adw::subclass::prelude::*;
use gtk::{gio, glib, glib::clone, prelude::*};

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

            let player_bar = PlayerBar::new(&self.player);
            self.player_bar_revealer.set_child(Some(&player_bar));

            let playlist_page = MusicusPlaylistPage::new(&self.player);
            self.stack.add_named(&playlist_page, Some("playlist"));

            let stack = self.stack.get();
            playlist_page.connect_close(clone!(@weak player_bar, @weak stack => move |_| {
                stack.set_visible_child_name("navigation");
                player_bar.playlist_hidden();
            }));

            player_bar.connect_show_playlist(clone!(@weak playlist_page, @weak stack => move |_, show| {
                if show {
                    playlist_page.scroll_to_current();
                    stack.set_visible_child_name("playlist");
                } else {
                    stack.set_visible_child_name("navigation");
                };
            }));

            self.player
                .bind_property("active", &self.player_bar_revealer.get(), "reveal-child")
                .sync_create()
                .build();
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
        let path = folder.path().unwrap();
        let library = MusicusLibrary::new(path);
        self.imp()
            .navigation_view
            .replace(&[MusicusHomePage::new(&library, &self.imp().player).into()]);
    }
}
