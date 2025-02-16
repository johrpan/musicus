use super::activatable_row::MusicusActivatableRow;
use crate::db::models::Work;

use gtk::{
    glib::{self, subclass::Signal},
    prelude::*,
    subclass::prelude::*,
};
use once_cell::sync::Lazy;

use std::cell::{OnceCell, RefCell};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/tracks_editor_parts_popover.blp")]
    pub struct TracksEditorPartsPopover {
        pub parts: OnceCell<Vec<Work>>,
        pub parts_filtered: RefCell<Vec<Work>>,

        #[template_child]
        pub search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub scrolled_window: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub list_box: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TracksEditorPartsPopover {
        const NAME: &'static str = "MusicusTracksEditorPartsPopover";
        type Type = super::TracksEditorPartsPopover;
        type ParentType = gtk::Popover;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for TracksEditorPartsPopover {
        fn constructed(&self) {
            self.parent_constructed();

            self.obj()
                .connect_visible_notify(|obj: &super::TracksEditorPartsPopover| {
                    if obj.is_visible() {
                        obj.imp().search_entry.set_text("");
                        obj.imp().search_entry.grab_focus();
                        obj.imp().scrolled_window.vadjustment().set_value(0.0);
                    }
                });
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("part-selected")
                        .param_types([Work::static_type()])
                        .build(),
                    Signal::builder("create").build(),
                ]
            });

            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for TracksEditorPartsPopover {
        // TODO: Fix focus.
        fn focus(&self, direction_type: gtk::DirectionType) -> bool {
            if direction_type == gtk::DirectionType::Down {
                self.list_box.child_focus(direction_type)
            } else {
                self.parent_focus(direction_type)
            }
        }
    }

    impl PopoverImpl for TracksEditorPartsPopover {}
}

glib::wrapper! {
    pub struct TracksEditorPartsPopover(ObjectSubclass<imp::TracksEditorPartsPopover>)
        @extends gtk::Widget, gtk::Popover;
}

#[gtk::template_callbacks]
impl TracksEditorPartsPopover {
    pub fn new(parts: Vec<Work>) -> Self {
        let obj: Self = glib::Object::new();
        obj.imp().parts.set(parts).unwrap();
        obj.search("");
        obj
    }

    pub fn connect_part_selected<F: Fn(&Self, Work) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("part-selected", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let role = values[1].get::<Work>().unwrap();
            f(&obj, role);
            None
        })
    }

    #[template_callback]
    fn search_changed(&self, entry: &gtk::SearchEntry) {
        self.search(&entry.text());
    }

    #[template_callback]
    fn activate(&self, _: &gtk::SearchEntry) {
        if let Some(work) = self.imp().parts_filtered.borrow().first() {
            self.select(work.clone());
        }
    }

    #[template_callback]
    fn stop_search(&self, _: &gtk::SearchEntry) {
        self.popdown();
    }

    fn search(&self, search: &str) {
        let imp = self.imp();

        let parts_filtered = imp
            .parts
            .get()
            .unwrap()
            .iter()
            .filter(|p| p.name.get().to_lowercase().contains(&search.to_lowercase()))
            .cloned()
            .collect::<Vec<Work>>();

        imp.list_box.remove_all();

        for part in &parts_filtered {
            let row = MusicusActivatableRow::new(
                &gtk::Label::builder()
                    .label(part.to_string())
                    .halign(gtk::Align::Start)
                    .build(),
            );

            row.set_tooltip_text(Some(&part.to_string()));

            let part = part.clone();
            let obj = self.clone();
            row.connect_activated(move |_: &MusicusActivatableRow| {
                obj.select(part.clone());
            });

            imp.list_box.append(&row);
        }

        imp.parts_filtered.replace(parts_filtered);
    }

    fn select(&self, part: Work) {
        self.emit_by_name::<()>("part-selected", &[&part]);
        self.popdown();
    }
}
