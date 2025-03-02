use std::{cell::RefCell, path::Path};

use adw::subclass::prelude::*;
use gtk::{gio, glib, glib::clone, prelude::*};

use crate::{
    config,
    editor::tracks::TracksEditor,
    library::{Library, LibraryQuery},
    library_manager::LibraryManager,
    player::Player,
    player_bar::PlayerBar,
    playlist_page::PlaylistPage,
    search_page::SearchPage,
    welcome_page::WelcomePage,
};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/window.blp")]
    pub struct Window {
        pub library: RefCell<Option<Library>>,
        pub player: Player,

        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub navigation_view: TemplateChild<adw::NavigationView>,
        #[template_child]
        pub player_bar_revealer: TemplateChild<gtk::Revealer>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Window {
        const NAME: &'static str = "MusicusWindow";
        type Type = super::Window;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            WelcomePage::static_type();
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Window {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().load_window_state();

            if config::PROFILE == "development" {
                self.obj().add_css_class("devel");
            }

            let obj = self.obj().to_owned();
            let import_action = gio::ActionEntry::builder("import")
                .activate(move |_, _, _| {
                    if let Some(library) = &*obj.imp().library.borrow() {
                        let editor = TracksEditor::new(&obj.imp().navigation_view, library, None);
                        obj.imp().navigation_view.push(&editor);
                    }
                })
                .build();

            let obj = self.obj().to_owned();
            let library_action = gio::ActionEntry::builder("library")
                .activate(move |_, _, _| {
                    if let Some(library) = &*obj.imp().library.borrow() {
                        let library_manager =
                            LibraryManager::new(&obj.imp().navigation_view, library);
                        obj.imp().navigation_view.push(&library_manager);
                    }
                })
                .build();

            self.obj()
                .add_action_entries([import_action, library_action]);

            let player_bar = PlayerBar::new(&self.player);
            self.player_bar_revealer.set_child(Some(&player_bar));

            let playlist_page = PlaylistPage::new(&self.player);
            self.stack.add_named(&playlist_page, Some("playlist"));

            let stack = self.stack.get();
            playlist_page.connect_close(clone!(
                #[weak]
                player_bar,
                #[weak]
                stack,
                move |_| {
                    stack.set_visible_child_name("navigation");
                    player_bar.playlist_hidden();
                }
            ));

            player_bar.connect_show_playlist(clone!(
                #[weak]
                playlist_page,
                #[weak]
                stack,
                move |_, show| {
                    if show {
                        playlist_page.scroll_to_current();
                        stack.set_visible_child_name("playlist");
                    } else {
                        stack.set_visible_child_name("navigation");
                    };
                }
            ));

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

    impl WidgetImpl for Window {}

    impl WindowImpl for Window {
        fn close_request(&self) -> glib::signal::Propagation {
            if let Err(err) = self.obj().save_window_state() {
                log::warn!("Failed to save window state: {err}");
            }

            glib::signal::Propagation::Proceed
        }
    }

    impl ApplicationWindowImpl for Window {}
    impl AdwApplicationWindowImpl for Window {}
}

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionGroup, gio::ActionMap;
}

#[gtk::template_callbacks]
impl Window {
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
    pub fn set_library_folder(&self, folder: &gio::File) {
        let path = folder.path().unwrap();

        let settings = gio::Settings::new(config::APP_ID);
        settings
            .set_string("library-path", path.to_str().unwrap())
            .unwrap();

        self.load_library(path);
    }

    fn load_library(&self, path: impl AsRef<Path>) {
        let library = Library::new(path);
        self.imp().player.set_library(&library);

        let navigation = self.imp().navigation_view.get();
        navigation.replace(&[SearchPage::new(
            &navigation,
            &library,
            &self.imp().player,
            LibraryQuery::default(),
        )
        .into()]);

        self.imp().library.replace(Some(library));
    }
}
