use crate::{
    db::models::{Album, Recording},
    editor::{
        recording_editor::MusicusRecordingEditor,
        recording_selector_popover::RecordingSelectorPopover,
        translation_editor::MusicusTranslationEditor,
    },
    library::MusicusLibrary,
};

use adw::{prelude::*, subclass::prelude::*};
use gettextrs::gettext;
use gtk::glib::{
    clone, Properties,
    {self, subclass::Signal},
};
use once_cell::sync::Lazy;

use std::cell::{OnceCell, RefCell};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, Properties)]
    #[properties(wrapper_type = super::AlbumEditor)]
    #[template(file = "data/ui/album_editor.blp")]
    pub struct AlbumEditor {
        #[property(get, construct_only)]
        pub navigation: OnceCell<adw::NavigationView>,
        #[property(get, construct_only)]
        pub library: OnceCell<MusicusLibrary>,

        pub album_id: OnceCell<String>,
        pub recordings: RefCell<Vec<Recording>>,

        pub recordings_popover: OnceCell<RecordingSelectorPopover>,

        #[template_child]
        pub name_editor: TemplateChild<MusicusTranslationEditor>,
        #[template_child]
        pub recordings_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub select_recording_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub save_row: TemplateChild<adw::ButtonRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AlbumEditor {
        const NAME: &'static str = "MusicusAlbumEditor";
        type Type = super::AlbumEditor;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            MusicusTranslationEditor::static_type();
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for AlbumEditor {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("created")
                    .param_types([Album::static_type()])
                    .build()]
            });

            SIGNALS.as_ref()
        }

        fn constructed(&self) {
            self.parent_constructed();

            let recordings_popover = RecordingSelectorPopover::new(self.library.get().unwrap());

            let obj = self.obj().clone();
            recordings_popover.connect_selected(move |_, recording| {
                obj.add_recording(recording);
            });

            let obj = self.obj().clone();
            recordings_popover.connect_create(move |_| {
                let editor = MusicusRecordingEditor::new(&obj.navigation(), &obj.library(), None);

                editor.connect_created(clone!(
                    #[weak]
                    obj,
                    move |_, recording| {
                        obj.add_recording(recording);
                    }
                ));

                obj.navigation().push(&editor);
            });

            self.select_recording_box.append(&recordings_popover);
            self.recordings_popover.set(recordings_popover).unwrap();
        }
    }

    impl WidgetImpl for AlbumEditor {}
    impl NavigationPageImpl for AlbumEditor {}
}

glib::wrapper! {
    pub struct AlbumEditor(ObjectSubclass<imp::AlbumEditor>)
        @extends gtk::Widget, adw::NavigationPage;
}

#[gtk::template_callbacks]
impl AlbumEditor {
    pub fn new(
        navigation: &adw::NavigationView,
        library: &MusicusLibrary,
        album: Option<&Album>,
    ) -> Self {
        let obj: Self = glib::Object::builder()
            .property("navigation", navigation)
            .property("library", library)
            .build();

        if let Some(album) = album {
            obj.imp().save_row.set_title(&gettext("Save changes"));
            obj.imp().album_id.set(album.album_id.clone()).unwrap();
            obj.imp().name_editor.set_translation(&album.name);

            for recording in &album.recordings {
                obj.add_recording(recording.to_owned());
            }
        }

        obj
    }

    pub fn connect_created<F: Fn(&Self, Album) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("created", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let album = values[1].get::<Album>().unwrap();
            f(&obj, album);
            None
        })
    }

    #[template_callback]
    fn select_recording(&self) {
        self.imp().recordings_popover.get().unwrap().popup();
    }

    fn add_recording(&self, recording: Recording) {
        let row = adw::ActionRow::builder()
            .title(recording.work.to_string())
            .subtitle(recording.performers_string())
            .build();

        let remove_button = gtk::Button::builder()
            .icon_name("user-trash-symbolic")
            .valign(gtk::Align::Center)
            .css_classes(["flat"])
            .build();

        remove_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            #[weak]
            row,
            #[strong]
            recording,
            move |_| {
                this.imp().recordings_list.remove(&row);
                this.imp()
                    .recordings
                    .borrow_mut()
                    .retain(|r| *r != recording);
            }
        ));

        row.add_suffix(&remove_button);

        self.imp()
            .recordings_list
            .insert(&row, self.imp().recordings.borrow().len() as i32);

        self.imp().recordings.borrow_mut().push(recording);
    }

    #[template_callback]
    fn save(&self) {
        let library = self.imp().library.get().unwrap();

        let name = self.imp().name_editor.translation();
        let recordings = self.imp().recordings.borrow().clone();

        if let Some(album_id) = self.imp().album_id.get() {
            library.update_album(album_id, name, recordings).unwrap();
        } else {
            let album = library.create_album(name, recordings).unwrap();
            self.emit_by_name::<()>("created", &[&album]);
        }

        self.imp().navigation.get().unwrap().pop();
    }
}
