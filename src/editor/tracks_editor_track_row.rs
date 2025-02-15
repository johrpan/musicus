use crate::{
    db::models::{Recording, Work},
    library::MusicusLibrary,
};

use adw::{prelude::*, subclass::prelude::*};
use formatx::formatx;
use gettextrs::gettext;
use gtk::glib::{self, clone, subclass::Signal, Properties};
use once_cell::sync::Lazy;

use std::{
    cell::{OnceCell, RefCell},
    path::PathBuf,
};

mod imp {
    use super::*;

    #[derive(Properties, Debug, Default, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::TracksEditorTrackRow)]
    #[template(file = "data/ui/tracks_editor_track_row.blp")]
    pub struct TracksEditorTrackRow {
        #[property(get, construct_only)]
        pub navigation: OnceCell<adw::NavigationView>,
        #[property(get, construct_only)]
        pub library: OnceCell<MusicusLibrary>,

        pub recording: OnceCell<Recording>,
        pub track_data: RefCell<TracksEditorTrackData>,

        #[template_child]
        pub select_parts_box: TemplateChild<gtk::Box>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TracksEditorTrackRow {
        const NAME: &'static str = "MusicusTracksEditorTrackRow";
        type Type = super::TracksEditorTrackRow;
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
    impl ObjectImpl for TracksEditorTrackRow {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> =
                Lazy::new(|| vec![Signal::builder("remove").build()]);

            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for TracksEditorTrackRow {}
    impl ListBoxRowImpl for TracksEditorTrackRow {}
    impl PreferencesRowImpl for TracksEditorTrackRow {}
    impl ActionRowImpl for TracksEditorTrackRow {}
}

glib::wrapper! {
    pub struct TracksEditorTrackRow(ObjectSubclass<imp::TracksEditorTrackRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::ActionRow;
}

#[gtk::template_callbacks]
impl TracksEditorTrackRow {
    pub fn new(
        navigation: &adw::NavigationView,
        library: &MusicusLibrary,
        recording: Recording,
        track_data: TracksEditorTrackData,
    ) -> Self {
        let obj: Self = glib::Object::builder()
            .property("navigation", navigation)
            .property("library", library)
            .build();

        obj.set_activatable(!recording.work.parts.is_empty());

        obj.set_subtitle(&match &track_data.path {
            PathType::None => String::new(),
            PathType::Library(path) => path.to_owned(),
            PathType::System(path) => {
                let format_string = gettext("Import from {}");
                let file_name = path.file_name().unwrap().to_str().unwrap();
                match formatx!(&format_string, file_name) {
                    Ok(title) => title,
                    Err(_) => {
                        log::error!("Error in translated format string: {format_string}");
                        file_name.to_owned()
                    }
                }
            }
        });

        obj.imp().recording.set(recording).unwrap();
        obj.imp().track_data.replace(track_data);
        obj.update_title();

        obj
    }

    pub fn connect_remove<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("remove", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            f(&obj);
            None
        })
    }

    pub fn track_data(&self) -> TracksEditorTrackData {
        self.imp().track_data.borrow().to_owned()
    }

    #[template_callback]
    fn select_parts(&self) {
        // self.imp().parts_popover.get().unwrap().popup();
    }

    #[template_callback]
    fn remove(&self) {
        self.emit_by_name::<()>("remove", &[]);
    }

    fn update_title(&self) {
        let parts = &self.imp().track_data.borrow().parts;

        self.set_title(&if parts.is_empty() {
            if self.imp().recording.get().unwrap().work.parts.is_empty() {
                gettext("Whole work")
            } else {
                gettext("Select parts")
            }
        } else {
            parts
                .iter()
                .map(|w| w.name.get())
                .collect::<Vec<&str>>()
                .join(", ")
        });
    }
}

#[derive(Clone, Default, Debug)]
pub struct TracksEditorTrackData {
    pub track_id: Option<String>,
    pub path: PathType,
    pub parts: Vec<Work>,
}

#[derive(Clone, Default, Debug)]
pub enum PathType {
    #[default]
    None,
    Library(String),
    System(PathBuf),
}
