use std::cell::OnceCell;

use adw::subclass::prelude::*;
use gtk::{
    glib::{self, Properties},
    prelude::*,
};

use crate::{
    db::models::*, editor::album::AlbumEditor, library::Library, player::Player,
    playlist_item::PlaylistItem, recording_tile::RecordingTile,
};

mod imp {
    use super::*;

    #[derive(Properties, Debug, Default, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::AlbumPage)]
    #[template(file = "data/ui/album_page.blp")]
    pub struct AlbumPage {
        #[property(get, construct_only)]
        pub navigation: OnceCell<adw::NavigationView>,

        #[property(get, construct_only)]
        pub library: OnceCell<Library>,

        #[property(get, construct_only)]
        pub player: OnceCell<Player>,

        pub album: OnceCell<Album>,

        #[template_child]
        pub title_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub subtitle_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub recordings_flow_box: TemplateChild<gtk::FlowBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AlbumPage {
        const NAME: &'static str = "MusicusAlbumPage";
        type Type = super::AlbumPage;
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
    impl ObjectImpl for AlbumPage {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for AlbumPage {}
    impl NavigationPageImpl for AlbumPage {}
}

glib::wrapper! {
    pub struct AlbumPage(ObjectSubclass<imp::AlbumPage>)
        @extends gtk::Widget, adw::NavigationPage;
}

#[gtk::template_callbacks]
impl AlbumPage {
    pub fn new(
        navigation: &adw::NavigationView,
        library: &Library,
        player: &Player,
        album: Album,
    ) -> Self {
        let obj: Self = glib::Object::builder()
            .property("navigation", navigation)
            .property("library", library)
            .property("player", player)
            .build();

        obj.imp().title_label.set_label(&album.to_string());
        obj.imp().subtitle_label.set_label(&album.performers_string());

        for recording in &album.recordings {
            obj.imp()
                .recordings_flow_box
                .append(&RecordingTile::new(navigation, library, recording));
        }

        obj.imp().album.set(album).unwrap();

        obj
    }

    #[template_callback]
    fn edit_button_clicked(&self) {
        self.navigation().push(&AlbumEditor::new(
            &self.navigation(),
            &self.library(),
            Some(&self.imp().album.get().unwrap().clone()),
        ));
    }

    #[template_callback]
    fn play_button_clicked(&self) {
        let playlist = self
            .imp()
            .album
            .get()
            .unwrap()
            .recordings
            .iter()
            .map(|r| self.player().recording_to_playlist(r))
            .flatten()
            .collect::<Vec<PlaylistItem>>();

        self.player().append_and_play(playlist);
    }

    #[template_callback]
    fn recording_selected(&self, tile: &gtk::FlowBoxChild) {
        let playlist = self
            .player()
            .recording_to_playlist(tile.downcast_ref::<RecordingTile>().unwrap().recording());
        self.player().append_and_play(playlist);
    }
}
