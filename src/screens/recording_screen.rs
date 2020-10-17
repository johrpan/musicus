use crate::backend::*;
use crate::database::*;
use crate::widgets::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use libhandy::HeaderBarExt;
use std::cell::RefCell;
use std::rc::Rc;

pub struct RecordingScreen {
    widget: gtk::Box,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl RecordingScreen {
    pub fn new(backend: Rc<Backend>, recording: RecordingDescription) -> Rc<Self> {
        let builder =
            gtk::Builder::from_resource("/de/johrpan/musicus_editor/ui/recording_screen.ui");

        get_widget!(builder, gtk::Box, widget);
        get_widget!(builder, libhandy::HeaderBar, header);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::MenuButton, menu_button);

        header.set_title(Some(&recording.work.get_title()));
        header.set_subtitle(Some(&recording.get_performers()));

        let result = Rc::new(Self {
            widget,
            navigator: RefCell::new(None),
        });

        back_button.connect_clicked(clone!(@strong result => move |_| {
            let navigator = result.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                navigator.clone().pop();
            }
        }));

        result
    }
}

impl NavigatorScreen for RecordingScreen {
    fn attach_navigator(&self, navigator: Rc<Navigator>) {
        self.navigator.replace(Some(navigator));
    }

    fn get_widget(&self) -> gtk::Widget {
        self.widget.clone().upcast()
    }

    fn detach_navigator(&self) {
        self.navigator.replace(None);
    }
}
