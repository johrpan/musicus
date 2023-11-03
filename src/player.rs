use crate::playlist_item::PlaylistItem;
use gtk::{gio, glib, glib::Properties, prelude::*, subclass::prelude::*};
use std::cell::{Cell, OnceCell};

mod imp {
    use super::*;

    #[derive(Properties, Debug, Default)]
    #[properties(wrapper_type = super::MusicusPlayer)]
    pub struct MusicusPlayer {
        #[property(get, set)]
        pub active: Cell<bool>,
        #[property(get, set)]
        pub playing: Cell<bool>,
        #[property(get, construct_only)]
        pub playlist: OnceCell<gio::ListStore>,
        #[property(get, set = Self::set_current_index)]
        pub current_index: Cell<u32>,
        #[property(get, set)]
        pub current_time: Cell<u32>,
        #[property(get, set)]
        pub remaining_time: Cell<u32>,
        #[property(get, set = Self::set_position)]
        pub position: Cell<f64>,
    }

    impl MusicusPlayer {
        pub fn set_current_index(&self, index: u32) {
            let playlist = self.playlist.get().unwrap();

            if let Some(item) = playlist.item(self.current_index.get()) {
                item.downcast::<PlaylistItem>()
                    .unwrap()
                    .set_is_playing(false);
            }

            self.current_index.set(index);

            if let Some(item) = playlist.item(index) {
                item.downcast::<PlaylistItem>()
                    .unwrap()
                    .set_is_playing(true);
            }
        }

        pub fn set_position(&self, position: f64) {
            self.position.set(position);
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MusicusPlayer {
        const NAME: &'static str = "MusicusPlayer";
        type Type = super::MusicusPlayer;
    }

    #[glib::derived_properties]
    impl ObjectImpl for MusicusPlayer {}
}

glib::wrapper! {
    pub struct MusicusPlayer(ObjectSubclass<imp::MusicusPlayer>);
}

impl MusicusPlayer {
    pub fn new() -> Self {
        glib::Object::builder()
            .property("active", false)
            .property("playing", false)
            .property("playlist", gio::ListStore::new::<PlaylistItem>())
            .property("current-index", 0u32)
            .property("current-time", 0u32)
            .property("remaining-time", 10000u32)
            .property("position", 0.0)
            .build()
    }

    pub fn append(&self, tracks: Vec<PlaylistItem>) {
        let playlist = self.playlist();

        for track in tracks {
            playlist.append(&track);
        }

        self.set_active(true);
    }

    pub fn play(&self) {
        self.set_playing(true)
    }

    pub fn pause(&self) {
        self.set_playing(false)
    }

    pub fn current_item(&self) -> Option<PlaylistItem> {
        let imp = self.imp();
        imp.playlist
            .get()
            .unwrap()
            .item(imp.current_index.get())
            .and_downcast::<PlaylistItem>()
    }

    pub fn next(&self) {
        if self.current_index() < self.playlist().n_items() - 1 {
            self.set_current_index(self.current_index() + 1);
        }
    }

    pub fn previous(&self) {
        if self.current_index() > 0 {
            self.set_current_index(self.current_index() - 1);
        }
    }
}

impl Default for MusicusPlayer {
    fn default() -> Self {
        Self::new()
    }
}
