use crate::{player::MusicusPlayer, playlist_tile::PlaylistTile};
use adw::subclass::prelude::*;
use gtk::{glib, glib::subclass::Signal, glib::Properties, prelude::*};
use once_cell::sync::Lazy;
use std::cell::OnceCell;

mod imp {
    use crate::playlist_item::PlaylistItem;

    use super::*;

    #[derive(Properties, Debug, Default, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::MusicusPlayer)]
    #[template(file = "data/ui/playlist_page.blp")]
    pub struct MusicusPlaylistPage {
        #[property(get, construct_only)]
        pub player: OnceCell<MusicusPlayer>,

        #[template_child]
        pub playlist: TemplateChild<gtk::ListView>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MusicusPlaylistPage {
        const NAME: &'static str = "MusicusPlaylistPage";
        type Type = super::MusicusPlaylistPage;
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
    impl ObjectImpl for MusicusPlaylistPage {
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
                tile.set_item(&playlist_item);
            });

            self.playlist.set_factory(Some(&factory));
        }
    }

    impl WidgetImpl for MusicusPlaylistPage {}
    impl BinImpl for MusicusPlaylistPage {}
}

glib::wrapper! {
    pub struct MusicusPlaylistPage(ObjectSubclass<imp::MusicusPlaylistPage>)
        @extends gtk::Widget, adw::Bin;
}

#[gtk::template_callbacks]
impl MusicusPlaylistPage {
    pub fn new(player: &MusicusPlayer) -> Self {
        glib::Object::builder().property("player", player).build()
    }

    pub fn connect_close<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("close", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            f(&obj);
            None
        })
    }

    #[template_callback]
    fn close(&self, _: &gtk::Button) {
        self.emit_by_name::<()>("close", &[]);
    }
}
