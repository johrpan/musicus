use crate::backend::Backend;
use crate::ripper::Ripper;
use crate::widgets::{List, Navigator, NavigatorScreen};
use anyhow::Result;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use std::cell::RefCell;
use std::rc::Rc;

/// The current status of a ripped track.
#[derive(Debug, Clone)]
enum RipStatus {
    None,
    Ripping,
    Ready,
    Error,
}

/// Representation of a track on the ripped disc.
#[derive(Debug, Clone)]
struct RipTrack {
    pub status: RipStatus,
    pub index: u32,
    pub title: String,
    pub subtitle: String,
}

/// A dialog for importing tracks from a CD.
pub struct ImportDiscDialog {
    backend: Rc<Backend>,
    widget: gtk::Box,
    stack: gtk::Stack,
    info_bar: gtk::InfoBar,
    list: Rc<List<RipTrack>>,
    ripper: Ripper,
    tracks: RefCell<Vec<RipTrack>>,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl ImportDiscDialog {
    /// Create a new import disc dialog.
    pub fn new(backend: Rc<Backend>) -> Rc<Self> {
        // Create UI

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/import_disc_dialog.ui");

        get_widget!(builder, gtk::Box, widget);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::Stack, stack);
        get_widget!(builder, gtk::InfoBar, info_bar);
        get_widget!(builder, gtk::Button, import_button);
        get_widget!(builder, gtk::Frame, frame);

        let list = List::<RipTrack>::new("No tracks found.");
        frame.add(&list.widget);

        let mut tmp_dir = glib::get_tmp_dir().unwrap();
        let dir_name = format!("musicus-{}", rand::random::<u64>());
        tmp_dir.push(dir_name);

        std::fs::create_dir(&tmp_dir).unwrap();

        let ripper = Ripper::new(tmp_dir.to_str().unwrap());

        let this = Rc::new(Self {
            backend,
            widget,
            stack,
            info_bar,
            list,
            ripper,
            tracks: RefCell::new(Vec::new()),
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
                match clone.ripper.load_disc().await {
                    Ok(disc) => {
                        let mut tracks = Vec::<RipTrack>::new();
                        for track in disc.first_track..=disc.last_track {
                            tracks.push(RipTrack {
                                status: RipStatus::None,
                                index: track,
                                title: "Track".to_string(),
                                subtitle: "Unknown".to_string(),
                            });
                        }

                        clone.tracks.replace(tracks.clone());
                        clone.list.show_items(tracks);
                        clone.stack.set_visible_child_name("content");

                        clone.rip().await.unwrap();
                    }
                    Err(_) => {
                        clone.info_bar.set_revealed(true);
                        clone.stack.set_visible_child_name("start");
                    }
                }
            });
        }));

        this.list.set_make_widget(|track| {
            let title = gtk::Label::new(Some(&format!("{}. {}", track.index, track.title)));
            title.set_ellipsize(pango::EllipsizeMode::End);
            title.set_halign(gtk::Align::Start);

            let subtitle = gtk::Label::new(Some(&track.subtitle));
            subtitle.set_ellipsize(pango::EllipsizeMode::End);
            subtitle.set_opacity(0.5);
            subtitle.set_halign(gtk::Align::Start);

            let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
            vbox.add(&title);
            vbox.add(&subtitle);
            vbox.set_hexpand(true);

            use RipStatus::*;

            let status: gtk::Widget = match track.status {
                None => {
                    let placeholder = gtk::Label::new(Option::None);
                    placeholder.set_property_width_request(16);
                    placeholder.upcast()
                }
                Ripping => {
                    let spinner = gtk::Spinner::new();
                    spinner.start();
                    spinner.upcast()
                }
                Ready => gtk::Image::from_icon_name(
                    Some("object-select-symbolic"),
                    gtk::IconSize::Button,
                )
                .upcast(),
                Error => {
                    gtk::Image::from_icon_name(Some("dialog-error-symbolic"), gtk::IconSize::Dialog)
                        .upcast()
                }
            };

            let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            hbox.set_border_width(6);
            hbox.add(&vbox);
            hbox.add(&status);

            hbox.upcast()
        });

        this
    }

    /// Rip the disc in the background.
    async fn rip(&self) -> Result<()> {
        let mut current_track = 0;

        while current_track < self.tracks.borrow().len() {
            {
                let mut tracks = self.tracks.borrow_mut();
                let mut track = &mut tracks[current_track];
                track.status = RipStatus::Ripping;
                self.list.show_items(tracks.clone());
            }

            self.ripper
                .rip_track(self.tracks.borrow()[current_track].index)
                .await
                .unwrap();

            {
                let mut tracks = self.tracks.borrow_mut();
                let mut track = &mut tracks[current_track];
                track.status = RipStatus::Ready;
                self.list.show_items(tracks.clone());
            }

            current_track += 1;
        }

        Ok(())
    }
}

impl NavigatorScreen for ImportDiscDialog {
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
