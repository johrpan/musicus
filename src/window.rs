use crate::{
    home_page::MusicusHomePage, library::MusicusLibrary, player::MusicusPlayer,
    playlist_page::MusicusPlaylistPage, welcome_page::MusicusWelcomePage,
};
use adw::subclass::prelude::*;
use gtk::{gio, glib, glib::clone, prelude::*};
use std::cell::OnceCell;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/window.blp")]
    pub struct MusicusWindow {
        pub player: MusicusPlayer,
        pub playlist_page: OnceCell<MusicusPlaylistPage>,

        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub navigation_view: TemplateChild<adw::NavigationView>,
        #[template_child]
        pub player_bar_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub play_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub playlist_button: TemplateChild<gtk::ToggleButton>,
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
            self.player
                .bind_property("active", &self.player_bar_revealer.get(), "reveal-child")
                .sync_create()
                .build();

            let play_button = self.play_button.get();

            self.player
                .connect_playing_notify(clone!(@weak play_button => move |player| {
                    play_button.set_icon_name(if player.playing() {
                        "media-playback-pause-symbolic"
                    } else {
                        "media-playback-start-symbolic"
                    });
                }));

            self.play_button
                .connect_clicked(clone!(@weak self.player as player => move |_| {
                    if player.playing() {
                        player.pause();
                    } else {
                        player.play();
                    }
                }));

            let playlist_page = MusicusPlaylistPage::new(&self.player);
            let playlist_button = self.playlist_button.get();
            playlist_page.connect_close(move |_| {
                playlist_button.set_active(false);
            });

            self.stack.add_named(&playlist_page, Some("playlist"));
            self.playlist_page.set(playlist_page).unwrap();
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

    #[template_callback]
    fn show_playlist(&self, button: &gtk::ToggleButton) {
        let imp = self.imp();

        if button.is_active() {
            imp.playlist_page.get().unwrap().scroll_to_current();
            imp.stack.set_visible_child_name("playlist");
        } else {
            imp.stack.set_visible_child_name("navigation");
        };
    }
}
