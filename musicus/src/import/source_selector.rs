use super::medium_editor::MediumEditor;
use super::disc_source::DiscSource;
use super::folder_source::FolderSource;
use super::source::Source;
use crate::navigator::{NavigationHandle, Screen};
use crate::widgets::Widget;
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use std::path::PathBuf;
use std::rc::Rc;

/// A dialog for starting to import music.
pub struct SourceSelector {
    handle: NavigationHandle<()>,
    widget: gtk::Stack,
    status_page: libadwaita::StatusPage,
}

impl Screen<(), ()> for SourceSelector {
    /// Create a new source selector.
    fn new(_: (), handle: NavigationHandle<()>) -> Rc<Self> {
        // Create UI

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/source_selector.ui");

        get_widget!(builder, gtk::Stack, widget);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::Button, folder_button);
        get_widget!(builder, gtk::Button, disc_button);
        get_widget!(builder, libadwaita::StatusPage, status_page);
        get_widget!(builder, gtk::Button, try_again_button);

        let this = Rc::new(Self {
            handle,
            widget,
            status_page,
        });

        // Connect signals and callbacks

        back_button.connect_clicked(clone!(@weak this => move |_| {
            this.handle.pop(None);
        }));

        folder_button.connect_clicked(clone!(@weak this => move |_| {
            let dialog = gtk::FileChooserDialog::new(
                Some(&gettext("Select folder")),
                Some(&this.handle.window),
                gtk::FileChooserAction::SelectFolder,
                &[
                    (&gettext("Cancel"), gtk::ResponseType::Cancel),
                    (&gettext("Select"), gtk::ResponseType::Accept),
                ]);

            dialog.set_modal(true);

            dialog.connect_response(clone!(@weak this => move |dialog, response| {
                dialog.hide();

                if let gtk::ResponseType::Accept = response {
                    if let Some(file) = dialog.get_file() {
                        if let Some(path) = file.get_path() {
                            this.widget.set_visible_child_name("loading");

                            spawn!(@clone this, async move {
                                let folder = FolderSource::new(PathBuf::from(path));
                                match folder.load().await {
                                    Ok(_) => {
                                        let source = Rc::new(Box::new(folder) as Box<dyn Source>);
                                        push!(this.handle, MediumEditor, source).await;
                                        this.handle.pop(Some(()));
                                    }
                                    Err(err) => {
                                        this.status_page.set_description(Some(&err.to_string()));
                                        this.widget.set_visible_child_name("error");
                                    }
                                }
                            });
                        }
                    }
                }
            }));

            dialog.show();
        }));

        disc_button.connect_clicked(clone!(@weak this => move |_| {
            this.widget.set_visible_child_name("loading");

            spawn!(@clone this, async move {
                let disc = DiscSource::new().unwrap();
                match disc.load().await {
                    Ok(_) => {
                        let source = Rc::new(Box::new(disc) as Box<dyn Source>);
                        push!(this.handle, MediumEditor, source).await;
                        this.handle.pop(Some(()));
                    }
                    Err(err) => {
                        this.status_page.set_description(Some(&err.to_string()));
                        this.widget.set_visible_child_name("error");
                    }
                }
            });
        }));

        try_again_button.connect_clicked(clone!(@weak this => move |_| {
            this.widget.set_visible_child_name("content");
        }));

        this
    }
}

impl Widget for SourceSelector {
    fn get_widget(&self) -> gtk::Widget {
        self.widget.clone().upcast()
    }
}
