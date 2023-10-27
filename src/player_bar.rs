use crate::player::MusicusPlayer;
use gtk::{
    glib::{self, subclass::Signal, Properties},
    prelude::*,
    subclass::prelude::*,
};
use once_cell::sync::Lazy;
use std::cell::RefCell;

mod imp {
    use super::*;

    #[derive(Properties, Debug, Default, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::PlayerBar)]
    #[template(file = "data/ui/player_bar.blp")]
    pub struct PlayerBar {
        #[property(get, construct_only)]
        pub player: RefCell<MusicusPlayer>,

        #[template_child]
        pub title_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub subtitle_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub back_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub playlist_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub forward_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub play_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub current_time_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub slider: TemplateChild<gtk::Scale>,
        #[template_child]
        pub remaining_time_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PlayerBar {
        const NAME: &'static str = "MusicusPlayerBar";
        type Type = super::PlayerBar;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for PlayerBar {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("show-playlist")
                    .param_types([glib::Type::BOOL])
                    .build()]
            });

            SIGNALS.as_ref()
        }

        fn constructed(&self) {
            self.parent_constructed();

            self.player
                .borrow()
                .bind_property("playing", &self.play_button.get(), "icon-name")
                .transform_to(|_, playing| {
                    Some(if playing {
                        "media-playback-pause-symbolic"
                    } else {
                        "media-playback-start-symbolic"
                    })
                })
                .sync_create()
                .build();
        }
    }

    impl WidgetImpl for PlayerBar {}
    impl BoxImpl for PlayerBar {}
}

glib::wrapper! {
    pub struct PlayerBar(ObjectSubclass<imp::PlayerBar>)
        @extends gtk::Widget, adw::Bin;
}

#[gtk::template_callbacks]
impl PlayerBar {
    pub fn new(player: &MusicusPlayer) -> Self {
        glib::Object::builder().property("player", player).build()
    }

    pub fn connect_show_playlist<F: Fn(&Self, bool) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("show-playlist", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let show = values[1].get::<bool>().unwrap();
            f(&obj, show);
            None
        })
    }

    pub fn playlist_hidden(&self) {
        self.imp().playlist_button.set_active(false);
    }

    #[template_callback]
    fn show_playlist(&self, button: &gtk::ToggleButton) {
        self.emit_by_name::<()>("show-playlist", &[&button.is_active()]);
    }

    #[template_callback]
    fn play_pause(&self, _: &gtk::Button) {
        let player = self.player();
        if player.playing() {
            player.pause();
        } else {
            player.play();
        }
    }
}
