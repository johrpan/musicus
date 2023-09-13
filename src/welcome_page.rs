use adw::subclass::{navigation_page::NavigationPageImpl, prelude::*};
use gtk::glib;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/de/johrpan/musicus/welcome_page.ui")]
    pub struct MusicusWelcomePage {
        #[template_child]
        pub choose_library_button: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MusicusWelcomePage {
        const NAME: &'static str = "MusicusWelcomePage";
        type Type = super::MusicusWelcomePage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for MusicusWelcomePage {}
    impl WidgetImpl for MusicusWelcomePage {}
    impl NavigationPageImpl for MusicusWelcomePage {}
}

glib::wrapper! {
    pub struct MusicusWelcomePage(ObjectSubclass<imp::MusicusWelcomePage>)
        @extends gtk::Widget, adw::NavigationPage;
}

impl MusicusWelcomePage {
    pub fn new() -> Self {
        glib::Object::new()
    }
}
