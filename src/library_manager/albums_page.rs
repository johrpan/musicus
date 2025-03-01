use std::cell::{OnceCell, RefCell};

use adw::{prelude::*, subclass::prelude::*};
use gettextrs::gettext;
use gtk::glib::{self, clone};

use crate::{db::models::Album, editor::album::AlbumEditor, library::Library};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/library_manager_albums_page.blp")]
    pub struct AlbumsPage {
        pub navigation: OnceCell<adw::NavigationView>,
        pub library: OnceCell<Library>,
        pub albums: RefCell<Vec<Album>>,
        pub albums_filtered: RefCell<Vec<Album>>,

        #[template_child]
        pub search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub list: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AlbumsPage {
        const NAME: &'static str = "MusicusLibraryManagerAlbumsPage";
        type Type = super::AlbumsPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AlbumsPage {}
    impl WidgetImpl for AlbumsPage {}

    impl NavigationPageImpl for AlbumsPage {
        fn showing(&self) {
            self.parent_showing();
            self.obj().update();
        }
    }
}

glib::wrapper! {
    pub struct AlbumsPage(ObjectSubclass<imp::AlbumsPage>)
        @extends gtk::Widget, adw::NavigationPage;
}

#[gtk::template_callbacks]
impl AlbumsPage {
    pub fn new(navigation: &adw::NavigationView, library: &Library) -> Self {
        let obj: Self = glib::Object::new();
        let imp = obj.imp();

        imp.navigation.set(navigation.to_owned()).unwrap();
        imp.library.set(library.to_owned()).unwrap();

        obj
    }

    fn update(&self) {
        let albums = self.imp().library.get().unwrap().all_albums().unwrap();
        self.imp().albums.replace(albums);
        self.search_changed();
    }

    #[template_callback]
    fn search_changed(&self) {
        let albums_filtered = self
            .imp()
            .albums
            .borrow()
            .iter()
            .filter(|a| {
                a.name
                    .get()
                    .contains(&self.imp().search_entry.text().to_string())
            })
            .cloned()
            .collect::<Vec<Album>>();

        self.imp().list.remove_all();

        for album in albums_filtered {
            let row = adw::ActionRow::builder()
                .title(album.name.get())
                .activatable(true)
                .build();

            row.connect_activated(clone!(
                #[weak(rename_to = obj)]
                self,
                #[strong]
                album,
                move |_| {
                    obj.imp().navigation.get().unwrap().push(&AlbumEditor::new(
                        &obj.imp().navigation.get().unwrap(),
                        &obj.imp().library.get().unwrap(),
                        Some(&album),
                    ));
                }
            ));

            let delete_button = gtk::Button::builder()
                .icon_name("user-trash-symbolic")
                .tooltip_text(gettext("Delete album"))
                .valign(gtk::Align::Center)
                .css_classes(["flat"])
                .build();

            // TODO:
            // delete_button.connect_clicked(clone!(
            //     #[weak(rename_to = obj)]
            //     self,
            //     #[strong]
            //     album,
            //     move |_| {
            //         obj.imp().library.delete_album(&album.album_id).unwrap();
            //     }
            // ));

            row.add_suffix(&delete_button);

            self.imp().list.append(&row);
        }
    }

    #[template_callback]
    fn create(&self) {
        self.imp().navigation.get().unwrap().push(&AlbumEditor::new(
            &self.imp().navigation.get().unwrap(),
            &self.imp().library.get().unwrap(),
            None,
        ));
    }
}
