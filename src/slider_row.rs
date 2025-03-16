use std::cell::RefCell;

use adw::{prelude::*, subclass::prelude::*};
use gtk::glib::{self, clone};

mod imp {
    use super::*;

    #[derive(glib::Properties, gtk::CompositeTemplate, Debug, Default)]
    #[properties(wrapper_type = super::SliderRow)]
    #[template(file = "data/ui/slider_row.blp")]
    pub struct SliderRow {
        #[property(get, set)]
        pub adjustment: RefCell<gtk::Adjustment>,

        #[property(get, set)]
        pub suffix: RefCell<String>,

        #[template_child]
        pub value_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SliderRow {
        const NAME: &'static str = "MusicusSliderRow";
        type Type = super::SliderRow;
        type ParentType = adw::PreferencesRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for SliderRow {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj().to_owned();
            obj.connect_adjustment_notify(move |obj| {
                obj.adjustment().connect_value_changed(clone!(
                    #[weak]
                    obj,
                    move |_| obj.update()
                ));

                obj.update();
            });
        }
    }

    impl WidgetImpl for SliderRow {}
    impl ListBoxRowImpl for SliderRow {}
    impl PreferencesRowImpl for SliderRow {}
}

glib::wrapper! {
    pub struct SliderRow(ObjectSubclass<imp::SliderRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow;
}

#[gtk::template_callbacks]
impl SliderRow {
    /// Create a new slider row.
    ///
    /// The adjustment can be used to control the range and initial value of the slider. Use the
    /// adjustment's `value-changed` signal for getting updates. The current value is displayed
    /// next to the slider followed by `suffix`.
    pub fn new(title: &str, adjustment: &gtk::Adjustment, suffix: &str) -> Self {
        glib::Object::builder()
            .property("title", title)
            .property("adjustment", adjustment)
            .property("suffix", suffix)
            .build()
    }

    pub fn update(&self) {
        self.imp().value_label.set_label(&format!(
            "{:.0}{}",
            self.adjustment().value(),
            self.suffix()
        ));
    }
}
