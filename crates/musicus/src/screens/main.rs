use super::{EnsembleScreen, PersonScreen, PlayerScreen};
use crate::config;
use crate::import::SourceSelector;
use crate::navigator::{Navigator, NavigatorWindow, NavigationHandle, Screen};
use crate::preferences::Preferences;
use crate::widgets::{List, PlayerBar, Widget};
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use libadwaita::prelude::*;
use musicus_backend::db::{Ensemble, Person};
use std::cell::RefCell;
use std::rc::Rc;

/// Either a person or an ensemble to be shown in the list.
#[derive(Clone, Debug)]
pub enum PersonOrEnsemble {
    Person(Person),
    Ensemble(Ensemble),
}

impl PersonOrEnsemble {
    /// Get a short textual representation of the item.
    pub fn get_title(&self) -> String {
        match self {
            PersonOrEnsemble::Person(person) => person.name_lf(),
            PersonOrEnsemble::Ensemble(ensemble) => ensemble.name.clone(),
        }
    }
}

/// The main screen of the app, once it's set up and finished loading. The screen assumes that the
/// music library and the player are available and initialized.
pub struct MainScreen {
    handle: NavigationHandle<()>,
    widget: gtk::Box,
    leaflet: libadwaita::Leaflet,
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
        get_widget!(builder, libadwaita::Leaflet, leaflet);
        get_widget!(builder, gtk::Button, add_button);
        get_widget!(builder, gtk::SearchEntry, search_entry);
        get_widget!(builder, gtk::Stack, stack);
        get_widget!(builder, gtk::ScrolledWindow, scroll);
        get_widget!(builder, gtk::Box, empty_screen);

        let actions = gio::SimpleActionGroup::new();
        let preferences_action = gio::SimpleAction::new("preferences", None);
        let about_action = gio::SimpleAction::new("about", None);
        actions.add_action(&preferences_action);
        actions.add_action(&about_action);
        widget.insert_action_group("widget", Some(&actions));

        let poe_list = List::new();
        poe_list.widget.add_css_class("navigation-sidebar");
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

        preferences_action.connect_activate(clone!(@weak this => move |_, _| {
            Preferences::new(Rc::clone(&this.handle.backend), &this.handle.window).show();
        }));

        about_action.connect_activate(clone!(@weak this => move |_, _| {
            this.show_about_dialog();
        }));

        add_button.connect_clicked(clone!(@weak this => move |_| {
            spawn!(@clone this, async move {
                let window = NavigatorWindow::new(Rc::clone(&this.handle.backend));
                replace!(window.navigator, SourceSelector).await;
            });
        }));

        this.search_entry.connect_search_changed(clone!(@weak this => move |_| {
            this.poe_list.invalidate_filter();
        }));

        this.poe_list.set_make_widget_cb(clone!(@weak this => move |index| {
            let poe = &this.poes.borrow()[index];

            let row = libadwaita::ActionRow::new();
            row.set_activatable(true);
            row.set_title(Some(&poe.get_title()));

            let poe = poe.to_owned();
            row.connect_activated(clone!(@weak this => move |_| {
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

        this.poe_list.set_filter_cb(clone!(@weak this => move |index| {
            let poe = &this.poes.borrow()[index];
            let search = this.search_entry.get_text().unwrap().to_string().to_lowercase();
            let title = poe.get_title().to_lowercase();
            search.is_empty() || title.contains(&search)
        }));

        this.navigator.set_back_cb(clone!(@weak this => move || {
            this.leaflet.set_visible_child_name("sidebar");
        }));

        player_bar.set_playlist_cb(clone!(@weak this => move || {
            spawn!(@clone this, async move {
                push!(this.handle, PlayerScreen).await;
            });
        }));

        // Load the content asynchronously.

        spawn!(@clone this, async move {
            let mut poes = Vec::new();

            let persons = this.handle.backend.db().get_persons().await.unwrap();
            let ensembles = this.handle.backend.db().get_ensembles().await.unwrap();

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
    /// Show a dialog with information on this application.
    fn show_about_dialog(&self) {
        let dialog = gtk::AboutDialogBuilder::new()
            .transient_for(&self.handle.window)
            .modal(true)
            .logo_icon_name("de.johrpan.musicus")
            .program_name(&gettext("Musicus"))
            .version(config::VERSION)
            .comments(&gettext("The classical music player and organizer."))
            .website("https://github.com/johrpan/musicus")
            .website_label(&gettext("Further information and source code"))
            .copyright("© 2020 Elias Projahn")
            .license_type(gtk::License::Agpl30)
            .authors(vec![String::from("Elias Projahn <johrpan@gmail.com>")])
            .build();

        dialog.show();
    }
}
