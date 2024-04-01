use crate::{
    library::LibraryQuery,
    search_tag::{MusicusSearchTag, Tag},
};
use adw::{prelude::*, subclass::prelude::*};
use gtk::{
    gdk, gio,
    glib::{self, clone, subclass::Signal, Propagation},
};
use once_cell::sync::Lazy;
use std::{cell::RefCell, time::Duration};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/search_entry.blp")]
    pub struct MusicusSearchEntry {
        #[template_child]
        pub tags_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub text: TemplateChild<gtk::Text>,
        #[template_child]
        pub clear_icon: TemplateChild<gtk::Image>,

        pub tags: RefCell<Vec<MusicusSearchTag>>,
        pub query_changed: RefCell<Option<gio::Cancellable>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MusicusSearchEntry {
        const NAME: &'static str = "MusicusSearchEntry";
        type Type = super::MusicusSearchEntry;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
            klass.set_css_name("entry");

            klass.add_shortcut(
                &gtk::Shortcut::builder()
                    .trigger(&gtk::KeyvalTrigger::new(
                        gdk::Key::Escape,
                        gdk::ModifierType::empty(),
                    ))
                    .action(&gtk::CallbackAction::new(|widget, _| match widget
                        .downcast_ref::<super::MusicusSearchEntry>(
                    ) {
                        Some(obj) => {
                            obj.reset();
                            Propagation::Stop
                        }
                        None => Propagation::Proceed,
                    }))
                    .build(),
            );
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for MusicusSearchEntry {
        fn constructed(&self) {
            let controller = gtk::GestureClick::new();

            controller.connect_pressed(|gesture, _, _, _| {
                gesture.set_state(gtk::EventSequenceState::Claimed);
            });

            controller.connect_released(clone!(@weak self as _self => move |_, _, _, _| {
                _self.obj().reset();
            }));

            self.clear_icon.add_controller(controller);
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("activate").build(),
                    Signal::builder("query-changed").build(),
                ]
            });

            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for MusicusSearchEntry {
        fn grab_focus(&self) -> bool {
            self.text.grab_focus_without_selecting()
        }
    }

    impl BoxImpl for MusicusSearchEntry {}
}

glib::wrapper! {
    pub struct MusicusSearchEntry(ObjectSubclass<imp::MusicusSearchEntry>)
        @extends gtk::Widget;
}

#[gtk::template_callbacks]
impl MusicusSearchEntry {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn connect_query_changed<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("query-changed", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            f(&obj);
            None
        })
    }

    pub fn set_key_capture_widget(&self, widget: &impl IsA<gtk::Widget>) {
        let controller = gtk::EventControllerKey::new();

        controller.connect_key_pressed(clone!(@weak self as _self => @default-return glib::Propagation::Proceed, move |controller, _, _, _| {
            match controller.forward(&_self.imp().text.get()) {
                true => {
                    _self.grab_focus();
                    glib::Propagation::Stop
                },
                false => glib::Propagation::Proceed,
            }
        }));

        controller.connect_key_released(clone!(@weak self as _self => move |controller, _, _, _| {
            controller.forward(&_self.imp().text.get());
        }));

        widget.add_controller(controller);
    }

    pub fn reset(&self) {
        {
            let mut tags = self.imp().tags.borrow_mut();
            while let Some(tag) = tags.pop() {
                self.imp().tags_box.remove(&tag);
            }
        }

        self.imp().text.set_text("");
        self.emit_by_name::<()>("query-changed", &[]);
    }

    pub fn add_tag(&self, tag: Tag) {
        let imp = self.imp();

        imp.clear_icon.set_visible(true);
        imp.text.set_text("");

        let tag = MusicusSearchTag::new(tag);

        tag.connect_remove(clone!(@weak self as self_ => move |tag| {
            let imp = self_.imp();

            imp.tags_box.remove(tag);

            {
                imp.tags.borrow_mut().retain(|t| t.tag() != tag.tag());
            }

            self_.emit_by_name::<()>("query-changed", &[]);
        }));

        imp.tags_box.append(&tag);
        imp.tags.borrow_mut().push(tag);
        self.emit_by_name::<()>("query-changed", &[]);
    }

    pub fn query(&self) -> LibraryQuery {
        let mut query = LibraryQuery {
            search: self.imp().text.text().to_string(),
            ..Default::default()
        };

        for tag in &*self.imp().tags.borrow() {
            match tag.tag().clone() {
                Tag::Composer(person) => query.composer = Some(person),
                Tag::Performer(person) => query.performer = Some(person),
                Tag::Ensemble(ensemble) => query.ensemble = Some(ensemble),
                Tag::Work(work) => query.work = Some(work),
            }
        }

        query
    }

    #[template_callback]
    fn activate(&self, _: &gtk::Text) {
        self.emit_by_name::<()>("activate", &[]);
    }

    #[template_callback]
    fn backspace(&self, text: &gtk::Text) {
        if text.cursor_position() == 0 {
            let changed = if let Some(tag) = self.imp().tags.borrow_mut().pop() {
                self.imp().tags_box.remove(&tag);
                true
            } else {
                false
            };

            if changed {
                self.emit_by_name::<()>("query-changed", &[]);
            }
        }
    }

    #[template_callback]
    async fn text_changed(&self, text: &gtk::Text) {
        let imp = self.imp();

        if imp.tags.borrow().is_empty() {
            imp.clear_icon.set_visible(!text.text().is_empty());
        }

        if let Some(cancellable) = imp.query_changed.borrow_mut().take() {
            cancellable.cancel();
        }

        let cancellable = gio::Cancellable::new();
        imp.query_changed.replace(Some(cancellable.clone()));

        let _ = gio::CancellableFuture::new(
            async {
                glib::timeout_future(Duration::from_millis(150)).await;
                self.emit_by_name::<()>("query-changed", &[]);
            },
            cancellable,
        )
        .await;
    }
}
