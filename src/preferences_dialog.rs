use adw::{
    prelude::{ActionRowExt, AdwDialogExt},
    subclass::prelude::*,
};
use gettextrs::gettext;
use gtk::{gio, glib, prelude::*};
use musicus_library::library::{audio_tags, filenames, pattern};

use crate::{config, slider_row::SliderRow};

/// One of the patterns that describe how a track file is named and tagged.
///
/// All four rows behave the same way; they only differ in which setting they
/// belong to and which of the two rule sets validates them.
#[derive(Clone, Copy)]
enum PatternKind {
    FileName,
    Album,
    Artist,
    Title,
}

impl PatternKind {
    const ALL: &'static [Self] = &[Self::FileName, Self::Album, Self::Artist, Self::Title];
    const TAGS: &'static [Self] = &[Self::Album, Self::Artist, Self::Title];

    fn key(self) -> &'static str {
        match self {
            Self::FileName => "track-filename-pattern",
            Self::Album => "track-tag-album-pattern",
            Self::Artist => "track-tag-artist-pattern",
            Self::Title => "track-tag-title-pattern",
        }
    }

    fn default_pattern(self) -> &'static str {
        match self {
            Self::FileName => pattern::DEFAULT_FILENAME_PATTERN,
            Self::Album => pattern::DEFAULT_ALBUM_PATTERN,
            Self::Artist => pattern::DEFAULT_ARTIST_PATTERN,
            Self::Title => pattern::DEFAULT_TITLE_PATTERN,
        }
    }

    fn validate(self, pattern: &str) -> anyhow::Result<()> {
        match self {
            Self::FileName => filenames::validate(pattern),
            _ => audio_tags::validate(pattern),
        }
    }

    /// What the pattern would produce for the example track.
    fn preview(self, pattern: &str) -> anyhow::Result<String> {
        match self {
            Self::FileName => filenames::preview(pattern),
            // An empty tag is a valid configuration rather than a preview.
            _ => audio_tags::preview(pattern).map(|value| {
                if value.is_empty() {
                    gettext("No tag is written.")
                } else {
                    value
                }
            }),
        }
    }
}

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
        pub album_pattern_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub album_pattern_preview_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub artist_pattern_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub artist_pattern_preview_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub title_pattern_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub title_pattern_preview_row: TemplateChild<adw::ActionRow>,
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

            // Unlike the other rows, the patterns are not bound to their
            // setting: an invalid one may not be saved, so they are applied
            // explicitly.
            for kind in PatternKind::ALL {
                let (row, _) = self.obj().pattern_rows(*kind);
                row.set_text(&settings.string(kind.key()));
                self.obj().update_pattern_preview(*kind);
            }

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

    /// The entry and the preview row belonging to `kind`.
    fn pattern_rows(&self, kind: PatternKind) -> (adw::EntryRow, adw::ActionRow) {
        let imp = self.imp();

        match kind {
            PatternKind::FileName => (
                imp.track_filename_pattern_row.get(),
                imp.filename_pattern_preview_row.get(),
            ),
            PatternKind::Album => (
                imp.album_pattern_row.get(),
                imp.album_pattern_preview_row.get(),
            ),
            PatternKind::Artist => (
                imp.artist_pattern_row.get(),
                imp.artist_pattern_preview_row.get(),
            ),
            PatternKind::Title => (
                imp.title_pattern_row.get(),
                imp.title_pattern_preview_row.get(),
            ),
        }
    }

    /// Show what the pattern currently in the entry would produce, or why it
    /// cannot be used.
    fn update_pattern_preview(&self, kind: PatternKind) {
        let (row, preview_row) = self.pattern_rows(kind);

        match kind.preview(&row.text()) {
            Ok(preview) => {
                row.remove_css_class("error");
                preview_row.set_subtitle(&preview);
            }
            Err(err) => {
                row.add_css_class("error");
                preview_row.set_subtitle(&err.to_string());
            }
        }
    }

    fn apply_pattern(&self, kind: PatternKind) {
        let (row, _) = self.pattern_rows(kind);
        let pattern = row.text();

        if let Err(err) = kind.validate(&pattern) {
            log::warn!("Not saving an unusable pattern for {}: {err:?}", kind.key());
            return;
        }

        let settings = gio::Settings::new(config::APP_ID);
        if let Err(err) = settings.set_string(kind.key(), &pattern) {
            log::error!("Failed to save the pattern for {}: {err:?}", kind.key());
        }
    }

    fn reset_patterns(&self, kinds: &[PatternKind]) {
        for kind in kinds {
            let (row, _) = self.pattern_rows(*kind);
            row.set_text(kind.default_pattern());
            self.apply_pattern(*kind);
        }
    }

    #[template_callback]
    fn filename_pattern_changed(&self) {
        self.update_pattern_preview(PatternKind::FileName);
    }

    #[template_callback]
    fn apply_filename_pattern(&self) {
        self.apply_pattern(PatternKind::FileName);
    }

    #[template_callback]
    fn reset_filename_pattern(&self) {
        self.reset_patterns(&[PatternKind::FileName]);
    }

    #[template_callback]
    fn album_pattern_changed(&self) {
        self.update_pattern_preview(PatternKind::Album);
    }

    #[template_callback]
    fn apply_album_pattern(&self) {
        self.apply_pattern(PatternKind::Album);
    }

    #[template_callback]
    fn artist_pattern_changed(&self) {
        self.update_pattern_preview(PatternKind::Artist);
    }

    #[template_callback]
    fn apply_artist_pattern(&self) {
        self.apply_pattern(PatternKind::Artist);
    }

    #[template_callback]
    fn title_pattern_changed(&self) {
        self.update_pattern_preview(PatternKind::Title);
    }

    #[template_callback]
    fn apply_title_pattern(&self) {
        self.apply_pattern(PatternKind::Title);
    }

    #[template_callback]
    fn reset_tag_patterns(&self) {
        self.reset_patterns(PatternKind::TAGS);
    }
}
