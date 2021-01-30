use super::medium_editor::MediumEditor;
use super::disc_source::DiscSource;
use super::folder_source::FolderSource;
use super::source::Source;
use crate::backend::Backend;
use crate::widgets::{Navigator, NavigatorScreen};
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

/// A dialog for starting to import music.
pub struct SourceSelector {
    backend: Rc<Backend>,
    widget: gtk::Box,
    stack: gtk::Stack,
    info_bar: gtk::InfoBar,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl SourceSelector {
    /// Create a new source selector.
    pub fn new(backend: Rc<Backend>) -> Rc<Self> {
        // Create UI

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/source_selector.ui");

        get_widget!(builder, gtk::Box, widget);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::Stack, stack);
        get_widget!(builder, gtk::InfoBar, info_bar);
        get_widget!(builder, gtk::Button, folder_button);
        get_widget!(builder, gtk::Button, disc_button);

        let this = Rc::new(Self {
            backend,
            widget,
            stack,
            info_bar,
            navigator: RefCell::new(None),
        });

        // Connect signals and callbacks

        back_button.connect_clicked(clone!(@strong this => move |_| {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                navigator.pop();
            }
        }));

        folder_button.connect_clicked(clone!(@strong this => move |_| {
            let window = this.navigator.borrow().clone().unwrap().window.clone();
            let dialog = gtk::FileChooserDialog::new(
                Some(&gettext("Select folder")),
                Some(&window),
                gtk::FileChooserAction::SelectFolder,
                &[
                    (&gettext("Cancel"), gtk::ResponseType::Cancel),
                    (&gettext("Select"), gtk::ResponseType::Accept),
                ]);

            dialog.connect_response(clone!(@strong this => move |dialog, response| {
                this.stack.set_visible_child_name("loading");
                dialog.hide();

                if let gtk::ResponseType::Accept = response {
                    if let Some(file) = dialog.get_file() {
                        if let Some(path) = file.get_path() {
                            let context = glib::MainContext::default();
                            let clone = this.clone();
                            context.spawn_local(async move {
                                let folder = FolderSource::new(PathBuf::from(path));
                                match folder.load().await {
                                    Ok(_) => {
                                        let navigator = clone.navigator.borrow().clone();
                                        if let Some(navigator) = navigator {
                                            let source = Rc::new(Box::new(folder) as Box<dyn Source>);
                                            let editor = MediumEditor::new(clone.backend.clone(), source);
                                            navigator.push(editor);
                                        }

                                        clone.info_bar.set_revealed(false);
                                        clone.stack.set_visible_child_name("start");
                                    }
                                    Err(_) => {
                                        // TODO: Present error.
                                        clone.info_bar.set_revealed(true);
                                        clone.stack.set_visible_child_name("start");
                                    }
                                }
                            });
                        }
                    }
                }
            }));

            dialog.show();
        }));

        disc_button.connect_clicked(clone!(@strong this => move |_| {
            this.stack.set_visible_child_name("loading");

            let context = glib::MainContext::default();
            let clone = this.clone();
            context.spawn_local(async move {
                let disc = DiscSource::new().unwrap();
                match disc.load().await {
                    Ok(_) => {
                        let navigator = clone.navigator.borrow().clone();
                        if let Some(navigator) = navigator {
                            let source = Rc::new(Box::new(disc) as Box<dyn Source>);
                            let editor = MediumEditor::new(clone.backend.clone(), source);
                            navigator.push(editor);
                        }

                        clone.info_bar.set_revealed(false);
                        clone.stack.set_visible_child_name("start");
                    }
                    Err(_) => {
                        // TODO: Present error.
                        clone.info_bar.set_revealed(true);
                        clone.stack.set_visible_child_name("start");
                    }
                }
            });
        }));

        this
    }
}

impl NavigatorScreen for SourceSelector {
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
