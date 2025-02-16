use crate::{db::models::Work, editor::work_editor::MusicusWorkEditor, library::MusicusLibrary};

use adw::{prelude::*, subclass::prelude::*};
use gtk::glib::{self, clone, subclass::Signal, Properties};
use once_cell::sync::Lazy;

use std::cell::{OnceCell, RefCell};

mod imp {

    use super::*;

    #[derive(Properties, Debug, Default, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::MusicusWorkEditorPartRow)]
    #[template(file = "data/ui/work_editor_part_row.blp")]
    pub struct MusicusWorkEditorPartRow {
        #[property(get, construct_only)]
        pub navigation: OnceCell<adw::NavigationView>,

        #[property(get, construct_only)]
        pub library: OnceCell<MusicusLibrary>,

        pub part: RefCell<Option<Work>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MusicusWorkEditorPartRow {
        const NAME: &'static str = "MusicusWorkEditorPartRow";
        type Type = super::MusicusWorkEditorPartRow;
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
    impl ObjectImpl for MusicusWorkEditorPartRow {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> =
                Lazy::new(|| vec![Signal::builder("remove").build()]);

            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for MusicusWorkEditorPartRow {}
    impl ListBoxRowImpl for MusicusWorkEditorPartRow {}
    impl PreferencesRowImpl for MusicusWorkEditorPartRow {}
    impl ActionRowImpl for MusicusWorkEditorPartRow {}
}

glib::wrapper! {
    pub struct MusicusWorkEditorPartRow(ObjectSubclass<imp::MusicusWorkEditorPartRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::ActionRow;
}

#[gtk::template_callbacks]
impl MusicusWorkEditorPartRow {
    pub fn new(navigation: &adw::NavigationView, library: &MusicusLibrary, part: Work) -> Self {
        let obj: Self = glib::Object::builder()
            .property("navigation", navigation)
            .property("library", library)
            .build();
        obj.set_part(part);
        obj
    }

    pub fn connect_remove<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("remove", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            f(&obj);
            None
        })
    }

    pub fn part(&self) -> Work {
        self.imp().part.borrow().to_owned().unwrap()
    }

    fn set_part(&self, part: Work) {
        self.set_title(&part.name.get());

        if !part.parts.is_empty() {
            self.set_subtitle(
                &part
                    .parts
                    .iter()
                    .map(|p| p.name.get())
                    .collect::<Vec<&str>>()
                    .join("\n"),
            );
        } else {
            self.set_subtitle("");
        }

        self.imp().part.replace(Some(part));
    }

    #[template_callback]
    fn edit(&self) {
        let editor = MusicusWorkEditor::new(
            &self.navigation(),
            &self.library(),
            self.imp().part.borrow().as_ref(),
            true,
        );

        editor.connect_created(clone!(
            #[weak(rename_to = this)]
            self,
            move |_, part| {
                this.set_part(part);
            }
        ));

        self.navigation().push(&editor);
    }

    #[template_callback]
    fn remove(&self) {
        self.emit_by_name::<()>("remove", &[]);
    }
}
