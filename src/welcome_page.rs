use adw::subclass::{navigation_page::NavigationPageImpl, prelude::*};
use gettextrs::gettext;
use gtk::{gio, glib, glib::subclass::Signal, prelude::*};
use once_cell::sync::Lazy;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/welcome_page.blp")]
    pub struct WelcomePage {
        #[template_child]
        pub status_page: TemplateChild<adw::StatusPage>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for WelcomePage {
        const NAME: &'static str = "MusicusWelcomePage";
        type Type = super::WelcomePage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for WelcomePage {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("folder-selected")
                    .param_types([gio::File::static_type()])
                    .build()]
            });

            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for WelcomePage {}
    impl NavigationPageImpl for WelcomePage {}
}

glib::wrapper! {
    pub struct WelcomePage(ObjectSubclass<imp::WelcomePage>)
        @extends gtk::Widget, adw::NavigationPage;
}

#[gtk::template_callbacks]
impl WelcomePage {
    pub fn new() -> Self {
        glib::Object::new()
    }

    #[template_callback]
    async fn choose_library_folder(&self) {
        let dialog = gtk::FileDialog::builder()
            .title(gettext("Select music library folder"))
            .modal(true)
            .build();

        match dialog
            .select_folder_future(
                self.root()
                    .as_ref()
                    .and_then(|r| r.downcast_ref::<gtk::Window>()),
            )
            .await
        {
            Err(err) => {
                if !err.matches(gtk::DialogError::Dismissed) {
                    log::error!("Folder selection failed: {err:?}");
                }
            }
            Ok(folder) => {
                self.emit_by_name::<()>("folder-selected", &[&folder]);
            }
        }
    }
}
