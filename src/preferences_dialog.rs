use adw::{
    prelude::{ActionRowExt, AdwDialogExt},
    subclass::prelude::*,
};
use gtk::{gio, glib, prelude::*};
use musicus_library::library::filenames;

use crate::{config, slider_row::SliderRow};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/preferences_dialog.blp")]
    pub struct PreferencesDialog {
        #[template_child]
        pub prefer_least_recently_played_adjustment: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub prefer_recently_added_adjustment: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub avoid_repeated_composers_adjustment: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub avoid_repeated_instruments_adjustment: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub play_full_recordings_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub track_filename_pattern_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub filename_pattern_preview_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub enable_automatic_metadata_updates_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub use_custom_metadata_url_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub custom_metadata_url_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub use_custom_library_url_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub custom_library_url_row: TemplateChild<adw::EntryRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PreferencesDialog {
        const NAME: &'static str = "MusicusPreferencesDialog";
        type Type = super::PreferencesDialog;
        type ParentType = adw::PreferencesDialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            SliderRow::static_type();
            obj.init_template();
        }
    }

    impl ObjectImpl for PreferencesDialog {
        fn constructed(&self) {
            self.parent_constructed();

            let settings = gio::Settings::new(config::APP_ID);

            settings
                .bind(
                    "prefer-least-recently-played",
                    &*self.prefer_least_recently_played_adjustment,
                    "value",
                )
                .build();

            settings
                .bind(
                    "prefer-recently-added",
                    &*self.prefer_recently_added_adjustment,
                    "value",
                )
                .build();

            settings
                .bind(
                    "avoid-repeated-composers",
                    &*self.avoid_repeated_composers_adjustment,
                    "value",
                )
                .build();

            settings
                .bind(
                    "avoid-repeated-instruments",
                    &*self.avoid_repeated_instruments_adjustment,
                    "value",
                )
                .build();

            settings
                .bind(
                    "play-full-recordings",
                    &*self.play_full_recordings_row,
                    "active",
                )
                .build();

            self.track_filename_pattern_row
                .set_text(&settings.string("track-filename-pattern"));
            self.obj().filename_pattern_changed();

            settings
                .bind(
                    "enable-automatic-metadata-updates",
                    &*self.enable_automatic_metadata_updates_row,
                    "active",
                )
                .build();

            settings
                .bind(
                    "use-custom-metadata-url",
                    &*self.use_custom_metadata_url_row,
                    "active",
                )
                .build();

            settings
                .bind(
                    "custom-metadata-url",
                    &*self.custom_metadata_url_row,
                    "text",
                )
                .build();

            self.use_custom_metadata_url_row
                .bind_property("active", &*self.custom_metadata_url_row, "sensitive")
                .sync_create()
                .build();

            settings
                .bind(
                    "use-custom-library-url",
                    &*self.use_custom_library_url_row,
                    "active",
                )
                .build();

            settings
                .bind("custom-library-url", &*self.custom_library_url_row, "text")
                .build();

            self.use_custom_library_url_row
                .bind_property("active", &*self.custom_library_url_row, "sensitive")
                .sync_create()
                .build();
        }
    }

    impl WidgetImpl for PreferencesDialog {}
    impl AdwDialogImpl for PreferencesDialog {}
    impl PreferencesDialogImpl for PreferencesDialog {}
}

glib::wrapper! {
    pub struct PreferencesDialog(ObjectSubclass<imp::PreferencesDialog>)
        @extends adw::PreferencesDialog, adw::Dialog, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

#[gtk::template_callbacks]
impl PreferencesDialog {
    pub fn show(parent: &impl IsA<gtk::Widget>) {
        let obj: Self = glib::Object::new();
        obj.present(Some(parent));
    }

    #[template_callback]
    fn filename_pattern_changed(&self) {
        let row = self.imp().track_filename_pattern_row.get();
        let preview_row = self.imp().filename_pattern_preview_row.get();

        match filenames::preview(&row.text()) {
            Ok(name) => {
                row.remove_css_class("error");
                preview_row.set_subtitle(&name);
            }
            Err(err) => {
                row.add_css_class("error");
                preview_row.set_subtitle(&err.to_string());
            }
        }
    }

    #[template_callback]
    fn apply_filename_pattern(&self) {
        let pattern = self.imp().track_filename_pattern_row.text();

        if let Err(err) = filenames::validate(&pattern) {
            log::warn!("Not saving an unusable filename pattern: {err:?}");
            return;
        }

        let settings = gio::Settings::new(config::APP_ID);
        if let Err(err) = settings.set_string("track-filename-pattern", &pattern) {
            log::error!("Failed to save the filename pattern: {err:?}");
        }
    }

    #[template_callback]
    fn reset_filename_pattern(&self) {
        self.imp()
            .track_filename_pattern_row
            .set_text(filenames::DEFAULT_FILENAME_PATTERN);

        self.apply_filename_pattern();
    }
}
