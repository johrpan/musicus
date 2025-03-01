use std::cell::{OnceCell, RefCell};

use adw::{prelude::*, subclass::prelude::*};
use gtk::glib::{self, clone, subclass::Signal, Properties};
use once_cell::sync::Lazy;

use crate::{
    db::models::Performer, editor::role::RoleEditor, library::Library,
    selector::performer_role::PerformerRoleSelectorPopover,
};

mod imp {
    use super::*;
    use crate::editor::instrument::InstrumentEditor;

    #[derive(Properties, Debug, Default, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::RecordingEditorPerformerRow)]
    #[template(file = "data/ui/editor/recording/performer_row.blp")]
    pub struct RecordingEditorPerformerRow {
        #[property(get, construct_only)]
        pub navigation: OnceCell<adw::NavigationView>,

        #[property(get, construct_only)]
        pub library: OnceCell<Library>,

        pub performer: RefCell<Option<Performer>>,
        pub role_popover: OnceCell<PerformerRoleSelectorPopover>,

        #[template_child]
        pub role_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub role_box: TemplateChild<gtk::Box>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RecordingEditorPerformerRow {
        const NAME: &'static str = "MusicusRecordingEditorPerformerRow";
        type Type = super::RecordingEditorPerformerRow;
        type ParentType = adw::ActionRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for RecordingEditorPerformerRow {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> =
                Lazy::new(|| vec![Signal::builder("remove").build()]);

            SIGNALS.as_ref()
        }

        fn constructed(&self) {
            self.parent_constructed();

            let role_popover = PerformerRoleSelectorPopover::new(self.library.get().unwrap());

            let obj = self.obj().to_owned();
            role_popover.connect_selected(move |_, role, instrument| {
                if let Some(performer) = &mut *obj.imp().performer.borrow_mut() {
                    let label = match &instrument {
                        Some(instrument) => instrument.to_string(),
                        None => role.to_string(),
                    };

                    obj.imp().role_label.set_label(&label);

                    performer.role = role;
                    performer.instrument = instrument;
                }
            });

            let obj = self.obj().to_owned();
            role_popover.connect_create_role(move |_| {
                let editor = RoleEditor::new(&obj.navigation(), &obj.library(), None);

                editor.connect_created(clone!(
                    #[weak]
                    obj,
                    move |_, role| {
                        if let Some(performer) = &mut *obj.imp().performer.borrow_mut() {
                            obj.imp().role_label.set_label(&role.to_string());
                            performer.role = role;
                            performer.instrument = None;
                        };
                    }
                ));

                obj.navigation().push(&editor);
            });

            let obj = self.obj().to_owned();
            role_popover.connect_create_instrument(move |_| {
                let editor = InstrumentEditor::new(&obj.navigation(), &obj.library(), None);

                editor.connect_created(clone!(
                    #[weak]
                    obj,
                    move |_, instrument| {
                        if let Some(performer) = &mut *obj.imp().performer.borrow_mut() {
                            obj.imp().role_label.set_label(&instrument.to_string());
                            performer.role = obj.library().performer_default_role().unwrap();
                            performer.instrument = Some(instrument);
                        };
                    }
                ));

                obj.navigation().push(&editor);
            });

            self.role_box.append(&role_popover);
            self.role_popover.set(role_popover).unwrap();
        }
    }

    impl WidgetImpl for RecordingEditorPerformerRow {}
    impl ListBoxRowImpl for RecordingEditorPerformerRow {}
    impl PreferencesRowImpl for RecordingEditorPerformerRow {}
    impl ActionRowImpl for RecordingEditorPerformerRow {}
}

glib::wrapper! {
    pub struct RecordingEditorPerformerRow(ObjectSubclass<imp::RecordingEditorPerformerRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::ActionRow;
}

#[gtk::template_callbacks]
impl RecordingEditorPerformerRow {
    pub fn new(navigation: &adw::NavigationView, library: &Library, performer: Performer) -> Self {
        let obj: Self = glib::Object::builder()
            .property("navigation", navigation)
            .property("library", library)
            .build();
        obj.set_performer(performer);
        obj
    }

    pub fn connect_remove<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("remove", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            f(&obj);
            None
        })
    }

    pub fn performer(&self) -> Performer {
        self.imp().performer.borrow().to_owned().unwrap()
    }

    fn set_performer(&self, performer: Performer) {
        self.set_title(&performer.person.to_string());

        let label = match &performer.instrument {
            Some(instrument) => instrument.to_string(),
            None => performer.role.to_string(),
        };

        self.imp().role_label.set_label(&label.to_string());
        self.imp().performer.replace(Some(performer));
    }

    #[template_callback]
    fn open_role_popover(&self) {
        self.imp().role_popover.get().unwrap().popup();
    }

    #[template_callback]
    fn remove(&self) {
        self.emit_by_name::<()>("remove", &[]);
    }
}
