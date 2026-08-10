use std::cell::OnceCell;

use gtk::{
    glib::{self, subclass::Signal, Properties},
    prelude::*,
    subclass::prelude::*,
};
use once_cell::sync::Lazy;

use super::{
    item_row_child,
    kind::{
        EnsembleKind, InstrumentKind, KindSource, PersonKind, RoleKind, SelectorKind,
        SelectorSource,
    },
};
use crate::{library::Library, util, util::activatable_row::ActivatableRow};

mod imp {
    use super::*;

    #[derive(Default, gtk::CompositeTemplate, Properties)]
    #[properties(wrapper_type = super::SelectorPopover)]
    #[template(file = "data/ui/selector/popover.blp")]
    pub struct SelectorPopover {
        #[property(get, construct_only)]
        pub library: OnceCell<Library>,

        pub source: OnceCell<Box<dyn SelectorSource>>,

        #[template_child]
        pub search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub reset_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub scrolled_window: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub list_box: TemplateChild<gtk::ListBox>,
    }

    impl std::fmt::Debug for SelectorPopover {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("SelectorPopover").finish_non_exhaustive()
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SelectorPopover {
        const NAME: &'static str = "MusicusSelectorPopover";
        type Type = super::SelectorPopover;
        type ParentType = gtk::Popover;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for SelectorPopover {
        fn constructed(&self) {
            self.parent_constructed();

            self.obj().connect_visible_notify(|obj| {
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
                    Signal::builder("selected")
                        .param_types([glib::BoxedAnyObject::static_type()])
                        .build(),
                    Signal::builder("create").build(),
                    Signal::builder("reset").build(),
                ]
            });

            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for SelectorPopover {
        // TODO: Fix focus.
        fn focus(&self, direction_type: gtk::DirectionType) -> bool {
            if direction_type == gtk::DirectionType::Down {
                self.list_box.child_focus(direction_type)
            } else {
                self.parent_focus(direction_type)
            }
        }
    }

    impl PopoverImpl for SelectorPopover {}
}

glib::wrapper! {
    pub struct SelectorPopover(ObjectSubclass<imp::SelectorPopover>)
        @extends gtk::Popover, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native, gtk::ShortcutManager;
}

#[gtk::template_callbacks]
impl SelectorPopover {
    /// A popover selecting items of kind `K`.
    pub fn new<K: SelectorKind>(library: &Library) -> Self {
        let obj: Self = glib::Object::builder().property("library", library).build();
        let imp = obj.imp();

        let source: Box<dyn SelectorSource> = Box::new(KindSource::<K>::default());

        imp.search_entry
            .set_placeholder_text(Some(&source.search_placeholder()));

        match source.reset_tooltip() {
            Some(tooltip) => {
                imp.reset_button.set_tooltip_text(Some(&tooltip));
                imp.reset_button.set_visible(true);
            }
            None => imp.reset_button.set_visible(false),
        }

        if imp.source.set(source).is_err() {
            log::error!("Selector popover was initialized twice");
        }

        obj.search("");

        obj
    }

    pub fn persons(library: &Library) -> Self {
        Self::new::<PersonKind>(library)
    }

    pub fn ensembles(library: &Library) -> Self {
        Self::new::<EnsembleKind>(library)
    }

    pub fn instruments(library: &Library) -> Self {
        Self::new::<InstrumentKind>(library)
    }

    pub fn roles(library: &Library) -> Self {
        Self::new::<RoleKind>(library)
    }

    pub fn connect_selected<T: Clone + 'static, F: Fn(&Self, T) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("selected", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let item = values[1]
                .get::<glib::BoxedAnyObject>()
                .unwrap()
                .borrow::<T>()
                .clone();
            f(&obj, item);
            None
        })
    }

    pub fn connect_create<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("create", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            f(&obj);
            None
        })
    }

    pub fn connect_reset<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("reset", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            f(&obj);
            None
        })
    }

    #[template_callback]
    fn search_changed(&self, entry: &gtk::SearchEntry) {
        self.search(&entry.text());
    }

    #[template_callback]
    fn activate(&self, _: &gtk::SearchEntry) {
        if self.source().is_empty() {
            self.create();
        } else {
            self.select(0);
        }
    }

    #[template_callback]
    fn stop_search(&self, _: &gtk::SearchEntry) {
        self.popdown();
    }

    #[template_callback]
    fn reset_button_clicked(&self) {
        self.emit_by_name::<()>("reset", &[]);
        self.popdown();
    }

    fn source(&self) -> &dyn SelectorSource {
        &**self
            .imp()
            .source
            .get()
            .expect("selector popover should have been initialized with a source")
    }

    fn search(&self, search: &str) {
        let imp = self.imp();

        let rows = match self.source().search(&self.library(), search) {
            Ok(rows) => rows,
            Err(err) => {
                log::error!("Search failed: {err:?}");
                Vec::new()
            }
        };

        imp.list_box.remove_all();

        for (index, row) in rows.iter().enumerate() {
            let activatable = ActivatableRow::new(&item_row_child(&row.text, row.in_library));

            let obj = self.clone();
            activatable.connect_activated(move |_: &ActivatableRow| {
                obj.select(index);
            });

            imp.list_box.append(&activatable);
        }

        let create_box = gtk::Box::builder().spacing(12).build();
        create_box.append(&gtk::Image::builder().icon_name("list-add-symbolic").build());
        create_box.append(
            &gtk::Label::builder()
                .label(self.source().create_label())
                .halign(gtk::Align::Start)
                .build(),
        );

        let create_row = ActivatableRow::new(&create_box);
        let obj = self.clone();
        create_row.connect_activated(move |_: &ActivatableRow| {
            obj.create();
        });

        imp.list_box.append(&create_row);
    }

    fn select(&self, index: usize) {
        match self.source().select(&self.library(), index) {
            Ok(item) => {
                self.emit_by_name::<()>("selected", &[&item]);
                self.popdown();
            }
            Err(err) => {
                log::error!("Failed to import the selected item: {err:?}");

                if let Some(toast_overlay) = util::find_toast_overlay(self) {
                    util::error_toast(
                        "Failed to add this item to the library",
                        err,
                        &toast_overlay,
                    );
                }

                self.popdown();
            }
        }
    }

    fn create(&self) {
        self.emit_by_name::<()>("create", &[]);
        self.popdown();
    }
}
