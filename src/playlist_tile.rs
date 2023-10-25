use crate::playlist_item::PlaylistItem;
use gtk::{glib, prelude::*, subclass::prelude::*};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/playlist_tile.blp")]
    pub struct PlaylistTile {
        #[template_child]
        pub playing_icon: TemplateChild<gtk::Image>,
        #[template_child]
        pub title_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub performances_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub part_title_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PlaylistTile {
        const NAME: &'static str = "MusicusPlaylistTile";
        type Type = super::PlaylistTile;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PlaylistTile {}
    impl WidgetImpl for PlaylistTile {}
    impl BoxImpl for PlaylistTile {}
}

glib::wrapper! {
    pub struct PlaylistTile(ObjectSubclass<imp::PlaylistTile>)
        @extends gtk::Widget, gtk::FlowBoxChild;
}

impl PlaylistTile {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_item(&self, item: &PlaylistItem) {
        let imp = self.imp();

        if let Some(title) = item.title() {
            imp.title_label.set_label(&title);
            imp.title_label.set_visible(true);
        }

        if let Some(performances) = item.performers() {
            imp.performances_label.set_label(&performances);
            imp.performances_label.set_visible(true);
        }

        if let Some(part_title) = item.part_title() {
            imp.part_title_label.set_label(&part_title);
            imp.part_title_label.set_visible(true);
        } else {
            imp.obj().set_margin_bottom(24);
        }
    }

    pub fn set_playing(&self, playing: bool) {
        self.imp().playing_icon.set_visible(playing);
    }
}
