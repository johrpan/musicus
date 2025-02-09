use std::cell::{Cell, RefCell};

use anyhow::Result;
use gtk::{glib, glib::Properties, prelude::*, subclass::prelude::*};
use serde::{Deserialize, Serialize};

use crate::library::LibraryQuery;

mod imp {
    use super::*;

    #[derive(Properties, Serialize, Deserialize, Default)]
    #[properties(wrapper_type = super::Program)]
    pub struct Program {
        #[property(get, set)]
        pub title: RefCell<Option<String>>,

        #[property(get, set)]
        pub description: RefCell<Option<String>>,

        #[property(get, set, builder(ProgramDesign::default()))]
        pub design: Cell<ProgramDesign>,

        #[property(get, set)]
        pub composer_id: RefCell<Option<String>>,

        #[property(get, set)]
        pub performer_id: RefCell<Option<String>>,

        #[property(get, set)]
        pub ensemble_id: RefCell<Option<String>>,

        #[property(get, set)]
        pub work_id: RefCell<Option<String>>,

        #[property(get, set)]
        pub album_id: RefCell<Option<String>>,

        #[property(get, set)]
        pub prefer_recently_added: Cell<f64>,

        #[property(get, set)]
        pub prefer_least_recently_played: Cell<f64>,

        #[property(get, set)]
        pub play_full_recordings: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Program {
        const NAME: &'static str = "MusicusProgram";
        type Type = super::Program;
    }

    #[glib::derived_properties]
    impl ObjectImpl for Program {}
}

glib::wrapper! {
    pub struct Program(ObjectSubclass<imp::Program>);
}

impl Program {
    pub fn new(title: &str, description: &str, design: ProgramDesign) -> Self {
        glib::Object::builder()
            .property("title", title)
            .property("description", description)
            .property("design", design)
            .build()
    }

    pub fn from_query(query: LibraryQuery) -> Self {
        glib::Object::builder()
            .property("composer-id", query.composer.map(|p| p.person_id))
            .property("performer-id", query.performer.map(|p| p.person_id))
            .property("ensemble-id", query.ensemble.map(|e| e.ensemble_id))
            .property("prefer-recently-added", 0.25)
            .property("prefer-least-recently-played", 0.5)
            .property("play-full-recordings", true)
            .build()
    }

    pub fn deserialize(input: &str) -> Result<Self> {
        let data: imp::Program = serde_json::from_str(input)?;

        let obj = glib::Object::builder()
            .property("title", &*data.title.borrow())
            .property("description", &*data.description.borrow())
            .property("design", data.design.get())
            .property("prefer-recently-added", data.prefer_recently_added.get())
            .property("prefer-least-recently-played", data.prefer_least_recently_played.get())
            .property("play-full-recordings", data.play_full_recordings.get())
            .build();

        Ok(obj)
    }

    pub fn serialize(&self) -> String {
        serde_json::to_string(self.imp()).unwrap()
    }
}

#[derive(glib::Enum, Serialize, Deserialize, Eq, PartialEq, Clone, Copy, Debug)]
#[enum_type(name = "MusicusProgramDesign")]
pub enum ProgramDesign {
    Generic,
    Program1,
    Program2,
    Program3,
    Program4,
    Program5,
    Program6,
}

impl Default for ProgramDesign {
    fn default() -> Self {
        Self::Generic
    }
}
