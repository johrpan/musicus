use std::cell::OnceCell;

use adw::subclass::prelude::*;
use gtk::{
    glib,
    glib::{subclass::Signal, Properties},
    prelude::*,
    ListScrollFlags,
};
use once_cell::sync::Lazy;

use crate::{player::Player, playlist_tile::PlaylistTile};

mod imp {
    use super::*;
    use crate::playlist_item::PlaylistItem;

    #[derive(Properties, Debug, Default, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::PlaylistPage)]
    #[template(file = "data/ui/playlist_page.blp")]
    pub struct PlaylistPage {
        #[property(get, construct_only)]
        pub player: OnceCell<Player>,

        #[template_child]
        pub playlist: TemplateChild<gtk::ListView>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PlaylistPage {
        const NAME: &'static str = "MusicusPlaylistPage";
        type Type = super::PlaylistPage;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for PlaylistPage {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> =
                Lazy::new(|| vec![Signal::builder("close").build()]);

            SIGNALS.as_ref()
        }

        fn constructed(&self) {
            self.parent_constructed();

            self.playlist.set_model(Some(&gtk::NoSelection::new(Some(
                self.player.get().unwrap().playlist(),
            ))));

            let factory = gtk::SignalListItemFactory::new();

            factory.connect_setup(|_, item| {
                let item = item.downcast_ref::<gtk::ListItem>().unwrap();
                item.set_child(Some(&PlaylistTile::new()));
            });

            factory.connect_bind(|_, item| {
                let item = item.downcast_ref::<gtk::ListItem>().unwrap();
                let tile = item.child().and_downcast::<PlaylistTile>().unwrap();
                let playlist_item = item.item().and_downcast::<PlaylistItem>().unwrap();
                tile.set_item(Some(&playlist_item));
            });

            factory.connect_unbind(|_, item| {
                let item = item.downcast_ref::<gtk::ListItem>().unwrap();
                let tile = item.child().and_downcast::<PlaylistTile>().unwrap();
                tile.set_item(None);
            });

            self.playlist.set_factory(Some(&factory));
        }
    }

    impl WidgetImpl for PlaylistPage {}
    impl BinImpl for PlaylistPage {}
}

glib::wrapper! {
    pub struct PlaylistPage(ObjectSubclass<imp::PlaylistPage>)
        @extends gtk::Widget, adw::Bin;
}

#[gtk::template_callbacks]
impl PlaylistPage {
    pub fn new(player: &Player) -> Self {
        glib::Object::builder().property("player", player).build()
    }

    pub fn connect_close<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("close", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            f(&obj);
            None
        })
    }

    pub fn scroll_to_current(&self) {
        self.imp()
            .playlist
            .scroll_to(self.player().current_index(), ListScrollFlags::NONE, None);
    }

    #[template_callback]
    fn select_item(&self, index: u32, _: &gtk::ListView) {
        self.player().set_current_index(index);
    }

    #[template_callback]
    fn close(&self) {
        self.emit_by_name::<()>("close", &[]);
    }
}
