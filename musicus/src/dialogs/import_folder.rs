use super::ImportDialog;
use crate::backend::Backend;
use crate::editors::TrackSource;
use crate::widgets::{Navigator, NavigatorScreen};
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use std::cell::RefCell;
use std::rc::Rc;
use std::path::Path;

/// The initial screen for importing a folder.
pub struct ImportFolderDialog {
    backend: Rc<Backend>,
    widget: gtk::Box,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl ImportFolderDialog {
    /// Create a new import folderdialog.
    pub fn new(backend: Rc<Backend>) -> Rc<Self> {
        // Create UI

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/import_folder_dialog.ui");

        get_widget!(builder, gtk::Box, widget);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::Button, import_button);

        let this = Rc::new(Self {
            backend,
            widget,
            navigator: RefCell::new(None),
        });

        // Connect signals and callbacks

        back_button.connect_clicked(clone!(@strong this => move |_| {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                navigator.pop();
            }
        }));

        import_button.connect_clicked(clone!(@strong this => move |_| {            
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                let chooser = gtk::FileChooserNative::new(
                    Some("Select folder"),
                    Some(&navigator.window),
                    gtk::FileChooserAction::SelectFolder,
                    None,
                    None,
                );
                
                chooser.connect_response(clone!(@strong this => move |chooser, response| {
                    if response == gtk::ResponseType::Accept {
                        let navigator = this.navigator.borrow().clone();
                        if let Some(navigator) = navigator {
                            let path = chooser.get_filename().unwrap();
                            let source = TrackSource::folder(&path).unwrap();
                            let dialog = ImportDialog::new(this.backend.clone(), Rc::new(source));
                            navigator.push(dialog);
                        }
                    }
                }));
                
                chooser.run();  
            }
        }));

        this
    }
}

impl NavigatorScreen for ImportFolderDialog {
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
