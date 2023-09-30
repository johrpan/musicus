use crate::{
    library::MusicusLibrary, player::MusicusPlayer, search_entry::MusicusSearchEntry,
    tile::MusicusTile,
};
use adw::subclass::{navigation_page::NavigationPageImpl, prelude::*};
use gtk::{glib, glib::Properties, prelude::*};
use std::cell::{OnceCell, RefCell};

mod imp {
    use super::*;

    #[derive(Properties, Debug, Default, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::MusicusHomePage)]
    #[template(file = "data/ui/home_page.blp")]
    pub struct MusicusHomePage {
        #[property(get, set)]
        pub player: RefCell<MusicusPlayer>,

        pub library: OnceCell<MusicusLibrary>,

        #[template_child]
        pub search_entry: TemplateChild<MusicusSearchEntry>,
        #[template_child]
        pub persons_flow_box: TemplateChild<gtk::FlowBox>,
        #[template_child]
        pub works_flow_box: TemplateChild<gtk::FlowBox>,
        #[template_child]
        pub recordings_flow_box: TemplateChild<gtk::FlowBox>,
        #[template_child]
        pub play_button: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MusicusHomePage {
        const NAME: &'static str = "MusicusHomePage";
        type Type = super::MusicusHomePage;
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
    impl ObjectImpl for MusicusHomePage {
        fn constructed(&self) {
            self.parent_constructed();

            self.search_entry.set_key_capture_widget(&*self.obj());

            self.player
                .borrow()
                .bind_property("active", &self.play_button.get(), "visible")
                .invert_boolean()
                .sync_create()
                .build();

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

#[gtk::template_callbacks]
impl MusicusHomePage {
    pub fn new(library: &MusicusLibrary, player: &MusicusPlayer) -> Self {
        let obj: MusicusHomePage = glib::Object::builder().property("player", player).build();
        obj.imp().library.set(library.to_owned()).unwrap();
        obj
    }

    #[template_callback]
    fn play(&self, _: &gtk::Button) {
        log::info!("Play button clicked");
        self.imp().player.borrow().play();
    }

    #[template_callback]
    fn select(&self, search_entry: &MusicusSearchEntry) {
        search_entry.add_tag("Tag");
    }
}
