use super::medium_editor::MediumEditor;
use super::disc_source::DiscSource;
use crate::backend::Backend;
use crate::widgets::{Navigator, NavigatorScreen};
use anyhow::Result;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use std::cell::RefCell;
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
        get_widget!(builder, gtk::Button, import_button);

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

        import_button.connect_clicked(clone!(@strong this => move |_| {
            this.stack.set_visible_child_name("loading");

            let context = glib::MainContext::default();
            let clone = this.clone();
            context.spawn_local(async move {
                match DiscSource::load().await {
                    Ok(disc) => {
                        let navigator = clone.navigator.borrow().clone();
                        if let Some(navigator) = navigator {
                            let editor = MediumEditor::new(clone.backend.clone(), disc);
                            navigator.push(editor);
                        }

                        clone.info_bar.set_revealed(false);
                        clone.stack.set_visible_child_name("start");
                    }
                    Err(_) => {
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
