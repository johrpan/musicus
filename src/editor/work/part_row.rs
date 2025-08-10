use std::cell::{OnceCell, RefCell};

use adw::{prelude::*, subclass::prelude::*};
use gtk::{
    gdk,
    glib::{self, clone, subclass::Signal, Properties},
};
use once_cell::sync::Lazy;

use crate::{
    db::models::Work, editor::work::WorkEditor, library::Library, util::drag_widget::DragWidget,
};

mod imp {
    use super::*;

    #[derive(Properties, Debug, Default, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::WorkEditorPartRow)]
    #[template(file = "data/ui/editor/work/part_row.blp")]
    pub struct WorkEditorPartRow {
        #[property(get, construct_only)]
        pub navigation: OnceCell<adw::NavigationView>,

        #[property(get, construct_only)]
        pub library: OnceCell<Library>,

        pub part: RefCell<Option<Work>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for WorkEditorPartRow {
        const NAME: &'static str = "MusicusWorkEditorPartRow";
        type Type = super::WorkEditorPartRow;
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
    impl ObjectImpl for WorkEditorPartRow {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("remove").build(),
                    Signal::builder("move")
                        .param_types([super::WorkEditorPartRow::static_type()])
                        .build(),
                ]
            });

            SIGNALS.as_ref()
        }

        fn constructed(&self) {
            self.parent_constructed();

            let drag_source = gtk::DragSource::builder()
                .actions(gdk::DragAction::MOVE)
                .content(&gdk::ContentProvider::for_value(&self.obj().to_value()))
                .build();

            drag_source.connect_drag_begin(clone!(
                #[weak(rename_to = obj)]
                self.obj(),
                move |_, drag| {
                    let icon = gtk::DragIcon::for_drag(drag);
                    icon.set_child(Some(&DragWidget::new(&obj)));
                }
            ));

            self.obj().add_controller(drag_source);

            let drop_target = gtk::DropTarget::builder()
                .actions(gdk::DragAction::MOVE)
                .build();
            drop_target.set_types(&[Self::Type::static_type()]);

            drop_target.connect_drop(clone!(
                #[weak(rename_to = obj)]
                self.obj(),
                #[upgrade_or]
                false,
                move |_, value, _, _| {
                    if let Ok(row) = value.get::<Self::Type>() {
                        obj.emit_by_name::<()>("move", &[&row]);
                        true
                    } else {
                        false
                    }
                }
            ));

            self.obj().add_controller(drop_target);
        }
    }

    impl WidgetImpl for WorkEditorPartRow {}
    impl ListBoxRowImpl for WorkEditorPartRow {}
    impl PreferencesRowImpl for WorkEditorPartRow {}
    impl ActionRowImpl for WorkEditorPartRow {}
}

glib::wrapper! {
    pub struct WorkEditorPartRow(ObjectSubclass<imp::WorkEditorPartRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::ActionRow;
}

#[gtk::template_callbacks]
impl WorkEditorPartRow {
    pub fn new(navigation: &adw::NavigationView, library: &Library, part: Work) -> Self {
        let obj: Self = glib::Object::builder()
            .property("navigation", navigation)
            .property("library", library)
            .build();
        obj.set_part(part);
        obj
    }

    pub fn connect_move<F: Fn(&Self, Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("move", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let source = values[1].get::<Self>().unwrap();
            f(&obj, source);
            None
        })
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
        self.set_title(part.name.get());

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
        let editor = WorkEditor::new(
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
