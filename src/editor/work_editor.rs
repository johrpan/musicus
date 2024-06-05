use crate::{
    db::{
        self,
        models::{Composer, Instrument, Person, Work, WorkPart},
    },
    editor::{
        instrument_selector_popover::MusicusInstrumentSelectorPopover,
        person_selector_popover::MusicusPersonSelectorPopover,
        translation_editor::MusicusTranslationEditor,
        work_editor_composer_row::MusicusWorkEditorComposerRow,
    },
    library::MusicusLibrary,
};

use adw::{prelude::*, subclass::prelude::*};
use gettextrs::gettext;
use gtk::glib::{self, clone, Properties};

use std::cell::{OnceCell, RefCell};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, Properties)]
    #[properties(wrapper_type = super::MusicusWorkEditor)]
    #[template(file = "data/ui/work_editor.blp")]
    pub struct MusicusWorkEditor {
        #[property(get, construct_only)]
        pub navigation: OnceCell<adw::NavigationView>,

        #[property(get, construct_only)]
        pub library: OnceCell<MusicusLibrary>,

        // Holding a reference to each composer row is the simplest way to enumerate all
        // results when finishing the process of editing the work. The composer rows
        // handle all state related to the composer.
        pub composer_rows: RefCell<Vec<MusicusWorkEditorComposerRow>>,
        pub parts: RefCell<Vec<WorkPart>>,
        pub instruments: RefCell<Vec<Instrument>>,

        pub persons_popover: OnceCell<MusicusPersonSelectorPopover>,
        pub instruments_popover: OnceCell<MusicusInstrumentSelectorPopover>,

        #[template_child]
        pub name_editor: TemplateChild<MusicusTranslationEditor>,
        #[template_child]
        pub composer_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub select_person_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub part_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub instrument_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub select_instrument_box: TemplateChild<gtk::Box>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MusicusWorkEditor {
        const NAME: &'static str = "MusicusWorkEditor";
        type Type = super::MusicusWorkEditor;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            MusicusTranslationEditor::static_type();
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for MusicusWorkEditor {
        fn constructed(&self) {
            self.parent_constructed();

            let persons_popover = MusicusPersonSelectorPopover::new(self.library.get().unwrap());

            let obj = self.obj().clone();
            persons_popover.connect_person_selected(
                move |_: &MusicusPersonSelectorPopover, person: Person| {
                    let role = obj.library().composer_default_role().unwrap();
                    let composer = Composer { person, role };
                    let row = MusicusWorkEditorComposerRow::new(&obj.library(), composer);

                    row.connect_remove(clone!(@weak obj => move |row| {
                        obj.imp().composer_list.remove(row);
                        obj.imp().composer_rows.borrow_mut().retain(|c| c != row);
                    }));

                    obj.imp()
                        .composer_list
                        .insert(&row, obj.imp().composer_rows.borrow().len() as i32);

                    obj.imp().composer_rows.borrow_mut().push(row);
                },
            );

            self.select_person_box.append(&persons_popover);
            self.persons_popover.set(persons_popover).unwrap();

            let instruments_popover =
                MusicusInstrumentSelectorPopover::new(self.library.get().unwrap());

            let obj = self.obj().clone();
            instruments_popover.connect_instrument_selected(
                move |_: &MusicusInstrumentSelectorPopover, instrument: Instrument| {
                    let row = adw::ActionRow::builder()
                        .title(instrument.to_string())
                        .build();

                    let remove_button = gtk::Button::builder()
                        .icon_name("user-trash-symbolic")
                        .valign(gtk::Align::Center)
                        .css_classes(["flat"])
                        .build();

                    remove_button.connect_clicked(
                        clone!(@weak obj, @weak row, @strong instrument => move |_| {
                            obj.imp().instrument_list.remove(&row);
                            obj.imp().instruments.borrow_mut().retain(|i| *i != instrument);
                        }),
                    );

                    row.add_suffix(&remove_button);

                    obj.imp()
                        .instrument_list
                        .insert(&row, obj.imp().instruments.borrow().len() as i32);

                    obj.imp().instruments.borrow_mut().push(instrument);
                },
            );

            self.select_instrument_box.append(&instruments_popover);
            self.instruments_popover.set(instruments_popover).unwrap();
        }
    }

    impl WidgetImpl for MusicusWorkEditor {}
    impl NavigationPageImpl for MusicusWorkEditor {}
}

glib::wrapper! {
    pub struct MusicusWorkEditor(ObjectSubclass<imp::MusicusWorkEditor>)
        @extends gtk::Widget, adw::NavigationPage;
}

#[gtk::template_callbacks]
impl MusicusWorkEditor {
    pub fn new(
        navigation: &adw::NavigationView,
        library: &MusicusLibrary,
        work: Option<&Work>,
    ) -> Self {
        let obj: Self = glib::Object::builder()
            .property("navigation", navigation)
            .property("library", library)
            .build();

        if let Some(_work) = work {
            // TODO: Initialize work data.
        }

        obj
    }

    #[template_callback]
    fn add_person(&self, _: &adw::ActionRow) {
        self.imp().persons_popover.get().unwrap().popup();
    }

    #[template_callback]
    fn add_part(&self, _: &adw::ActionRow) {
        let part = WorkPart {
            work_id: db::generate_id(),
            ..Default::default()
        };

        let row = adw::EntryRow::builder().title(gettext("Name")).build();

        let remove_button = gtk::Button::builder()
            .icon_name("user-trash-symbolic")
            .valign(gtk::Align::Center)
            .css_classes(["flat"])
            .build();

        remove_button.connect_clicked(
            clone!(@weak self as obj, @weak row, @strong part => move |_| {
                obj.imp().part_list.remove(&row);
                obj.imp().parts.borrow_mut().retain(|p| *p != part);
            }),
        );

        row.add_suffix(&remove_button);

        self.imp()
            .part_list
            .insert(&row, self.imp().parts.borrow().len() as i32);

        row.grab_focus();

        self.imp().parts.borrow_mut().push(part);
    }

    #[template_callback]
    fn add_instrument(&self, _: &adw::ActionRow) {
        self.imp().instruments_popover.get().unwrap().popup();
    }
}
