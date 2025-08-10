use std::{
    cell::{Cell, RefCell},
    str::FromStr,
};

use anyhow::Result;
use gtk::{gio, glib, glib::Properties, prelude::*, subclass::prelude::*};
use serde::{Deserialize, Serialize};

use crate::{config, library::LibraryQuery};

mod imp {
    use super::*;

    #[derive(Properties, Serialize, Deserialize, Default)]
    #[properties(wrapper_type = super::Program)]
    #[serde(default)]
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
        pub instrument_id: RefCell<Option<String>>,

        #[property(get, set)]
        pub work_id: RefCell<Option<String>>,

        #[property(get, set)]
        pub album_id: RefCell<Option<String>>,

        #[property(get, set)]
        pub prefer_recently_added: Cell<f64>,

        #[property(get, set)]
        pub prefer_least_recently_played: Cell<f64>,

        #[property(get, set)]
        pub avoid_repeated_composers: Cell<i32>,

        #[property(get, set)]
        pub avoid_repeated_instruments: Cell<i32>,

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
        let settings = gio::Settings::new(config::APP_ID);

        glib::Object::builder()
            .property(
                "composer-id",
                query.composer.as_ref().map(|p| p.person_id.clone()),
            )
            .property("performer-id", query.performer.map(|p| p.person_id))
            .property("ensemble-id", query.ensemble.map(|e| e.ensemble_id))
            .property(
                "instrument-id",
                query.instrument.as_ref().map(|i| i.instrument_id.clone()),
            )
            .property("work-id", query.work.as_ref().map(|w| w.work_id.clone()))
            .property(
                "prefer-recently-added",
                settings.int("prefer-recently-added") as f64 / 100.0,
            )
            .property(
                "prefer-least-recently-played",
                settings.int("prefer-least-recently-played") as f64 / 100.0,
            )
            .property(
                "avoid-repeated-composers",
                if query.composer.is_none() && query.work.is_none() {
                    settings.int("avoid-repeated-composers")
                } else {
                    0
                },
            )
            .property(
                "avoid-repeated-instruments",
                if query.instrument.is_none() && query.work.is_none() {
                    settings.int("avoid-repeated-instruments")
                } else {
                    0
                },
            )
            .property(
                "play-full-recordings",
                settings.boolean("play-full-recordings"),
            )
            .build()
    }

    pub fn deserialize(input: &str) -> Result<Self> {
        let data: imp::Program = serde_json::from_str(input)?;

        let obj = glib::Object::builder()
            .property("title", &*data.title.borrow())
            .property("description", &*data.description.borrow())
            .property("design", data.design.get())
            .property("prefer-recently-added", data.prefer_recently_added.get())
            .property(
                "prefer-least-recently-played",
                data.prefer_least_recently_played.get(),
            )
            .property(
                "avoid-repeated-composers",
                data.avoid_repeated_composers.get(),
            )
            .property(
                "avoid-repeated-instruments",
                data.avoid_repeated_instruments.get(),
            )
            .property("play-full-recordings", data.play_full_recordings.get())
            .build();

        Ok(obj)
    }

    pub fn serialize(&self) -> String {
        serde_json::to_string(self.imp()).unwrap()
    }
}

impl Default for Program {
    fn default() -> Self {
        glib::Object::new()
    }
}

#[derive(glib::Enum, Serialize, Deserialize, Eq, PartialEq, Clone, Copy, Debug)]
#[enum_type(name = "MusicusProgramDesign")]
pub enum ProgramDesign {
    Default,
    Blue,
    Teal,
    Green,
    Yellow,
    Orange,
    Red,
    Pink,
    Purple,
    Slate,
}

impl ProgramDesign {
    pub fn css_class(&self) -> String {
        self.to_string()
    }
}

impl Default for ProgramDesign {
    fn default() -> Self {
        Self::Default
    }
}

impl ToString for ProgramDesign {
    fn to_string(&self) -> String {
        String::from(match self {
            ProgramDesign::Default => "default",
            ProgramDesign::Blue => "blue",
            ProgramDesign::Teal => "teal",
            ProgramDesign::Green => "green",
            ProgramDesign::Yellow => "yellow",
            ProgramDesign::Orange => "orange",
            ProgramDesign::Red => "red",
            ProgramDesign::Pink => "pink",
            ProgramDesign::Purple => "purple",
            ProgramDesign::Slate => "slate",
        })
    }
}

impl FromStr for ProgramDesign {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, ()> {
        match s {
            "default" => Ok(ProgramDesign::Default),
            "blue" => Ok(ProgramDesign::Blue),
            "teal" => Ok(ProgramDesign::Teal),
            "green" => Ok(ProgramDesign::Green),
            "yellow" => Ok(ProgramDesign::Yellow),
            "orange" => Ok(ProgramDesign::Orange),
            "red" => Ok(ProgramDesign::Red),
            "pink" => Ok(ProgramDesign::Pink),
            "purple" => Ok(ProgramDesign::Purple),
            "slate" => Ok(ProgramDesign::Slate),
            _ => Err(()),
        }
    }
}
