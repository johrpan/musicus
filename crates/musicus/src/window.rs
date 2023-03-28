use crate::navigator::Navigator;
use crate::screens::{MainScreen, WelcomeScreen};

use adw::prelude::*;
use glib::clone;

use musicus_backend::{Backend, BackendState};
use std::rc::Rc;

/// The main window of this application. This will also handle initializing and managing the
/// backend.
pub struct Window {
    window: adw::ApplicationWindow,
    backend: Rc<Backend>,
    navigator: Rc<Navigator>,
}

impl Window {
    pub fn new(app: &gtk::Application) -> Rc<Self> {
        let backend = Rc::new(Backend::new());

        let window = adw::ApplicationWindow::new(app);
        window.set_title(Some("Musicus"));
        window.set_default_size(1000, 707);

        let loading_screen = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        let header = gtk::HeaderBar::builder()
            .title_widget(&adw::WindowTitle::new("Musicus", ""))
            .build();

        let spinner = gtk::Spinner::builder()
            .hexpand(true)
            .vexpand(true)
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .width_request(32)
            .height_request(32)
            .spinning(true)
            .build();

        loading_screen.append(&header);
        loading_screen.append(&spinner);

        let navigator = Navigator::new(Rc::clone(&backend), &window, &loading_screen);
        window.set_content(Some(&navigator.widget));

        let this = Rc::new(Self {
            backend,
            window,
            navigator,
        });

        // Listen for backend state changes.
        this.backend
            .set_state_cb(clone!(@weak this => move |state| {
                match state {
                    BackendState::Loading => this.navigator.reset(),
                    BackendState::NoMusicLibrary => this.show_welcome_screen(),
                    BackendState::Ready => this.show_main_screen(),
                }
            }));

        // Initialize the backend.
        Rc::clone(&this.backend).init().unwrap();

        this
    }

    /// Present this window to the user.
    pub fn present(&self) {
        self.window.present();
    }

    /// Replace the current screen with the welcome screen.
    fn show_welcome_screen(self: &Rc<Self>) {
        let this = self;
        spawn!(@clone this, async move {
            replace!(this.navigator, WelcomeScreen).await;
        });
    }

    /// Replace the current screen with the main screen.
    fn show_main_screen(self: &Rc<Self>) {
        let this = self;
        spawn!(@clone this, async move {
            replace!(this.navigator, MainScreen).await;
        });
    }
}
