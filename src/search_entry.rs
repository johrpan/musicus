use crate::search_tag::MusicusSearchTag;
use adw::{gdk, glib, glib::clone, glib::subclass::Signal, prelude::*, subclass::prelude::*};
use once_cell::sync::Lazy;
use std::cell::RefCell;

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
                            true
                        }
                        None => false,
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
            static SIGNALS: Lazy<Vec<Signal>> =
                Lazy::new(|| vec![Signal::builder("activate").build()]);

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
        let mut tags = self.imp().tags.borrow_mut();

        while let Some(tag) = tags.pop() {
            self.imp().tags_box.remove(&tag);
        }

        self.imp().text.set_text("");
    }

    pub fn add_tag(&self, name: &str) {
        self.imp().text.set_text("");
        let tag = MusicusSearchTag::new(name);
        self.imp().tags_box.append(&tag);
        self.imp().tags.borrow_mut().push(tag);
    }

    #[template_callback]
    fn activate(&self, _: &gtk::Text) {
        self.emit_by_name::<()>("activate", &[]);
    }

    #[template_callback]
    fn backspace(&self, text: &gtk::Text) {
        if text.cursor_position() == 0 {
            if let Some(tag) = self.imp().tags.borrow_mut().pop() {
                self.imp().tags_box.remove(&tag);
            }
        }
    }
}
