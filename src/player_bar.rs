use crate::player::MusicusPlayer;
use gtk::{
    glib::{self, clone, subclass::Signal, Properties},
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

    impl PlayerBar {
        fn update(&self) {
            if let Some(item) = self.player.borrow().current_item() {
                let mut title = item.title();

                if let Some(part_title) = item.part_title() {
                    title.push_str(": ");
                    title.push_str(&part_title);
                }

                self.title_label.set_label(&title);

                if let Some(performances) = item.performers() {
                    self.subtitle_label.set_label(&performances);
                    self.subtitle_label.set_visible(true);
                } else {
                    self.subtitle_label.set_visible(false);
                }
            }
        }
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

            let player = self.player.borrow();

            player
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

            let obj = self.obj().clone();

            player.connect_current_index_notify(clone!(@weak obj => move |_| obj.imp().update()));
            player
                .playlist()
                .connect_items_changed(clone!(@weak obj => move |_, _, _, _| obj.imp().update()));

            player.connect_current_time_ms_notify(clone!(@weak obj => move |player| {
                let imp = obj.imp();
                imp.current_time_label.set_label(&format_time(player.current_time_ms()));
                imp.remaining_time_label.set_label(&format_time(player.duration_ms() - player.current_time_ms()));
            }));

            player.connect_duration_ms_notify(clone!(@weak obj => move |player| {
                let imp = obj.imp();
                imp.current_time_label.set_label(&format_time(player.current_time_ms()));
                imp.remaining_time_label.set_label(&format_time(player.duration_ms() - player.current_time_ms()));
            }));

            player
                .bind_property("position", &self.slider.adjustment(), "value")
                .sync_create()
                .bidirectional()
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

    pub fn connect_show_playlist<F: Fn(&Self, bool) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
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
    fn previous(&self, _: &gtk::Button) {
        self.player().previous();
    }

    #[template_callback]
    fn show_playlist(&self, button: &gtk::ToggleButton) {
        self.emit_by_name::<()>("show-playlist", &[&button.is_active()]);
    }

    #[template_callback]
    fn next(&self, _: &gtk::Button) {
        self.player().next();
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

fn format_time(time_ms: u64) -> String {
    let s = time_ms / 1000;
    let (m, s) = (s / 60, s % 60);
    let (h, m) = (m / 60, m % 60);

    if h > 0 {
        format!("{h:0>2}:{m:0>2}:{s:0>2}")
    } else {
        format!("{m:0>2}:{s:0>2}")
    }
}
