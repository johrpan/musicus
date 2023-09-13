use crate::welcome_page::MusicusWelcomePage;

use adw::subclass::prelude::*;
use gtk::{gio, glib};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/de/johrpan/musicus/window.ui")]
    pub struct MusicusWindow {
        #[template_child]
        pub navigation_view: TemplateChild<adw::NavigationView>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MusicusWindow {
        const NAME: &'static str = "MusicusWindow";
        type Type = super::MusicusWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for MusicusWindow {
        fn constructed(&self) {
            self.parent_constructed();

            self.navigation_view.add(&MusicusWelcomePage::new());
        }
    }

    impl WidgetImpl for MusicusWindow {}
    impl WindowImpl for MusicusWindow {}
    impl ApplicationWindowImpl for MusicusWindow {}
    impl AdwApplicationWindowImpl for MusicusWindow {}
}

glib::wrapper! {
    pub struct MusicusWindow(ObjectSubclass<imp::MusicusWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl MusicusWindow {
    pub fn new<P: glib::IsA<gtk::Application>>(application: &P) -> Self {
        glib::Object::builder()
            .property("application", application)
            .build()
    }
}
