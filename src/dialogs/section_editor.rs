use crate::database::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use std::rc::Rc;

pub struct SectionEditor {
    window: gtk::Window,
    title_entry: gtk::Entry,
}

impl SectionEditor {
    pub fn new<F: Fn(WorkSectionDescription) -> () + 'static, P: IsA<gtk::Window>>(
        parent: &P,
        section: Option<WorkSectionDescription>,
        callback: F,
    ) -> Rc<Self> {
        let builder =
            gtk::Builder::from_resource("/de/johrpan/musicus_editor/ui/section_editor.ui");

        get_widget!(builder, gtk::Window, window);
        get_widget!(builder, gtk::Button, cancel_button);
        get_widget!(builder, gtk::Button, save_button);
        get_widget!(builder, gtk::Entry, title_entry);

        match section {
            Some(section) => {
                title_entry.set_text(&section.title);
            }
            None => (),
        }

        let result = Rc::new(SectionEditor {
            window: window,
            title_entry: title_entry,
        });

        cancel_button.connect_clicked(clone!(@strong result => move |_| {
            result.window.close();
        }));

        save_button.connect_clicked(clone!(@strong result => move |_| {
            result.window.close();

            let section = WorkSectionDescription {
                before_index: 0,
                title: result.title_entry.get_text().to_string(),
            };

            callback(section);
        }));

        result.window.set_transient_for(Some(parent));

        result
    }

    pub fn show(&self) {
        self.window.show();
    }
}
