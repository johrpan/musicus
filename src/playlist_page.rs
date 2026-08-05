use std::cell::OnceCell;

use adw::subclass::prelude::*;
use gtk::{
    gio, glib,
    glib::{subclass::Signal, Properties},
    prelude::*,
    ListScrollFlags,
};
use once_cell::sync::Lazy;

use crate::{
    player::Player, playlist_tile::PlaylistTile, program::Program, program_section::ProgramSection,
};

mod imp {
    use super::*;
    use crate::playlist_item::PlaylistItem;

    #[derive(Properties, Debug, Default, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::PlaylistPage)]
    #[template(file = "data/ui/playlist_page.blp")]
    pub struct PlaylistPage {
        #[property(get, construct_only)]
        pub player: OnceCell<Player>,

        /// Contains the active program, if there is one, so that it is shown as the last row
        /// after the playlist items.
        pub program_model: OnceCell<gio::ListStore>,

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

            let player = self.player.get().unwrap();

            let program_model = gio::ListStore::new::<Program>();
            let models = gio::ListStore::with_type(gio::ListModel::static_type());
            models.append(&player.playlist());
            models.append(&program_model);
            self.program_model.set(program_model).unwrap();

            self.playlist.set_model(Some(&gtk::NoSelection::new(Some(
                gtk::FlattenListModel::new(Some(models)),
            ))));

            let factory = gtk::SignalListItemFactory::new();

            let player = player.to_owned();
            factory.connect_bind(move |_, item| {
                let item = item.downcast_ref::<gtk::ListItem>().unwrap();

                if let Some(playlist_item) = item.item().and_downcast::<PlaylistItem>() {
                    let tile = match item.child().and_downcast::<PlaylistTile>() {
                        Some(tile) => tile,
                        None => {
                            let tile = PlaylistTile::new();
                            item.set_child(Some(&tile));
                            tile
                        }
                    };

                    tile.set_item(Some(&playlist_item));

                    // The list item may have been used for the program before.
                    item.set_activatable(true);
                    item.set_selectable(true);
                } else if let Some(program) = item.item().and_downcast::<Program>() {
                    let section = match item.child().and_downcast::<ProgramSection>() {
                        Some(section) => section,
                        None => {
                            let section = ProgramSection::new(&player);
                            item.set_child(Some(&section));
                            section
                        }
                    };

                    section.set_program(&program);

                    // The program is not a playlist item that could be played.
                    item.set_activatable(false);
                    item.set_selectable(false);
                }
            });

            factory.connect_unbind(|_, item| {
                let item = item.downcast_ref::<gtk::ListItem>().unwrap();
                if let Some(tile) = item.child().and_downcast::<PlaylistTile>() {
                    tile.set_item(None);
                }
            });

            self.playlist.set_factory(Some(&factory));

            self.update_program();

            let obj = self.obj().to_owned();
            self.player
                .get()
                .unwrap()
                .connect_program_notify(move |_| obj.imp().update_program());
        }
    }

    impl WidgetImpl for PlaylistPage {}
    impl BinImpl for PlaylistPage {}

    impl PlaylistPage {
        /// Show the active program after the playlist items or nothing, if there is none.
        fn update_program(&self) {
            let program_model = self.program_model.get().unwrap();
            program_model.remove_all();

            if let Some(program) = self.player.get().unwrap().program() {
                program_model.append(&program);
            }
        }
    }
}

glib::wrapper! {
    pub struct PlaylistPage(ObjectSubclass<imp::PlaylistPage>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
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
