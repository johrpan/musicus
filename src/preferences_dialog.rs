use adw::{prelude::AdwDialogExt, subclass::prelude::*};
use gtk::{gio, glib, prelude::*};

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
        }
    }

    impl WidgetImpl for PreferencesDialog {}
    impl AdwDialogImpl for PreferencesDialog {}
    impl PreferencesDialogImpl for PreferencesDialog {}
}

glib::wrapper! {
    pub struct PreferencesDialog(ObjectSubclass<imp::PreferencesDialog>)
        @extends gtk::Widget, adw::Dialog, adw::PreferencesDialog;
}

#[gtk::template_callbacks]
impl PreferencesDialog {
    pub fn show(parent: &impl IsA<gtk::Widget>) {
        let obj: Self = glib::Object::new();
        obj.present(Some(parent));
    }
}
