use adw::subclass::prelude::*;
use gtk::{glib, glib::subclass::Signal, prelude::*};
use once_cell::sync::Lazy;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/playlist_page.blp")]
    pub struct MusicusPlaylistPage {}

    #[glib::object_subclass]
    impl ObjectSubclass for MusicusPlaylistPage {
        const NAME: &'static str = "MusicusPlaylistPage";
        type Type = super::MusicusPlaylistPage;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for MusicusPlaylistPage {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> =
                Lazy::new(|| vec![Signal::builder("close").build()]);

            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for MusicusPlaylistPage {}
    impl BinImpl for MusicusPlaylistPage {}
}

glib::wrapper! {
    pub struct MusicusPlaylistPage(ObjectSubclass<imp::MusicusPlaylistPage>)
        @extends gtk::Widget, adw::Bin;
}

#[gtk::template_callbacks]
impl MusicusPlaylistPage {
    pub fn new() -> Self {
        glib::Object::new()
    }

    #[template_callback]
    fn close(&self, _: &gtk::Button) {
        self.emit_by_name::<()>("close", &[]);
    }
}
