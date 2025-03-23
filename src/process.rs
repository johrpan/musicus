use std::cell::{Cell, OnceCell, RefCell};

use anyhow::Result;
use gtk::{
    glib::{self, Properties},
    prelude::*,
    subclass::prelude::*,
};

mod imp {
    use super::*;

    #[derive(Properties, Default, Debug)]
    #[properties(wrapper_type = super::Process)]
    pub struct Process {
        #[property(get, construct_only)]
        pub description: OnceCell<String>,
        #[property(get, set, nullable)]
        pub message: RefCell<Option<String>>,
        #[property(get, set)]
        pub progress: Cell<f64>,
        #[property(get, set)]
        pub finished: Cell<bool>,
        #[property(get, set)]
        pub error: RefCell<Option<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Process {
        const NAME: &'static str = "MusicusProcess";
        type Type = super::Process;
    }

    #[glib::derived_properties]
    impl ObjectImpl for Process {}
}

glib::wrapper! {
    pub struct Process(ObjectSubclass<imp::Process>);
}

impl Process {
    pub fn new(description: &str, receiver: async_channel::Receiver<ProcessMsg>) -> Self {
        let obj: Self = glib::Object::builder()
            .property("description", description)
            .build();

        let obj_clone = obj.clone();
        glib::spawn_future_local(async move {
            while let Ok(msg) = receiver.recv().await {
                match msg {
                    ProcessMsg::Message(message) => {
                        obj_clone.set_message(Some(message));
                    }
                    ProcessMsg::Progress(fraction) => {
                        obj_clone.set_progress(fraction);
                    }
                    ProcessMsg::Result(result) => {
                        obj_clone.set_message(None::<String>);

                        if let Err(err) = result {
                            log::error!("Process \"{}\" failed: {err:?}", obj_clone.description());
                            obj_clone.set_error(err.to_string());
                        }

                        obj_clone.set_finished(true);
                    }
                }
            }
        });

        obj
    }
}

#[derive(Debug)]
pub enum ProcessMsg {
    Message(String),
    Progress(f64),
    Result(Result<()>),
}
