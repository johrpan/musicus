use std::cell::{OnceCell, RefCell};

use adw::{prelude::*, subclass::prelude::*};
use gettextrs::gettext;
use gtk::{
    gdk,
    glib::{self, clone, subclass::Signal, Properties},
};
use once_cell::sync::Lazy;

use crate::{
    db::models::EnsemblePerformer, editor::role::RoleEditor, library::Library,
    selector::role::RoleSelectorPopover, util::drag_widget::DragWidget,
};

mod imp {
    use super::*;

    #[derive(Properties, Debug, Default, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::RecordingEditorEnsembleRow)]
    #[template(file = "data/ui/editor/recording/ensemble_row.blp")]
    pub struct RecordingEditorEnsembleRow {
        #[property(get, construct_only)]
        pub navigation: OnceCell<adw::NavigationView>,

        #[property(get, construct_only)]
        pub library: OnceCell<Library>,

        pub ensemble: RefCell<Option<EnsemblePerformer>>,
        pub role_popover: OnceCell<RoleSelectorPopover>,

        #[template_child]
        pub role_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub role_box: TemplateChild<gtk::Box>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RecordingEditorEnsembleRow {
        const NAME: &'static str = "MusicusRecordingEditorEnsembleRow";
        type Type = super::RecordingEditorEnsembleRow;
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
    impl ObjectImpl for RecordingEditorEnsembleRow {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("remove").build(),
                    Signal::builder("move")
                        .param_types([super::RecordingEditorEnsembleRow::static_type()])
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

            let role_popover = RoleSelectorPopover::new(self.library.get().unwrap());

            let obj = self.obj().to_owned();
            role_popover.connect_role_selected(move |_, role| {
                if let Some(ensemble) = &mut *obj.imp().ensemble.borrow_mut() {
                    obj.imp().role_label.set_label(&role.to_string());
                    ensemble.role = Some(role);
                }
            });

            let obj = self.obj().to_owned();
            role_popover.connect_create(move |_| {
                let editor = RoleEditor::new(&obj.navigation(), &obj.library(), None);

                editor.connect_created(clone!(
                    #[weak]
                    obj,
                    move |_, role| {
                        if let Some(ensemble) = &mut *obj.imp().ensemble.borrow_mut() {
                            obj.imp().role_label.set_label(&role.to_string());
                            ensemble.role = Some(role);
                        };
                    }
                ));

                obj.navigation().push(&editor);
            });

            self.role_box.append(&role_popover);
            self.role_popover.set(role_popover).unwrap();
        }
    }

    impl WidgetImpl for RecordingEditorEnsembleRow {}
    impl ListBoxRowImpl for RecordingEditorEnsembleRow {}
    impl PreferencesRowImpl for RecordingEditorEnsembleRow {}
    impl ActionRowImpl for RecordingEditorEnsembleRow {}
}

glib::wrapper! {
    pub struct RecordingEditorEnsembleRow(ObjectSubclass<imp::RecordingEditorEnsembleRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::ActionRow;
}

#[gtk::template_callbacks]
impl RecordingEditorEnsembleRow {
    pub fn new(
        navigation: &adw::NavigationView,
        library: &Library,
        ensemble: EnsemblePerformer,
    ) -> Self {
        let obj: Self = glib::Object::builder()
            .property("navigation", navigation)
            .property("library", library)
            .build();
        obj.set_ensemble(ensemble);
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

    pub fn ensemble(&self) -> EnsemblePerformer {
        self.imp().ensemble.borrow().to_owned().unwrap()
    }

    fn set_ensemble(&self, ensemble: EnsemblePerformer) {
        self.set_title(&ensemble.ensemble.to_string());
        self.imp().role_label.set_label(
            &ensemble
                .role
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| gettext("Performer")),
        );
        self.imp().ensemble.replace(Some(ensemble));
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
