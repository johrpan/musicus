use gtk::{glib, glib::Properties, prelude::*, subclass::prelude::*};
use std::{
    cell::{Cell, OnceCell},
    path::{Path, PathBuf},
};

mod imp {
    use super::*;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::PlaylistItem)]
    pub struct PlaylistItem {
        #[property(get, set)]
        pub is_playing: Cell<bool>,

        #[property(get, construct_only)]
        pub is_title: OnceCell<bool>,

        #[property(get, construct_only)]
        pub title: OnceCell<String>,

        #[property(get, construct_only, nullable)]
        pub performers: OnceCell<Option<String>>,

        #[property(get, construct_only, nullable)]
        pub part_title: OnceCell<Option<String>>,

        #[property(get, construct_only)]
        pub path: OnceCell<PathBuf>,

        #[property(get, construct_only)]
        pub track_id: OnceCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PlaylistItem {
        const NAME: &'static str = "MusicusPlaylistItem";
        type Type = super::PlaylistItem;
    }

    #[glib::derived_properties]
    impl ObjectImpl for PlaylistItem {}
}

glib::wrapper! {
    pub struct PlaylistItem(ObjectSubclass<imp::PlaylistItem>);
}

impl PlaylistItem {
    pub fn new(
        is_title: bool,
        title: &str,
        performers: Option<&str>,
        part_title: Option<&str>,
        path: impl AsRef<Path>,
        track_id: &str,
    ) -> Self {
        glib::Object::builder()
            .property("is-title", is_title)
            .property("title", title)
            .property("performers", performers)
            .property("part-title", part_title)
            .property("path", path.as_ref())
            .property("track-id", track_id)
            .build()
    }

    pub fn make_title(&self) -> String {
        let mut title = self.title();

        if let Some(part_title) = self.part_title() {
            title.push_str(": ");
            title.push_str(&part_title);
        }

        title
    }

    pub fn make_subtitle(&self) -> Option<String> {
        self.performers()
    }
}
