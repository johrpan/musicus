use crate::navigator::Navigator;
use crate::screens::{MainScreen, WelcomeScreen};
use gtk::prelude::*;
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

        let loading_screen = gtk::BoxBuilder::new()
            .orientation(gtk::Orientation::Vertical)
            .build();

        let header = adw::HeaderBarBuilder::new()
            .title_widget(&adw::WindowTitle::new(Some("Musicus"), None))
            .build();

        let spinner = gtk::SpinnerBuilder::new()
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
        adw::prelude::ApplicationWindowExt::set_child(&window, Some(&navigator.widget));

        let this = Rc::new(Self {
            backend,
            window,
            navigator,
        });

        spawn!(@clone this, async move {
            while let Ok(state) = this.backend.next_state().await {
                match state {
                    BackendState::Loading => this.navigator.reset(),
                    BackendState::NoMusicLibrary => this.show_welcome_screen(),
                    BackendState::Ready => this.show_main_screen(),
                }
            }
        });

        spawn!(@clone this, async move {
            // This is not done in the async block above, because backend state changes may happen
            // while this method is running.
            this.backend.init().await.unwrap();
        });

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
