use std::cell::{OnceCell, RefCell};

use adw::{prelude::*, subclass::prelude::*};
use gtk::glib::{self, clone, subclass::Signal, Properties};
use once_cell::sync::Lazy;

use crate::{
    db::models::EnsemblePerformer,
    editor::{role_editor::MusicusRoleEditor, role_selector_popover::MusicusRoleSelectorPopover},
    library::MusicusLibrary,
};

mod imp {
    use super::*;

    #[derive(Properties, Debug, Default, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::MusicusRecordingEditorEnsembleRow)]
    #[template(file = "data/ui/recording_editor_ensemble_row.blp")]
    pub struct MusicusRecordingEditorEnsembleRow {
        #[property(get, construct_only)]
        pub navigation: OnceCell<adw::NavigationView>,

        #[property(get, construct_only)]
        pub library: OnceCell<MusicusLibrary>,

        pub ensemble: RefCell<Option<EnsemblePerformer>>,
        pub role_popover: OnceCell<MusicusRoleSelectorPopover>,

        #[template_child]
        pub role_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub role_box: TemplateChild<gtk::Box>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MusicusRecordingEditorEnsembleRow {
        const NAME: &'static str = "MusicusRecordingEditorEnsembleRow";
        type Type = super::MusicusRecordingEditorEnsembleRow;
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
    impl ObjectImpl for MusicusRecordingEditorEnsembleRow {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> =
                Lazy::new(|| vec![Signal::builder("remove").build()]);

            SIGNALS.as_ref()
        }

        fn constructed(&self) {
            self.parent_constructed();

            let role_popover = MusicusRoleSelectorPopover::new(self.library.get().unwrap());

            let obj = self.obj().to_owned();
            role_popover.connect_role_selected(move |_, role| {
                if let Some(ensemble) = &mut *obj.imp().ensemble.borrow_mut() {
                    obj.imp().role_label.set_label(&role.to_string());
                    ensemble.role = role;
                }
            });

            let obj = self.obj().to_owned();
            role_popover.connect_create(move |_| {
                let editor = MusicusRoleEditor::new(&obj.navigation(), &obj.library(), None);

                editor.connect_created(clone!(
                    #[weak]
                    obj,
                    move |_, role| {
                        if let Some(ensemble) = &mut *obj.imp().ensemble.borrow_mut() {
                            obj.imp().role_label.set_label(&role.to_string());
                            ensemble.role = role;
                        };
                    }
                ));

                obj.navigation().push(&editor);
            });

            self.role_box.append(&role_popover);
            self.role_popover.set(role_popover).unwrap();
        }
    }

    impl WidgetImpl for MusicusRecordingEditorEnsembleRow {}
    impl ListBoxRowImpl for MusicusRecordingEditorEnsembleRow {}
    impl PreferencesRowImpl for MusicusRecordingEditorEnsembleRow {}
    impl ActionRowImpl for MusicusRecordingEditorEnsembleRow {}
}

glib::wrapper! {
    pub struct MusicusRecordingEditorEnsembleRow(ObjectSubclass<imp::MusicusRecordingEditorEnsembleRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::ActionRow;
}

#[gtk::template_callbacks]
impl MusicusRecordingEditorEnsembleRow {
    pub fn new(
        navigation: &adw::NavigationView,
        library: &MusicusLibrary,
        ensemble: EnsemblePerformer,
    ) -> Self {
        let obj: Self = glib::Object::builder()
            .property("navigation", navigation)
            .property("library", library)
            .build();
        obj.set_ensemble(ensemble);
        obj
    }

    pub fn connect_remove<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("remove", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            f(&obj);
            None
        })
    }

    pub fn ensemble(&self) -> EnsemblePerformer {
        self.imp().ensemble.borrow().to_owned().unwrap()
    }

    fn set_ensemble(&self, ensemble: EnsemblePerformer) {
        self.set_title(&ensemble.ensemble.to_string());
        self.imp().role_label.set_label(&ensemble.role.to_string());
        self.imp().ensemble.replace(Some(ensemble));
    }

    #[template_callback]
    fn open_role_popover(&self, _: &gtk::Button) {
        self.imp().role_popover.get().unwrap().popup();
    }

    #[template_callback]
    fn remove(&self, _: &gtk::Button) {
        self.emit_by_name::<()>("remove", &[]);
    }
}
