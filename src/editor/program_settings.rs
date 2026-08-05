use adw::{prelude::*, subclass::prelude::*};
use gtk::glib;

use crate::{program::Program, slider_row::SliderRow};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/editor/program_settings.blp")]
    pub struct ProgramSettings {
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
    impl ObjectSubclass for ProgramSettings {
        const NAME: &'static str = "MusicusProgramSettings";
        type Type = super::ProgramSettings;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            SliderRow::static_type();
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ProgramSettings {}
    impl WidgetImpl for ProgramSettings {}
    impl BinImpl for ProgramSettings {}
}

glib::wrapper! {
    pub struct ProgramSettings(ObjectSubclass<imp::ProgramSettings>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl ProgramSettings {
    pub fn load(&self, program: &Program) {
        let imp = self.imp();

        imp.prefer_least_recently_played_adjustment
            .set_value(program.prefer_least_recently_played() * 100.0);

        imp.prefer_recently_added_adjustment
            .set_value(program.prefer_recently_added() * 100.0);

        imp.avoid_repeated_composers_adjustment
            .set_value(program.avoid_repeated_composers() as f64);

        imp.avoid_repeated_instruments_adjustment
            .set_value(program.avoid_repeated_instruments() as f64);

        imp.play_full_recordings_row
            .set_active(program.play_full_recordings());
    }

    pub fn apply(&self, program: &Program) {
        let imp = self.imp();

        program.set_prefer_least_recently_played(
            imp.prefer_least_recently_played_adjustment.value() / 100.0,
        );

        program.set_prefer_recently_added(imp.prefer_recently_added_adjustment.value() / 100.0);

        program
            .set_avoid_repeated_composers(imp.avoid_repeated_composers_adjustment.value() as i32);

        program.set_avoid_repeated_instruments(
            imp.avoid_repeated_instruments_adjustment.value() as i32
        );

        program.set_play_full_recordings(imp.play_full_recordings_row.is_active());
    }
}

impl Default for ProgramSettings {
    fn default() -> Self {
        glib::Object::new()
    }
}
