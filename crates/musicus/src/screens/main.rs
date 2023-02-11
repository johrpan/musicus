use super::{EnsembleScreen, PersonScreen, PlayerScreen};
use crate::config;
use crate::import::SourceSelector;
use crate::navigator::{NavigationHandle, Navigator, NavigatorWindow, Screen};
use crate::preferences::Preferences;
use crate::widgets::{List, PlayerBar, Widget};
use adw::builders::ActionRowBuilder;
use adw::prelude::*;
use gettextrs::gettext;
use glib::clone;
use gtk_macros::get_widget;
use musicus_backend::db::{self, PersonOrEnsemble};
use std::cell::RefCell;
use std::rc::Rc;

/// The main screen of the app, once it's set up and finished loading. The screen assumes that the
/// music library and the player are available and initialized.
pub struct MainScreen {
    handle: NavigationHandle<()>,
    widget: gtk::Box,
    leaflet: adw::Leaflet,
    search_entry: gtk::SearchEntry,
    stack: gtk::Stack,
    poe_list: Rc<List>,
    navigator: Rc<Navigator>,
    poes: RefCell<Vec<PersonOrEnsemble>>,
}

impl Screen<(), ()> for MainScreen {
    /// Create a new main screen.
    fn new(_: (), handle: NavigationHandle<()>) -> Rc<Self> {
        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/main_screen.ui");

        get_widget!(builder, gtk::Box, widget);
        get_widget!(builder, adw::Leaflet, leaflet);
        get_widget!(builder, gtk::Revealer, play_button_revealer);
        get_widget!(builder, gtk::Button, play_button);
        get_widget!(builder, gtk::Button, add_button);
        get_widget!(builder, gtk::SearchEntry, search_entry);
        get_widget!(builder, gtk::Stack, stack);
        get_widget!(builder, gtk::ScrolledWindow, scroll);
        get_widget!(builder, gtk::Box, empty_screen);

        let actions = gio::SimpleActionGroup::new();
        let preferences_action = gio::SimpleAction::new("preferences", None);
        let log_action = gio::SimpleAction::new("log", None);
        let about_action = gio::SimpleAction::new("about", None);
        actions.add_action(&preferences_action);
        actions.add_action(&log_action);
        actions.add_action(&about_action);
        widget.insert_action_group("widget", Some(&actions));

        let poe_list = List::new();
        poe_list.widget.set_css_classes(&["navigation-sidebar"]);
        poe_list.enable_selection();

        let navigator = Navigator::new(Rc::clone(&handle.backend), &handle.window, &empty_screen);

        scroll.set_child(Some(&poe_list.widget));
        leaflet.append(&navigator.widget);

        let player_bar = PlayerBar::new();
        widget.append(&player_bar.widget);
        player_bar.set_player(Some(Rc::clone(&handle.backend.pl())));

        let this = Rc::new(Self {
            handle,
            widget,
            leaflet,
            search_entry,
            stack,
            poe_list,
            navigator,
            poes: RefCell::new(Vec::new()),
        });

        preferences_action.connect_activate(clone!(@weak this =>  move |_, _| {
            Preferences::new(Rc::clone(&this.handle.backend), &this.handle.window).show();
        }));

        log_action.connect_activate(clone!(@weak this => move |_, _| {
            this.show_log_window();
        }));

        about_action.connect_activate(clone!(@weak this =>  move |_, _| {
            this.show_about_dialog();
        }));

        add_button.connect_clicked(clone!(@weak this =>  move |_| {
            spawn!(@clone this, async move {
                let window = NavigatorWindow::new(Rc::clone(&this.handle.backend));
                replace!(window.navigator, SourceSelector).await;
            });
        }));

        this.search_entry
            .connect_search_changed(clone!(@weak this =>  move |_| {
                this.poe_list.invalidate_filter();
            }));

        this.poe_list
            .set_make_widget_cb(clone!(@weak this =>  @default-panic, move |index| {
                let poe = &this.poes.borrow()[index];

                let row = ActionRowBuilder::new()
                    .activatable(true)
                    .title(&poe.get_title())
                    .build();

                let poe = poe.to_owned();
                row.connect_activated(clone!(@weak this =>  move |_| {
                    let poe = poe.clone();
                    spawn!(@clone this, async move {
                        this.leaflet.set_visible_child(&this.navigator.widget);

                        match poe {
                            PersonOrEnsemble::Person(person) => {
                                replace!(this.navigator, PersonScreen, person).await;
                            }
                            PersonOrEnsemble::Ensemble(ensemble) => {
                                replace!(this.navigator, EnsembleScreen, ensemble).await;
                            }
                        }
                    });
                }));

                row.upcast()
            }));

        this.poe_list
            .set_filter_cb(clone!(@weak this =>  @default-panic, move |index| {
                let poe = &this.poes.borrow()[index];
                let search = this.search_entry.text().to_string().to_lowercase();
                let title = poe.get_title().to_lowercase();
                search.is_empty() || title.contains(&search)
            }));

        this.handle.backend.pl().add_playlist_cb(
            clone!(@weak play_button_revealer => move |new_playlist| {
                    play_button_revealer.set_reveal_child(new_playlist.is_empty());
            }),
        );

        play_button.connect_clicked(clone!(@weak this => move |_| {
            let recording = db::random_recording(&mut this.handle.backend.db().lock().unwrap());
            if let Ok(recording) = recording {
                this.handle.backend.pl().add_items(db::get_tracks(&mut this.handle.backend.db().lock().unwrap(), &recording.id).unwrap()).unwrap();
            }
        }));

        this.navigator.set_back_cb(clone!(@weak this =>  move || {
            this.leaflet.set_visible_child_name("sidebar");
        }));

        player_bar.set_playlist_cb(clone!(@weak this =>  move || {
            spawn!(@clone this, async move {
                push!(this.handle, PlayerScreen).await;
            });
        }));

        // Load the content whenever there is a new library update.
        spawn!(@clone this, async move {
            loop {
                this.navigator.reset();

                let mut poes = Vec::new();

                let persons = db::get_persons(&mut this.handle.backend.db().lock().unwrap(), ).unwrap();
                let ensembles = db::get_ensembles(&mut this.handle.backend.db().lock().unwrap(), ).unwrap();

                for person in persons {
                    poes.push(PersonOrEnsemble::Person(person));
                }

                for ensemble in ensembles {
                    poes.push(PersonOrEnsemble::Ensemble(ensemble));
                }

                let length = poes.len();
                this.poes.replace(poes);
                this.poe_list.update(length);

                this.stack.set_visible_child_name("content");

                if this.handle.backend.library_update().await.is_err() {
                    break;
                }
            }
        });

        this
    }
}

impl Widget for MainScreen {
    fn get_widget(&self) -> gtk::Widget {
        self.widget.clone().upcast()
    }
}

impl MainScreen {
    /// Show a window displaying all currently cached log messages.
    fn show_log_window(&self) {
        let copy_button = gtk::Button::builder().icon_name("copy-symbolic").build();
        let logger = self.handle.backend.logger();
        let toast_overlay = adw::ToastOverlay::new();

        copy_button.connect_clicked(clone!(@weak logger, @weak toast_overlay => move |widget| {
            widget.clipboard().set_text(&logger.messages().into_iter().map(|m| m.to_string()).collect::<Vec<String>>().join("\n"));
            toast_overlay.add_toast(&adw::Toast::builder().title(&gettext("Copied to clipboard")).build());
        }));

        let header = adw::HeaderBar::builder()
            .title_widget(
                &adw::WindowTitle::builder()
                    .title(&gettext("Debug log"))
                    .build(),
            )
            .build();

        header.pack_end(&copy_button);

        let log_list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .build();

        for message in logger.messages() {
            log_list.append(
                &adw::ActionRow::builder()
                    .title(&format!(
                        "<b>{}</b> {} <i>{}</i>",
                        message.level,
                        message.time.format("%Y-%m-%d %H:%M:%S"),
                        message.module
                    ))
                    .subtitle(&message.message)
                    .build(),
            );
        }

        let content = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        content.append(&header);
        content.append(
            &gtk::ScrolledWindow::builder()
                .vexpand(true)
                .child(&log_list)
                .build(),
        );

        toast_overlay.set_child(Some(&content));

        adw::Window::builder()
            .transient_for(&self.handle.window)
            .modal(true)
            .title(&gettext("Debug log"))
            .default_width(640)
            .default_height(480)
            .content(&toast_overlay)
            .build()
            .show();
    }

    /// Show a dialog with information on this application.
    fn show_about_dialog(&self) {
        let dialog = adw::AboutWindow::builder()
            .transient_for(&self.handle.window)
            .modal(true)
            .application_icon("de.johrpan.musicus")
            .application_name(&gettext("Musicus"))
            .developer_name("Elias Projahn")
            .version(config::VERSION)
            .comments(&gettext("The classical music player and organizer."))
            .website("https://code.johrpan.de/johrpan/musicus")
            .developers(vec![String::from("Elias Projahn <elias@johrpan.de>")])
            .copyright("© 2022 Elias Projahn")
            .license_type(gtk::License::Agpl30)
            .build();

        dialog.show();
    }
}
