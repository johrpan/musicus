use gtk::{glib, glib::Properties, prelude::*, subclass::prelude::*};
use std::cell::Cell;

mod imp {
    use super::*;

    #[derive(Properties, Debug, Default)]
    #[properties(wrapper_type = super::MusicusPlayer)]
    pub struct MusicusPlayer {
        #[property(get, set)]
        pub active: Cell<bool>,
        #[property(get, set)]
        pub playing: Cell<bool>,
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
        glib::Object::new()
    }

    pub fn play(&self) {
        if !self.imp().active.get() {
            self.set_property("active", true);
        }

        self.set_property("playing", true);
    }

    pub fn pause(&self) {
        self.set_property("playing", false);
    }
}

impl Default for MusicusPlayer {
    fn default() -> Self {
        Self::new()
    }
}
