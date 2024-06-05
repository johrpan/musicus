use gtk::{glib, subclass::prelude::*};
use std::cell::OnceCell;

use crate::db::models::Album;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/album_tile.blp")]
    pub struct MusicusAlbumTile {
        pub album: OnceCell<Album>,

        #[template_child]
        pub title_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MusicusAlbumTile {
        const NAME: &'static str = "MusicusAlbumTile";
        type Type = super::MusicusAlbumTile;
        type ParentType = gtk::FlowBoxChild;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for MusicusAlbumTile {}
    impl WidgetImpl for MusicusAlbumTile {}
    impl FlowBoxChildImpl for MusicusAlbumTile {}
}

glib::wrapper! {
    pub struct MusicusAlbumTile(ObjectSubclass<imp::MusicusAlbumTile>)
        @extends gtk::Widget, gtk::FlowBoxChild;
}

impl MusicusAlbumTile {
    pub fn new(album: &Album) -> Self {
        let obj: Self = glib::Object::new();

        obj.imp().title_label.set_label(&album.name.get());
        obj.imp().album.set(album.clone()).unwrap();

        obj
    }

    pub fn album(&self) -> &Album {
        self.imp().album.get().unwrap()
    }
}
