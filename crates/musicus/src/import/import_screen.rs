use super::medium_editor::MediumEditor;
use super::medium_preview::MediumPreview;
use crate::navigator::{NavigationHandle, Screen};
use crate::selectors::MediumSelector;
use crate::widgets::Widget;
use adw::builders::ActionRowBuilder;
use adw::prelude::*;
use glib::clone;
use gtk_macros::get_widget;
use musicus_backend::db::{self, Medium};
use musicus_backend::import::ImportSession;
use std::rc::Rc;
use std::sync::Arc;

/// A dialog for selecting metadata when importing music.
pub struct ImportScreen {
    handle: NavigationHandle<()>,
    session: Arc<ImportSession>,
    widget: gtk::Box,
    matching_stack: gtk::Stack,
    error_row: adw::ActionRow,
    matching_list: gtk::ListBox,
}

impl ImportScreen {
    /// Find matching mediums in the library.
    fn load_matches(self: &Rc<Self>) {
        self.matching_stack.set_visible_child_name("loading");

        let this = self;
        spawn!(@clone this, async move {
            let mediums = db::get_mediums_by_source_id(
                &mut this.handle.backend.db().lock().unwrap(),
                this.session.source_id()
            );

            match mediums {
                Ok(mediums) => {
                    if !mediums.is_empty() {
                        this.show_matches(mediums);
                        this.matching_stack.set_visible_child_name("content");
                    } else {
                        this.matching_stack.set_visible_child_name("empty");
                    }
                }
                Err(err) => {
                    this.error_row.set_subtitle(&err.to_string());
                    this.matching_stack.set_visible_child_name("error");
                }
            }
        });
    }

    /// Populate the list of matches
    fn show_matches(self: &Rc<Self>, mediums: Vec<Medium>) {
        if let Some(mut child) = self.matching_list.first_child() {
            loop {
                let next_child = child.next_sibling();
                self.matching_list.remove(&child);

                match next_child {
                    Some(next_child) => child = next_child,
                    None => break,
                }
            }
        }

        let this = self;

        for medium in mediums {
            let row = ActionRowBuilder::new()
                .activatable(true)
                .title(&medium.name)
                .subtitle(&format!("{} Tracks", medium.tracks.len()))
                .build();

            row.connect_activated(clone!(@weak this =>  move |_| {
                let medium = medium.clone();
                spawn!(@clone this, async move {
                    if let Some(()) = push!(this.handle, MediumPreview, (this.session.clone(), medium.clone())).await {
                        this.handle.pop(Some(()));
                    }
                });
            }));

            this.matching_list.append(&row);
        }
    }

    /// Select a medium from somewhere and present a preview.
    fn select_medium(self: &Rc<Self>, medium: Medium) {
        let this = self;

        spawn!(@clone this, async move {
            if let Some(()) = push!(this.handle, MediumPreview, (this.session.clone(), medium)).await {
                this.handle.pop(Some(()));
            }
        });
    }
}

impl Screen<Arc<ImportSession>, ()> for ImportScreen {
    /// Create a new import screen.
    fn new(session: Arc<ImportSession>, handle: NavigationHandle<()>) -> Rc<Self> {
        // Create UI

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/import_screen.ui");

        get_widget!(builder, gtk::Box, widget);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::Stack, matching_stack);
        get_widget!(builder, gtk::Button, try_again_button);
        get_widget!(builder, adw::ActionRow, error_row);
        get_widget!(builder, gtk::ListBox, matching_list);
        get_widget!(builder, gtk::Button, select_button);
        get_widget!(builder, gtk::Button, add_medium_button);

        let this = Rc::new(Self {
            handle,
            session,
            widget,
            matching_stack,
            error_row,
            matching_list,
        });

        // Connect signals and callbacks

        back_button.connect_clicked(clone!(@weak this =>  move |_| {
            this.handle.pop(None);
        }));

        try_again_button.connect_clicked(clone!(@weak this =>  move |_| {
            this.load_matches();
        }));

        select_button.connect_clicked(clone!(@weak this =>  move |_| {
            spawn!(@clone this, async move {
                if let Some(medium) = push!(this.handle, MediumSelector).await {
                    this.select_medium(medium);
                }
            });
        }));

        add_medium_button.connect_clicked(clone!(@weak this =>  move |_| {
            spawn!(@clone this, async move {
                if let Some(medium) = push!(this.handle, MediumEditor, (Arc::clone(&this.session), None)).await {
                    this.select_medium(medium);
                }
            });
        }));

        // Initialize the view

        this.load_matches();

        // Copy the tracks in the background, if necessary.
        this.session.copy();

        this
    }
}

impl Widget for ImportScreen {
    fn get_widget(&self) -> gtk::Widget {
        self.widget.clone().upcast()
    }
}
