use crate::backend::*;
use crate::database::*;
use crate::widgets::*;
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use libhandy::HeaderBarExt;
use std::cell::RefCell;
use std::rc::Rc;

/// A screen within the recording selector presenting a list of recordings for a work.
pub struct RecordingSelectorWorkScreen {
    backend: Rc<Backend>,
    work: Work,
    online: bool,
    widget: gtk::Box,
    stack: gtk::Stack,
    recording_list: Rc<List<Recording>>,
    selected_cb: RefCell<Option<Box<dyn Fn(Recording) -> ()>>>,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl RecordingSelectorWorkScreen {
    /// Create a new recording selector work screen.
    pub fn new(backend: Rc<Backend>, work: Work, online: bool) -> Rc<Self> {
        // Create UI

        let builder =
            gtk::Builder::from_resource("/de/johrpan/musicus/ui/recording_selector_screen.ui");

        get_widget!(builder, gtk::Box, widget);
        get_widget!(builder, libhandy::HeaderBar, header);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::Stack, stack);
        get_widget!(builder, gtk::ScrolledWindow, scroll);
        get_widget!(builder, gtk::Button, try_again_button);

        header.set_title(Some(&work.title));
        header.set_subtitle(Some(&work.composer.name_fl()));

        let recording_list = List::new(&gettext("No recordings found."));
        scroll.add(&recording_list.widget);

        let this = Rc::new(Self {
            backend,
            work,
            online,
            widget,
            stack,
            recording_list,
            selected_cb: RefCell::new(None),
            navigator: RefCell::new(None),
        });

        // Connect signals and callbacks

        back_button.connect_clicked(clone!(@strong this => move |_| {
            let navigator = this.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                navigator.pop();
            }
        }));

        let load_online = Rc::new(clone!(@strong this => move || {
            this.stack.set_visible_child_name("loading");

            let context = glib::MainContext::default();
            let clone = this.clone();
            context.spawn_local(async move {
                match clone.backend.get_recordings_for_work(&clone.work.id).await {
                    Ok(recordings) => {
                        clone.recording_list.show_items(recordings);
                        clone.stack.set_visible_child_name("content");
                    }
                    Err(_) => {
                        clone.recording_list.show_items(Vec::new());
                        clone.stack.set_visible_child_name("error");
                    }
                }
            });
        }));

        let load_local = Rc::new(clone!(@strong this => move || {
            this.stack.set_visible_child_name("loading");

            let context = glib::MainContext::default();
            let clone = this.clone();
            context.spawn_local(async move {
                let recordings = clone.backend.db().get_recordings_for_work(&clone.work.id).await.unwrap();
                clone.recording_list.show_items(recordings);
                clone.stack.set_visible_child_name("content");
            });
        }));

        this.recording_list
            .set_make_widget(|recording: &Recording| {
                let work_label = gtk::Label::new(Some(&recording.work.get_title()));
                work_label.set_ellipsize(pango::EllipsizeMode::End);
                work_label.set_halign(gtk::Align::Start);

                let performers_label = gtk::Label::new(Some(&recording.get_performers()));
                performers_label.set_ellipsize(pango::EllipsizeMode::End);
                performers_label.set_opacity(0.5);
                performers_label.set_halign(gtk::Align::Start);

                let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
                vbox.set_border_width(6);
                vbox.add(&work_label);
                vbox.add(&performers_label);

                vbox.upcast()
            });

        this.recording_list
            .set_selected(clone!(@strong this => move |recording| {
                if let Some(cb) = &*this.selected_cb.borrow() {
                    cb(recording.clone());
                }
            }));

        try_again_button.connect_clicked(clone!(@strong load_online => move |_| {
            load_online();
        }));

        // Initialize

        if this.online {
            load_online();
        } else {
            load_local();
        }

        this
    }

    /// Sets a closure to be called when the user has selected a recording.
    pub fn set_selected_cb<F: Fn(Recording) -> () + 'static>(&self, cb: F) {
        self.selected_cb.replace(Some(Box::new(cb)));
    }
}

impl NavigatorScreen for RecordingSelectorWorkScreen {
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
