use crate::backend::*;
use crate::dialogs::*;
use crate::import::SourceSelector;
use crate::screens::*;
use crate::widgets::*;
use futures::prelude::*;
use gettextrs::gettext;
use gio::prelude::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::{action, get_widget};
use std::rc::Rc;

pub struct Window {
    backend: Rc<Backend>,
    window: libhandy::ApplicationWindow,
    stack: gtk::Stack,
    leaflet: libhandy::Leaflet,
    sidebar_box: gtk::Box,
    poe_list: Rc<PoeList>,
    navigator: Rc<Navigator>,
    player_bar: PlayerBar,
    player_screen: Rc<PlayerScreen>,
}

impl Window {
    pub fn new(app: &gtk::Application) -> Rc<Self> {
        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/window.ui");

        get_widget!(builder, libhandy::ApplicationWindow, window);
        get_widget!(builder, gtk::Stack, stack);
        get_widget!(builder, gtk::Button, select_music_library_path_button);
        get_widget!(builder, gtk::Box, content_box);
        get_widget!(builder, libhandy::Leaflet, leaflet);
        get_widget!(builder, gtk::Button, add_button);
        get_widget!(builder, gtk::Box, sidebar_box);
        get_widget!(builder, gtk::Box, empty_screen);

        let backend = Rc::new(Backend::new());

        let player_screen = PlayerScreen::new();
        stack.add_named(&player_screen.widget, Some("player_screen"));

        let poe_list = PoeList::new(backend.clone());
        let navigator = Navigator::new(&window, &empty_screen);
        navigator.set_back_cb(clone!(@strong leaflet, @strong sidebar_box => move || {
            leaflet.set_visible_child(&sidebar_box);
        }));

        let player_bar = PlayerBar::new();
        content_box.append(&player_bar.widget);

        let result = Rc::new(Self {
            backend,
            window,
            stack,
            leaflet,
            sidebar_box,
            poe_list,
            navigator,
            player_bar,
            player_screen,
        });

        result.window.set_application(Some(app));

        select_music_library_path_button.connect_clicked(clone!(@strong result => move |_| {
            let dialog = gtk::FileChooserNative::new(
                Some(&gettext("Select music library folder")),
                Some(&result.window),
                gtk::FileChooserAction::SelectFolder,
                None,
                None);

            dialog.connect_response(clone!(@strong result => move |dialog, response| {
                if response == gtk::ResponseType::Accept {
                    if let Some(file) = dialog.get_file() {
                        if let Some(path) = file.get_path() {
                            let context = glib::MainContext::default();
                            let backend = result.backend.clone();
                            context.spawn_local(async move {
                                backend.set_music_library_path(path).await.unwrap();
                            });
                        }
                    }
                }
            }));

            dialog.show();
        }));

        add_button.connect_clicked(clone!(@strong result => move |_| {
            // let editor = TracksEditor::new(result.backend.clone(), None, Vec::new());

            // editor.set_callback(clone!(@strong result => move || {
            //     result.reload();
            // }));

            // let window = NavigatorWindow::new(editor);
            // window.show();

            let dialog = SourceSelector::new(result.backend.clone());
            let window = NavigatorWindow::new(dialog);
            window.show();
        }));

        result
            .player_bar
            .set_playlist_cb(clone!(@strong result => move || {
                result.stack.set_visible_child_name("player_screen");
            }));

        result
            .player_screen
            .set_back_cb(clone!(@strong result => move || {
                result.stack.set_visible_child_name("content");
            }));

        // action!(
        //     result.window,
        //     "import-disc",
        //     clone!(@strong result => move |_, _| {
        //         let dialog = ImportDiscDialog::new(result.backend.clone());
        //         let window = NavigatorWindow::new(dialog);
        //         window.show();
        //     })
        // );

        action!(
            result.window,
            "preferences",
            clone!(@strong result => move |_, _| {
                Preferences::new(result.backend.clone(), &result.window).show();
            })
        );

        action!(
            result.window,
            "about",
            clone!(@strong result => move |_, _| {
                show_about_dialog(&result.window);
            })
        );

        let context = glib::MainContext::default();
        let clone = result.clone();
        context.spawn_local(async move {
            let mut state_stream = clone.backend.state_stream.borrow_mut();
            while let Some(state) = state_stream.next().await {
                match state {
                    BackendState::NoMusicLibrary => {
                        clone.stack.set_visible_child_name("empty");
                    }
                    BackendState::Loading => {
                        clone.stack.set_visible_child_name("loading");
                    }
                    BackendState::Ready => {
                        clone.stack.set_visible_child_name("content");
                        clone.poe_list.clone().reload();
                        clone.navigator.reset();

                        let player = clone.backend.get_player().unwrap();
                        clone.player_bar.set_player(Some(player.clone()));
                        clone.player_screen.clone().set_player(Some(player));
                    }
                }
            }
        });

        let clone = result.clone();
        context.spawn_local(async move {
            // This is not done in the async block below, because backend state changes may happen
            // while this method is running.
            clone.backend.clone().init().await.unwrap();
        });

        result.leaflet.append(&result.navigator.widget);

        result
            .poe_list
            .set_selected_cb(clone!(@strong result => move |poe| {
                result.leaflet.set_visible_child(&result.navigator.widget);
                match poe {
                    PersonOrEnsemble::Person(person) => {
                        result.navigator.clone().replace(PersonScreen::new(result.backend.clone(), person.clone()));
                    }
                    PersonOrEnsemble::Ensemble(ensemble) => {
                        result.navigator.clone().replace(EnsembleScreen::new(result.backend.clone(), ensemble.clone()));
                    }
                }
            }));

        result
            .sidebar_box
            .append(&result.poe_list.widget);

        result
    }

    pub fn present(&self) {
        self.window.present();
    }

    fn reload(&self) {
        self.poe_list.clone().reload();
        self.navigator.reset();
        self.leaflet.set_visible_child(&self.sidebar_box);
    }
}
