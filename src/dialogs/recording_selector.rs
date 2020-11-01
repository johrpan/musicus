use super::*;
use crate::backend::Backend;
use crate::database::*;
use crate::widgets::*;
use gettextrs::gettext;
use gio::prelude::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use libhandy::prelude::*;
use libhandy::HeaderBarExt;
use std::cell::RefCell;
use std::rc::Rc;

pub struct RecordingSelector {
    backend: Rc<Backend>,
    window: gtk::Window,
    callback: Box<dyn Fn(RecordingDescription) -> () + 'static>,
    leaflet: libhandy::Leaflet,
    navigator: Rc<Navigator>,
}

impl RecordingSelector {
    pub fn new<P, F>(backend: Rc<Backend>, parent: &P, callback: F) -> Rc<Self>
    where
        P: IsA<gtk::Window>,
        F: Fn(RecordingDescription) -> () + 'static,
    {
        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/recording_selector.ui");

        get_widget!(builder, gtk::Window, window);
        get_widget!(builder, libhandy::Leaflet, leaflet);
        get_widget!(builder, gtk::Button, add_button);
        get_widget!(builder, gtk::Box, sidebar_box);
        get_widget!(builder, gtk::Box, empty_screen);

        let person_list = PersonList::new(backend.clone());
        sidebar_box.pack_start(&person_list.widget, true, true, 0);

        let navigator = Navigator::new(&empty_screen);
        leaflet.add(&navigator.widget);

        let result = Rc::new(Self {
            backend: backend,
            window: window,
            callback: Box::new(callback),
            leaflet: leaflet,
            navigator: navigator,
        });

        add_button.connect_clicked(clone!(@strong result => move |_| {
            let editor = RecordingEditor::new(
                result.backend.clone(),
                &result.window,
                None,
                clone!(@strong result => move |recording| {
                    result.select(recording);
                }),
            );

            editor.show();
        }));

        person_list.set_selected(clone!(@strong result => move |person| {
            result.navigator.clone().replace(RecordingSelectorPersonScreen::new(result.backend.clone(), result.clone(), person.clone()));
            result.leaflet.set_visible_child(&result.navigator.widget);
        }));

        result.window.set_transient_for(Some(parent));

        result
    }

    pub fn show(&self) {
        self.window.show();
    }

    pub fn select(&self, recording: RecordingDescription) {
        self.window.close();
        (self.callback)(recording);
    }
}

struct RecordingSelectorPersonScreen {
    backend: Rc<Backend>,
    selector: Rc<RecordingSelector>,
    widget: gtk::Box,
    stack: gtk::Stack,
    work_list: Rc<List<WorkDescription>>,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl RecordingSelectorPersonScreen {
    pub fn new(backend: Rc<Backend>, selector: Rc<RecordingSelector>, person: Person) -> Rc<Self> {
        let builder =
            gtk::Builder::from_resource("/de/johrpan/musicus/ui/recording_selector_screen.ui");

        get_widget!(builder, gtk::Box, widget);
        get_widget!(builder, libhandy::HeaderBar, header);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::Stack, stack);

        header.set_title(Some(&person.name_fl()));

        let work_list = List::new(
            |work: &WorkDescription| {
                let label = gtk::Label::new(Some(&work.title));
                label.set_halign(gtk::Align::Start);
                label.upcast()
            },
            |_| true,
            &gettext("No works found."),
        );

        stack.add_named(&work_list.widget, "content");

        let result = Rc::new(Self {
            backend,
            selector,
            widget,
            stack,
            work_list,
            navigator: RefCell::new(None),
        });

        back_button.connect_clicked(clone!(@strong result => move |_| {
            let navigator = result.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                navigator.clone().pop();
            }
        }));

        result
            .work_list
            .set_selected(clone!(@strong result => move |work| {
                let navigator = result.navigator.borrow().clone();
                if let Some(navigator) = navigator {
                    navigator.push(RecordingSelectorWorkScreen::new(result.backend.clone(), result.selector.clone(), work.clone()));
                }
            }));

        let context = glib::MainContext::default();
        let clone = result.clone();
        context.spawn_local(async move {
            let works = clone
                .backend
                .get_work_descriptions(person.id)
                .await
                .unwrap();

            clone.work_list.show_items(works);
            clone.stack.set_visible_child_name("content");
        });

        result
    }
}

impl NavigatorScreen for RecordingSelectorPersonScreen {
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

struct RecordingSelectorWorkScreen {
    backend: Rc<Backend>,
    selector: Rc<RecordingSelector>,
    widget: gtk::Box,
    stack: gtk::Stack,
    recording_list: Rc<List<RecordingDescription>>,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl RecordingSelectorWorkScreen {
    pub fn new(
        backend: Rc<Backend>,
        selector: Rc<RecordingSelector>,
        work: WorkDescription,
    ) -> Rc<Self> {
        let builder =
            gtk::Builder::from_resource("/de/johrpan/musicus/ui/recording_selector_screen.ui");

        get_widget!(builder, gtk::Box, widget);
        get_widget!(builder, libhandy::HeaderBar, header);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::Stack, stack);

        header.set_title(Some(&work.title));
        header.set_subtitle(Some(&work.composer.name_fl()));

        let recording_list = List::new(
            |recording: &RecordingDescription| {
                let work_label = gtk::Label::new(Some(&recording.work.get_title()));

                work_label.set_ellipsize(pango::EllipsizeMode::End);
                work_label.set_halign(gtk::Align::Start);

                let performers_label = gtk::Label::new(Some(&recording.get_performers()));
                performers_label.set_ellipsize(pango::EllipsizeMode::End);
                performers_label.set_opacity(0.5);
                performers_label.set_halign(gtk::Align::Start);

                let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
                vbox.add(&work_label);
                vbox.add(&performers_label);

                vbox.upcast()
            },
            |_| true,
            &gettext("No recordings found."),
        );

        stack.add_named(&recording_list.widget, "content");

        let result = Rc::new(Self {
            backend,
            selector,
            widget,
            stack,
            recording_list,
            navigator: RefCell::new(None),
        });

        back_button.connect_clicked(clone!(@strong result => move |_| {
            let navigator = result.navigator.borrow().clone();
            if let Some(navigator) = navigator {
                navigator.clone().pop();
            }
        }));

        result
            .recording_list
            .set_selected(clone!(@strong result => move |recording| {
                result.selector.select(recording.clone());
            }));

        let context = glib::MainContext::default();
        let clone = result.clone();
        context.spawn_local(async move {
            let recordings = clone
                .backend
                .get_recordings_for_work(work.id)
                .await
                .unwrap();

            clone.recording_list.show_items(recordings);
            clone.stack.set_visible_child_name("content");
        });

        result
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
