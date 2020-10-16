use crate::backend::*;
use crate::database::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use libhandy::HeaderBarExt;
use std::cell::RefCell;
use std::rc::Rc;

pub struct RecordingScreen {
    pub widget: gtk::Box,
    back: RefCell<Option<Box<dyn Fn() -> () + 'static>>>,
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
            back: RefCell::new(None),
        });

        back_button.connect_clicked(clone!(@strong result => move |_| {
            if let Some(back) = &*result.back.borrow() {
                back();
            }
        }));

        result
    }

    pub fn set_back<B>(&self, back: B)
    where
        B: Fn() -> () + 'static,
    {
        self.back.replace(Some(Box::new(back)));
    }
}
