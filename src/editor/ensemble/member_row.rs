use std::cell::{OnceCell, RefCell};

use adw::{prelude::*, subclass::prelude::*};
use gettextrs::gettext;
use gtk::{
    gdk,
    glib::{self, clone, subclass::Signal, Properties},
};
use once_cell::sync::Lazy;

use musicus_library::db::models::{Instrument, Person};

use crate::{
    editor::simple_entity::SimpleEntityEditor, library::Library, selector::SelectorPopover,
    util::drag_widget::DragWidget,
};

mod imp {
    use super::*;

    #[derive(Properties, Debug, Default, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::EnsembleEditorMemberRow)]
    #[template(file = "data/ui/editor/ensemble/member_row.blp")]
    pub struct EnsembleEditorMemberRow {
        #[property(get, construct_only)]
        pub navigation: OnceCell<adw::NavigationView>,

        #[property(get, construct_only)]
        pub library: OnceCell<Library>,

        pub person: RefCell<Option<Person>>,
        pub instrument: RefCell<Option<Instrument>>,
        pub instrument_popover: OnceCell<SelectorPopover>,

        #[template_child]
        pub instrument_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub instrument_box: TemplateChild<gtk::Box>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EnsembleEditorMemberRow {
        const NAME: &'static str = "MusicusEnsembleEditorMemberRow";
        type Type = super::EnsembleEditorMemberRow;
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
    impl ObjectImpl for EnsembleEditorMemberRow {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("remove").build(),
                    Signal::builder("move")
                        .param_types([super::EnsembleEditorMemberRow::static_type()])
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

            let instrument_popover = SelectorPopover::instruments(self.library.get().unwrap());

            let obj = self.obj().to_owned();
            instrument_popover.connect_selected(move |_, instrument: Instrument| {
                obj.set_instrument(instrument);
            });

            let obj = self.obj().to_owned();
            instrument_popover.connect_create(move |_| {
                let editor =
                    SimpleEntityEditor::instrument(&obj.navigation(), &obj.library(), None);

                editor.connect_created(clone!(
                    #[weak]
                    obj,
                    move |_, instrument| {
                        obj.set_instrument(instrument);
                    }
                ));

                obj.navigation().push(&editor);
            });

            self.instrument_box.append(&instrument_popover);
            self.instrument_popover.set(instrument_popover).unwrap();
        }
    }

    impl WidgetImpl for EnsembleEditorMemberRow {}
    impl ListBoxRowImpl for EnsembleEditorMemberRow {}
    impl PreferencesRowImpl for EnsembleEditorMemberRow {}
    impl ActionRowImpl for EnsembleEditorMemberRow {}
}

glib::wrapper! {
    pub struct EnsembleEditorMemberRow(ObjectSubclass<imp::EnsembleEditorMemberRow>)
        @extends adw::ActionRow, adw::PreferencesRow, gtk::ListBoxRow, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Actionable;
}

#[gtk::template_callbacks]
impl EnsembleEditorMemberRow {
    pub fn new(
        navigation: &adw::NavigationView,
        library: &Library,
        person: Person,
        instrument: Option<Instrument>,
    ) -> Self {
        let obj: Self = glib::Object::builder()
            .property("navigation", navigation)
            .property("library", library)
            .build();

        obj.set_title(&person.name.get());
        obj.imp().person.replace(Some(person));

        match instrument {
            Some(instrument) => obj.set_instrument(instrument),
            None => obj
                .imp()
                .instrument_label
                .set_label(&gettext("Select instrument")),
        }

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

    pub fn member(&self) -> (Person, Option<Instrument>) {
        let person = self.imp().person.borrow().clone().unwrap();
        let instrument = self.imp().instrument.borrow().clone();
        (person, instrument)
    }

    fn set_instrument(&self, instrument: Instrument) {
        self.imp()
            .instrument_label
            .set_label(&instrument.to_string());
        self.imp().instrument.replace(Some(instrument));
    }

    #[template_callback]
    fn open_instrument_popover(&self) {
        self.imp().instrument_popover.get().unwrap().popup();
    }

    #[template_callback]
    fn remove(&self) {
        self.emit_by_name::<()>("remove", &[]);
    }
}
