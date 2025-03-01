use std::cell::OnceCell;

use adw::{prelude::*, subclass::prelude::*};
use gtk::{
    gdk,
    glib::{self, clone, subclass::Signal},
};
use once_cell::sync::Lazy;

use crate::{db::models::Recording, util::drag_widget::DragWidget};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/editor/album/recording_row.blp")]
    pub struct RecordingRow {
        pub recording: OnceCell<Recording>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RecordingRow {
        const NAME: &'static str = "MusicusAlbumEditorRecordingRow";
        type Type = super::RecordingRow;
        type ParentType = adw::ActionRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for RecordingRow {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("remove").build(),
                    Signal::builder("move")
                        .param_types([super::RecordingRow::static_type()])
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

    impl WidgetImpl for RecordingRow {}
    impl ListBoxRowImpl for RecordingRow {}
    impl PreferencesRowImpl for RecordingRow {}
    impl ActionRowImpl for RecordingRow {}
}

glib::wrapper! {
    pub struct RecordingRow(ObjectSubclass<imp::RecordingRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::ActionRow;
}

#[gtk::template_callbacks]
impl RecordingRow {
    pub fn new(recording: Recording) -> Self {
        let obj: Self = glib::Object::new();
        obj.set_title(&recording.work.to_string());
        obj.set_subtitle(&recording.performers_string());
        obj.imp().recording.set(recording).unwrap();
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

    pub fn recording(&self) -> Recording {
        self.imp().recording.get().unwrap().clone()
    }

    #[template_callback]
    fn remove(&self) {
        self.emit_by_name::<()>("remove", &[]);
    }
}
