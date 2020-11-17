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

/// A screen within the work selector that presents a list of works by a person.
pub struct WorkSelectorPersonScreen {
    backend: Rc<Backend>,
    widget: gtk::Box,
    stack: gtk::Stack,
    work_list: Rc<List<Work>>,
    selected_cb: RefCell<Option<Box<dyn Fn(Work) -> ()>>>,
    navigator: RefCell<Option<Rc<Navigator>>>,
}

impl WorkSelectorPersonScreen {
    /// Create a new work selector person screen.
    pub fn new(backend: Rc<Backend>, person: Person) -> Rc<Self> {
        // Create UI

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/work_selector_screen.ui");

        get_widget!(builder, gtk::Box, widget);
        get_widget!(builder, libhandy::HeaderBar, header);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::Stack, stack);

        header.set_title(Some(&person.name_fl()));

        let work_list = List::new(&gettext("No works found."));
        stack.add_named(&work_list.widget, "content");

        let this = Rc::new(Self {
            backend,
            widget,
            stack,
            work_list,
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

        this.work_list.set_make_widget(|work: &Work| {
            let label = gtk::Label::new(Some(&work.title));
            label.set_ellipsize(pango::EllipsizeMode::End);
            label.set_halign(gtk::Align::Start);
            label.set_margin_start(6);
            label.set_margin_end(6);
            label.set_margin_top(6);
            label.set_margin_bottom(6);
            label.upcast()
        });

        this.work_list
            .set_selected(clone!(@strong this => move |work| {
                let navigator = this.navigator.borrow().clone();
                if let Some(navigator) = navigator {
                    if let Some(cb) = &*this.selected_cb.borrow() {
                        cb(work.clone());
                    }
                }
            }));

        // Initialize

        let context = glib::MainContext::default();
        let clone = this.clone();
        context.spawn_local(async move {
            let works = clone.backend.db().get_works(person.id).await.unwrap();

            clone.work_list.show_items(works);
            clone.stack.set_visible_child_name("content");
        });

        this
    }

    /// Sets a closure to be called when the user has selected a work.
    pub fn set_selected_cb<F: Fn(Work) -> () + 'static>(&self, cb: F) {
        self.selected_cb.replace(Some(Box::new(cb)));
    }
}

impl NavigatorScreen for WorkSelectorPersonScreen {
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
