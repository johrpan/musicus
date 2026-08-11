//! One tag assignment within the work or recording editor.
//!
//! Shared by both editors because a tag assignment looks and behaves the same
//! on either: a reorderable row naming the tag, plus an entry for its value
//! when the tag takes one.

use std::cell::OnceCell;

use adw::{prelude::*, subclass::prelude::*};
use gtk::{
    gdk,
    glib::{self, clone, subclass::Signal},
};
use once_cell::sync::Lazy;

use musicus_library::db::models::TagValue;

use crate::util::drag_widget::DragWidget;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/editor/tag_row.blp")]
    pub struct TagRow {
        pub tag_value: OnceCell<TagValue>,

        #[template_child]
        pub value_entry: TemplateChild<gtk::Entry>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TagRow {
        const NAME: &'static str = "MusicusEditorTagRow";
        type Type = super::TagRow;
        type ParentType = adw::ActionRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for TagRow {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("remove").build(),
                    Signal::builder("move")
                        .param_types([super::TagRow::static_type()])
                        .build(),
                ]
            });

            SIGNALS.as_ref()
        }

        fn constructed(&self) {
            self.parent_constructed();

            let drag_source = gtk::DragSource::builder()
                .actions(gdk::DragAction::MOVE)
                .content(&gdk::ContentProvider::for_value(&self.obj().to_value()))
                .build();

            drag_source.connect_drag_begin(clone!(
                #[weak(rename_to = obj)]
                self.obj(),
                move |_, drag| {
                    let icon = gtk::DragIcon::for_drag(drag);
                    icon.set_child(Some(&DragWidget::new(&obj)));
                }
            ));

            self.obj().add_controller(drag_source);

            let drop_target = gtk::DropTarget::builder()
                .actions(gdk::DragAction::MOVE)
                .build();
            drop_target.set_types(&[Self::Type::static_type()]);

            drop_target.connect_drop(clone!(
                #[weak(rename_to = obj)]
                self.obj(),
                #[upgrade_or]
                false,
                move |_, value, _, _| {
                    if let Ok(row) = value.get::<Self::Type>() {
                        obj.emit_by_name::<()>("move", &[&row]);
                        true
                    } else {
                        false
                    }
                }
            ));

            self.obj().add_controller(drop_target);
        }
    }

    impl WidgetImpl for TagRow {}
    impl ListBoxRowImpl for TagRow {}
    impl PreferencesRowImpl for TagRow {}
    impl ActionRowImpl for TagRow {}
}

glib::wrapper! {
    pub struct TagRow(ObjectSubclass<imp::TagRow>)
        @extends adw::ActionRow, adw::PreferencesRow, gtk::ListBoxRow, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Actionable;
}

#[gtk::template_callbacks]
impl TagRow {
    pub fn new(tag_value: TagValue) -> Self {
        let obj: Self = glib::Object::new();
        let imp = obj.imp();

        obj.set_title(tag_value.tag.name.get());

        if tag_value.tag.takes_value {
            imp.value_entry.set_visible(true);

            if let Some(value) = &tag_value.value {
                imp.value_entry.set_text(value);
            }
        }

        imp.tag_value.set(tag_value).unwrap();

        obj
    }

    pub fn connect_move<F: Fn(&Self, Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("move", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let source = values[1].get::<Self>().unwrap();
            f(&obj, source);
            None
        })
    }

    pub fn connect_remove<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("remove", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            f(&obj);
            None
        })
    }

    /// The assignment as edited, with an empty value read as no value.
    pub fn tag_value(&self) -> TagValue {
        let imp = self.imp();
        let tag_value = imp.tag_value.get().unwrap();

        let value = if tag_value.tag.takes_value {
            let text = imp.value_entry.text().trim().to_string();
            (!text.is_empty()).then_some(text)
        } else {
            None
        };

        TagValue {
            tag: tag_value.tag.clone(),
            value,
        }
    }

    #[template_callback]
    fn remove(&self) {
        self.emit_by_name::<()>("remove", &[]);
    }
}
