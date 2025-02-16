use std::cell::{OnceCell, RefCell};

use adw::{prelude::*, subclass::prelude::*};
use gtk::glib::{self, clone, subclass::Signal, Properties};
use once_cell::sync::Lazy;

use crate::{
    db::models::Composer,
    editor::{role_editor::MusicusRoleEditor, role_selector_popover::MusicusRoleSelectorPopover},
    library::MusicusLibrary,
};

mod imp {
    use super::*;

    #[derive(Properties, Debug, Default, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::MusicusWorkEditorComposerRow)]
    #[template(file = "data/ui/work_editor_composer_row.blp")]
    pub struct MusicusWorkEditorComposerRow {
        #[property(get, construct_only)]
        pub navigation: OnceCell<adw::NavigationView>,

        #[property(get, construct_only)]
        pub library: OnceCell<MusicusLibrary>,

        pub composer: RefCell<Option<Composer>>,
        pub role_popover: OnceCell<MusicusRoleSelectorPopover>,

        #[template_child]
        pub role_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub role_box: TemplateChild<gtk::Box>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MusicusWorkEditorComposerRow {
        const NAME: &'static str = "MusicusWorkEditorComposerRow";
        type Type = super::MusicusWorkEditorComposerRow;
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
    impl ObjectImpl for MusicusWorkEditorComposerRow {
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
                if let Some(composer) = &mut *obj.imp().composer.borrow_mut() {
                    obj.imp().role_label.set_label(&role.to_string());
                    composer.role = role;
                }
            });

            let obj = self.obj().to_owned();
            role_popover.connect_create(move |_| {
                let editor = MusicusRoleEditor::new(&obj.navigation(), &obj.library(), None);

                editor.connect_created(clone!(
                    #[weak]
                    obj,
                    move |_, role| {
                        if let Some(composer) = &mut *obj.imp().composer.borrow_mut() {
                            obj.imp().role_label.set_label(&role.to_string());
                            composer.role = role;
                        };
                    }
                ));

                obj.navigation().push(&editor);
            });

            self.role_box.append(&role_popover);
            self.role_popover.set(role_popover).unwrap();
        }
    }

    impl WidgetImpl for MusicusWorkEditorComposerRow {}
    impl ListBoxRowImpl for MusicusWorkEditorComposerRow {}
    impl PreferencesRowImpl for MusicusWorkEditorComposerRow {}
    impl ActionRowImpl for MusicusWorkEditorComposerRow {}
}

glib::wrapper! {
    pub struct MusicusWorkEditorComposerRow(ObjectSubclass<imp::MusicusWorkEditorComposerRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::ActionRow;
}

#[gtk::template_callbacks]
impl MusicusWorkEditorComposerRow {
    pub fn new(
        navigation: &adw::NavigationView,
        library: &MusicusLibrary,
        composer: Composer,
    ) -> Self {
        let obj: Self = glib::Object::builder()
            .property("navigation", navigation)
            .property("library", library)
            .build();
        obj.set_composer(composer);
        obj
    }

    pub fn connect_remove<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("remove", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            f(&obj);
            None
        })
    }

    pub fn composer(&self) -> Composer {
        self.imp().composer.borrow().to_owned().unwrap()
    }

    fn set_composer(&self, composer: Composer) {
        self.set_title(&composer.person.to_string());
        self.imp().role_label.set_label(&composer.role.to_string());
        self.imp().composer.replace(Some(composer));
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
