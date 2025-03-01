use std::{
    cell::{OnceCell, RefCell},
    path::PathBuf,
};

use adw::{prelude::*, subclass::prelude::*};
use formatx::formatx;
use gettextrs::gettext;
use gtk::{
    gdk,
    glib::{self, clone, subclass::Signal, Properties},
};
use once_cell::sync::Lazy;

use super::parts_popover::TracksEditorPartsPopover;
use crate::{
    db::models::{Recording, Track, Work},
    library::Library,
    util::drag_widget::DragWidget,
};

mod imp {
    use super::*;

    #[derive(Properties, Debug, Default, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::TracksEditorTrackRow)]
    #[template(file = "data/ui/editor/tracks/track_row.blp")]
    pub struct TracksEditorTrackRow {
        #[property(get, construct_only)]
        pub navigation: OnceCell<adw::NavigationView>,
        #[property(get, construct_only)]
        pub library: OnceCell<Library>,

        pub recording: OnceCell<Recording>,
        pub track_data: RefCell<TracksEditorTrackData>,

        pub parts_popover: OnceCell<TracksEditorPartsPopover>,

        #[template_child]
        pub select_parts_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub edit_image: TemplateChild<gtk::Image>,
        #[template_child]
        pub reset_button: TemplateChild<gtk::Button>,
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
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("remove").build(),
                    Signal::builder("move")
                        .param_types([super::TracksEditorTrackRow::static_type()])
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
        library: &Library,
        recording: Recording,
        track_data: TracksEditorTrackData,
    ) -> Self {
        let obj: Self = glib::Object::builder()
            .property("navigation", navigation)
            .property("library", library)
            .build();

        obj.set_activatable(!recording.work.parts.is_empty());
        obj.imp()
            .edit_image
            .set_visible(!recording.work.parts.is_empty());

        obj.set_subtitle(&match &track_data.location {
            TrackLocation::Undefined => String::new(),
            TrackLocation::Library(track) => track.path.clone(),
            TrackLocation::System(path) => {
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

        let parts_popover = TracksEditorPartsPopover::new(recording.work.parts.clone());

        parts_popover.connect_part_selected(clone!(
            #[weak]
            obj,
            move |_, part| {
                obj.imp().track_data.borrow_mut().parts.push(part);
                obj.parts_updated();
            }
        ));

        obj.imp().select_parts_box.append(&parts_popover);
        obj.imp().parts_popover.set(parts_popover).unwrap();

        obj.imp().recording.set(recording).unwrap();
        obj.imp().track_data.replace(track_data);
        obj.parts_updated();

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

    pub fn track_data(&self) -> TracksEditorTrackData {
        self.imp().track_data.borrow().to_owned()
    }

    #[template_callback]
    fn select_parts(&self) {
        self.imp().parts_popover.get().unwrap().popup();
    }

    #[template_callback]
    fn reset(&self) {
        self.imp().track_data.borrow_mut().parts.clear();
        self.parts_updated();
    }

    #[template_callback]
    fn remove(&self) {
        self.emit_by_name::<()>("remove", &[]);
    }

    fn parts_updated(&self) {
        let parts = &self.imp().track_data.borrow().parts;

        self.imp().reset_button.set_visible(!parts.is_empty());

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
    pub location: TrackLocation,
    pub parts: Vec<Work>,
}

#[derive(Clone, Default, Debug)]
pub enum TrackLocation {
    #[default]
    Undefined,
    Library(Track),
    System(PathBuf),
}
