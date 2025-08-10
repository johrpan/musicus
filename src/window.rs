use std::{
    cell::{Cell, RefCell},
    path::Path,
};

use adw::{prelude::*, subclass::prelude::*};
use anyhow::{anyhow, Result};
use gettextrs::gettext;
use gtk::{gio, glib, glib::clone};

use crate::{
    album_page::AlbumPage,
    config,
    editor::{album::AlbumEditor, tracks::TracksEditor},
    empty_page::EmptyPage,
    library::{Library, LibraryQuery},
    library_manager::LibraryManager,
    player::Player,
    player_bar::PlayerBar,
    playlist_page::PlaylistPage,
    preferences_dialog::PreferencesDialog,
    process::Process,
    process_manager::ProcessManager,
    search_page::SearchPage,
    util,
    welcome_page::WelcomePage,
};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/window.blp")]
    pub struct Window {
        pub library: RefCell<Option<Library>>,
        pub player: Player,
        pub process_manager: ProcessManager,
        pub inhibitor_cookie: Cell<Option<u32>>,

        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,
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
                        let editor = TracksEditor::new(
                            &obj.imp().toast_overlay,
                            &obj.imp().navigation_view,
                            library,
                            None,
                        );
                        obj.imp().navigation_view.push(&editor);
                    }
                })
                .build();

            let obj = self.obj().to_owned();
            let create_album_action = gio::ActionEntry::builder("create-album")
                .activate(move |_, _, _| {
                    if let Some(library) = &*obj.imp().library.borrow() {
                        let editor = AlbumEditor::new(&obj.imp().navigation_view, library, None);
                        obj.imp().navigation_view.push(&editor);
                    }
                })
                .build();

            let obj = self.obj().to_owned();
            let library_action = gio::ActionEntry::builder("library")
                .activate(move |_, _, _| {
                    if let Some(library) = &*obj.imp().library.borrow() {
                        let library_manager = LibraryManager::new(
                            &obj.imp().navigation_view,
                            library,
                            &obj.imp().process_manager,
                        );
                        obj.imp().navigation_view.push(&library_manager);
                    }
                })
                .build();

            let obj = self.obj().to_owned();
            let preferences_action = gio::ActionEntry::builder("preferences")
                .activate(move |_, _, _| {
                    PreferencesDialog::show(&obj);
                })
                .build();

            self.obj().add_action_entries([
                import_action,
                create_album_action,
                library_action,
                preferences_action,
            ]);

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

            let obj = self.obj().to_owned();
            self.player.connect_playing_notify(move |player| {
                if let Some(app) = obj.application() {
                    if let Some(cookie) = obj.imp().inhibitor_cookie.take() {
                        app.uninhibit(cookie);
                    };

                    if player.playing() {
                        let cookie = app.inhibit(
                            Some(&obj),
                            gtk::ApplicationInhibitFlags::SUSPEND,
                            Some(&gettext("Currently playing music")),
                        );

                        obj.imp().inhibitor_cookie.set(Some(cookie));
                    }
                }
            });

            let settings = gio::Settings::new(config::APP_ID);
            let library_path = settings.string("library-path").to_string();
            if !library_path.is_empty() {
                if let Err(err) = self.obj().load_library(&library_path) {
                    util::error_toast("Failed to open music library", err, &self.toast_overlay);
                }
            }
        }
    }

    impl WidgetImpl for Window {}

    impl WindowImpl for Window {
        fn close_request(&self) -> glib::signal::Propagation {
            if self.process_manager.any_ongoing() {
                let dialog = adw::AlertDialog::builder()
                    .heading(&gettext("Close window?"))
                    .body(&gettext(
                        "There are ongoing processes that will be canceled.",
                    ))
                    .build();

                dialog.add_responses(&[
                    ("cancel", &gettext("Keep open")),
                    ("close", &gettext("Close window")),
                ]);

                dialog.set_response_appearance("close", adw::ResponseAppearance::Destructive);
                dialog.set_close_response("cancel");
                dialog.set_default_response(Some("cancel"));

                let obj = self.obj().to_owned();
                glib::spawn_future_local(async move {
                    if dialog.choose_future(&obj).await == "close" {
                        obj.destroy();
                    }
                });

                glib::signal::Propagation::Stop
            } else {
                if let Err(err) = self.obj().save_window_state() {
                    log::warn!("Failed to save window state: {err:?}");
                }

                glib::signal::Propagation::Proceed
            }
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

        match self.load_library(&path) {
            Ok(_) => {
                if let Err(err) = self.save_library_path(path) {
                    util::error_toast(
                        "Failed to save library folder",
                        err,
                        &self.imp().toast_overlay,
                    );
                }
            }
            Err(err) => {
                util::error_toast(
                    "Failed to open music library",
                    err,
                    &self.imp().toast_overlay,
                );
            }
        }
    }

    fn load_library(&self, path: impl AsRef<Path>) -> Result<()> {
        let library = Library::new(path)?;

        library.connect_changed(clone!(
            #[weak(rename_to = obj)]
            self,
            move |_| obj.reset_view()
        ));

        self.imp().player.set_library(&library);

        let is_empty = library.is_empty()?;

        let settings = gio::Settings::new(config::APP_ID);
        if settings.boolean("enable-automatic-metadata-updates") {
            let url = if settings.boolean("use-custom-metadata-url") {
                settings.string("custom-metadata-url").to_string()
            } else {
                config::METADATA_URL.to_string()
            };

            match library.import_metadata_from_url(&url) {
                Ok(receiver) => {
                    let process = Process::new(&gettext("Updating metadata"), receiver);
                    self.imp().process_manager.add_process(&process);
                }
                Err(err) => log::error!("Failed to update metadata: {err:?}"),
            }
        }

        self.imp().library.replace(Some(library));

        if is_empty {
            let navigation = self.imp().navigation_view.get();
            let empty_page = EmptyPage::new(
                self.imp().library.borrow().as_ref().unwrap(),
                &self.imp().process_manager,
            );

            empty_page.connect_ready(clone!(
                #[weak(rename_to = obj)]
                self,
                move |_| {
                    obj.initial_view();
                }
            ));

            navigation.replace(&[empty_page.into()]);
        } else {
            self.initial_view();
        }

        Ok(())
    }

    fn save_library_path(&self, path: impl AsRef<Path>) -> Result<()> {
        let settings = gio::Settings::new(config::APP_ID);
        settings.set_string(
            "library-path",
            path.as_ref()
                .to_str()
                .ok_or_else(|| anyhow!("Failed to convert path to string"))?,
        )?;

        Ok(())
    }

    fn initial_view(&self) {
        let navigation = self.imp().navigation_view.get();

        navigation.replace(&[SearchPage::new(
            &self.imp().toast_overlay,
            &navigation,
            self.imp().library.borrow().as_ref().unwrap(),
            &self.imp().player,
            LibraryQuery::default(),
        )
        .into()]);
    }

    fn reset_view(&self) {
        let navigation = self.imp().navigation_view.get();

        // Get all pages that are not instances of SearchPage or AlbumPage.
        let mut navigation_stack = navigation
            .navigation_stack()
            .iter::<adw::NavigationPage>()
            .filter_map(|page| page.ok())
            .filter(|page| !page.is::<SearchPage>() && !page.is::<AlbumPage>())
            .collect::<Vec<adw::NavigationPage>>();

        navigation_stack.insert(
            0,
            SearchPage::new(
                &self.imp().toast_overlay,
                &navigation,
                self.imp().library.borrow().as_ref().unwrap(),
                &self.imp().player,
                LibraryQuery::default(),
            )
            .into(),
        );

        // Readd all pages except for instances of SearchPage and add a new SearchPage as the root.
        navigation.replace(&navigation_stack);
    }
}
