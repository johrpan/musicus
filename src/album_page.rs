use std::cell::OnceCell;

use adw::subclass::prelude::*;
use gtk::{
    gio,
    glib::{self, Properties},
    prelude::*,
};

use crate::{
    db::models::*, editor::album::AlbumEditor, library::Library, player::Player,
    playlist_item::PlaylistItem, recording_tile::RecordingTile, util,
};

mod imp {
    use super::*;

    #[derive(Properties, Debug, Default, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::AlbumPage)]
    #[template(file = "data/ui/album_page.blp")]
    pub struct AlbumPage {
        #[property(get, construct_only)]
        pub toast_overlay: OnceCell<adw::ToastOverlay>,

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

            let obj = self.obj().to_owned();
            let add_to_playlist_action = gio::ActionEntry::builder("add-to-playlist")
                .activate(move |_, _, _| {
                    let playlist = obj
                        .imp()
                        .album
                        .get()
                        .unwrap()
                        .recordings
                        .iter()
                        .map(|r| obj.player().recording_to_playlist(r))
                        .flatten()
                        .collect::<Vec<PlaylistItem>>();

                    if let Err(err) = obj.player().append(playlist) {
                        log::warn!("Failed to add album to the playlits: {err:?}");
                    };
                })
                .build();

            let obj = self.obj().to_owned();
            let edit_action = gio::ActionEntry::builder("edit")
                .activate(move |_, _, _| {
                    obj.navigation().push(&AlbumEditor::new(
                        &obj.navigation(),
                        &obj.library(),
                        Some(&obj.imp().album.get().unwrap().clone()),
                    ));
                })
                .build();

            let obj = self.obj().to_owned();
            let delete_action = gio::ActionEntry::builder("delete")
                .activate(move |_, _, _| {
                    if let Err(err) = obj
                        .library()
                        .delete_album(&obj.imp().album.get().unwrap().album_id)
                    {
                        util::error_toast("Failed to delete album", err, &obj.toast_overlay());
                    }
                })
                .build();

            let actions = gio::SimpleActionGroup::new();
            actions.add_action_entries([add_to_playlist_action, edit_action, delete_action]);
            self.obj().insert_action_group("album", Some(&actions));
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
        toast_overlay: &adw::ToastOverlay,
        navigation: &adw::NavigationView,
        library: &Library,
        player: &Player,
        album: Album,
    ) -> Self {
        let obj: Self = glib::Object::builder()
            .property("toast-overlay", toast_overlay)
            .property("navigation", navigation)
            .property("library", library)
            .property("player", player)
            .build();

        obj.imp().title_label.set_label(&album.to_string());
        obj.imp()
            .subtitle_label
            .set_label(&album.performers_string());

        for recording in &album.recordings {
            obj.imp().recordings_flow_box.append(&RecordingTile::new(
                toast_overlay,
                navigation,
                library,
                player,
                recording,
            ));
        }

        obj.imp().album.set(album).unwrap();

        obj
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
