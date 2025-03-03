use std::cell::RefCell;

use gtk::{
    glib::{self},
    subclass::prelude::*,
};

use crate::process::Process;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct ProcessManager {
        pub processes: RefCell<Vec<Process>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProcessManager {
        const NAME: &'static str = "MusicusProcessManager";
        type Type = super::ProcessManager;
    }

    impl ObjectImpl for ProcessManager {}
}

glib::wrapper! {
    pub struct ProcessManager(ObjectSubclass<imp::ProcessManager>);
}

impl ProcessManager {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn add_process(&self, process: &Process) {
        self.imp().processes.borrow_mut().push(process.to_owned());
    }

    pub fn processes(&self) -> Vec<Process> {
        self.imp().processes.borrow().clone()
    }

    pub fn any_ongoing(&self) -> bool {
        self.imp().processes.borrow().iter().any(|p| !p.finished())
    }

    pub fn remove_process(&self, process: &Process) {
        self.imp().processes.borrow_mut().retain(|p| p != process);
    }
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}
