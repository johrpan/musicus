use crate::{
    library::{LibraryQuery, MusicusLibrary},
    player::MusicusPlayer,
    search_entry::MusicusSearchEntry,
    tile::MusicusTile,
};
use adw::subclass::{navigation_page::NavigationPageImpl, prelude::*};
use gtk::{
    glib::{self, clone, Properties},
    prelude::*,
};
use std::cell::OnceCell;

mod imp {
    use super::*;

    #[derive(Properties, Debug, Default, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::MusicusHomePage)]
    #[template(file = "data/ui/home_page.blp")]
    pub struct MusicusHomePage {
        #[property(get, construct_only)]
        pub library: OnceCell<MusicusLibrary>,

        #[property(get, construct_only)]
        pub player: OnceCell<MusicusPlayer>,

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

            self.search_entry
                .connect_query_changed(clone!(@weak self as _self => move |entry| {
                    _self.obj().query(&entry.query());
                }));

            self.player
                .get()
                .unwrap()
                .bind_property("active", &self.play_button.get(), "visible")
                .invert_boolean()
                .sync_create()
                .build();

            self.obj().query(&LibraryQuery::default());

            for _ in 0..9 {
                self.works_flow_box.append(&MusicusTile::with_title("Test"));
                self.recordings_flow_box
                    .append(&MusicusTile::with_title("Test"));
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
        glib::Object::builder()
            .property("library", library)
            .property("player", player)
            .build()
    }

    #[template_callback]
    fn play(&self, _: &gtk::Button) {
        log::info!("Play button clicked");
        self.player().play();
    }

    #[template_callback]
    fn select(&self, search_entry: &MusicusSearchEntry) {
        search_entry.add_tag("Tag");
    }

    fn query(&self, query: &LibraryQuery) {
        let results = self.library().query(query);

        clear_flowbox(&self.imp().persons_flow_box);
        for person in results.persons {
            self.imp()
                .persons_flow_box
                .append(&MusicusTile::with_title(&person.name_fl()));
        }
    }
}

fn clear_flowbox(flowbox: &gtk::FlowBox) {
    while let Some(widget) = flowbox.first_child() {
        flowbox.remove(&widget);
    }
}
