use adw::subclass::{navigation_page::NavigationPageImpl, prelude::*};
use gtk::glib;

mod imp {
    use crate::tile::MusicusTile;

    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/de/johrpan/musicus/home_page.ui")]
    pub struct MusicusHomePage {
        #[template_child]
        pub search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub persons_flow_box: TemplateChild<gtk::FlowBox>,
        #[template_child]
        pub works_flow_box: TemplateChild<gtk::FlowBox>,
        #[template_child]
        pub recordings_flow_box: TemplateChild<gtk::FlowBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MusicusHomePage {
        const NAME: &'static str = "MusicusHomePage";
        type Type = super::MusicusHomePage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for MusicusHomePage {
        fn constructed(&self) {
            self.parent_constructed();
            self.search_entry
                .set_key_capture_widget(Some(self.obj().as_ref()));

            for _ in 0..9 {
                self.persons_flow_box.append(&MusicusTile::new());
                self.works_flow_box.append(&MusicusTile::new());
                self.recordings_flow_box.append(&MusicusTile::new());
            }
        }
    }

    impl WidgetImpl for MusicusHomePage {}
    impl NavigationPageImpl for MusicusHomePage {}
}

glib::wrapper! {
    pub struct MusicusHomePage(ObjectSubclass<imp::MusicusHomePage>)
        @extends gtk::Widget, adw::NavigationPage;
}

impl MusicusHomePage {
    pub fn new() -> Self {
        glib::Object::new()
    }
}
