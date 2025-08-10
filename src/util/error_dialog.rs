use std::cell::OnceCell;

use adw::{prelude::*, subclass::prelude::*};
use gettextrs::gettext;
use gtk::{
    gdk,
    glib::{self, Properties},
};

mod imp {
    use super::*;

    #[derive(Properties, Debug, Default, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::ErrorDialog)]
    #[template(file = "data/ui/error_dialog.blp")]
    pub struct ErrorDialog {
        #[property(get, construct_only)]
        pub error_text: OnceCell<String>,

        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,

        #[template_child]
        pub error_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ErrorDialog {
        const NAME: &'static str = "MusicusErrorDialog";
        type Type = super::ErrorDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for ErrorDialog {
        fn constructed(&self) {
            self.parent_constructed();
            self.error_label.set_label(&self.obj().error_text());
        }
    }

    impl WidgetImpl for ErrorDialog {}
    impl AdwDialogImpl for ErrorDialog {}
}

glib::wrapper! {
    pub struct ErrorDialog(ObjectSubclass<imp::ErrorDialog>)
        @extends gtk::Widget, adw::Dialog;
}

#[gtk::template_callbacks]
impl ErrorDialog {
    pub fn present(err: &anyhow::Error, parent: &impl IsA<gtk::Widget>) {
        let obj: Self = glib::Object::builder()
            .property("error-text", format!("{err:?}"))
            .build();

        obj.present(Some(parent));
    }

    #[template_callback]
    fn copy(&self) {
        if let Some(display) = gdk::Display::default() {
            display.clipboard().set_text(&self.error_text());
            self.imp()
                .toast_overlay
                .add_toast(adw::Toast::new(&gettext("Copied to clipboard")));
        }
    }
}
