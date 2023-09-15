use crate::welcome_page::MusicusWelcomePage;

use adw::subclass::prelude::*;
use gtk::{gio, glib, prelude::*};

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

#[gtk::template_callbacks]
impl MusicusWindow {
    pub fn new<P: glib::IsA<gtk::Application>>(application: &P) -> Self {
        glib::Object::builder()
            .property("application", application)
            .build()
    }

    #[template_callback]
    async fn set_library_folder(&self, folder: &gio::File) {
        let path = folder.path();
        log::info!("{path:?}");
    }
}
