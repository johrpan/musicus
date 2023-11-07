use crate::library::MusicusLibrary;
use adw::{
    prelude::*,
    subclass::{navigation_page::NavigationPageImpl, prelude::*},
};
use gtk::glib::{self, Properties};
use std::cell::OnceCell;

mod imp {
    use super::*;

    #[derive(Properties, Debug, Default, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::LibraryManager)]
    #[template(file = "data/ui/library_manager.blp")]
    pub struct LibraryManager {
        #[property(get, construct_only)]
        pub library: OnceCell<MusicusLibrary>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LibraryManager {
        const NAME: &'static str = "MusicusLibraryManager";
        type Type = super::LibraryManager;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for LibraryManager {}

    impl WidgetImpl for LibraryManager {}
    impl NavigationPageImpl for LibraryManager {}
}

glib::wrapper! {
    pub struct LibraryManager(ObjectSubclass<imp::LibraryManager>)
        @extends gtk::Widget, adw::NavigationPage;
}

#[gtk::template_callbacks]
impl LibraryManager {
    pub fn new(library: &MusicusLibrary) -> Self {
        glib::Object::builder().property("library", library).build()
    }
}
