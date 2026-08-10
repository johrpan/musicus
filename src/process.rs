use std::cell::{Cell, OnceCell, RefCell};

use gtk::{
    glib::{self, Properties},
    prelude::*,
    subclass::prelude::*,
};
use musicus_library::process::Cancellation;
pub use musicus_library::process::{ProcessHandle, ProcessMsg};

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
        /// Whether the process stopped because the user cancelled it.
        #[property(get, set)]
        pub cancelled: Cell<bool>,
        pub cancellation: OnceCell<Cancellation>,
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
    pub fn new(description: &str, handle: ProcessHandle) -> Self {
        let obj: Self = glib::Object::builder()
            .property("description", description)
            .build();

        let ProcessHandle {
            receiver,
            cancellation,
        } = handle;

        let _ = obj.imp().cancellation.set(cancellation);

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
                    ProcessMsg::Cancelled => {
                        log::info!("Process \"{}\" was cancelled", obj_clone.description());
                        obj_clone.set_message(None::<String>);
                        obj_clone.set_cancelled(true);
                        obj_clone.set_finished(true);
                    }
                }
            }
        });

        obj
    }

    /// Whether this process can still be cancelled.
    pub fn is_cancellable(&self) -> bool {
        !self.finished() && !self.cancellation_requested()
    }

    pub fn cancellation_requested(&self) -> bool {
        self.imp()
            .cancellation
            .get()
            .is_some_and(|cancellation| cancellation.is_cancelled())
    }

    /// Ask the background operation to stop. It stops at its next cancellation
    /// point, so `finished` may only become true some time afterwards.
    pub fn cancel(&self) {
        if let Some(cancellation) = self.imp().cancellation.get() {
            cancellation.cancel();
        }
    }
}
