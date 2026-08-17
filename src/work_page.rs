use std::cell::OnceCell;

use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::{
    gio,
    glib::{self, Properties},
    prelude::*,
};

use musicus_library::{db::models::*, format_translated};

use crate::{
    editor::work::WorkEditor,
    facet_tile::FacetTile,
    library::{Facet, Library, LibraryQuery},
    player::Player,
    program::Program,
    recording_tile::RecordingTile,
    util,
};

mod imp {
    use super::*;

    #[derive(Properties, Debug, Default, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::WorkPage)]
    #[template(file = "data/ui/work_page.blp")]
    pub struct WorkPage {
        #[property(get, construct_only)]
        pub toast_overlay: OnceCell<adw::ToastOverlay>,

        #[property(get, construct_only)]
        pub navigation: OnceCell<adw::NavigationView>,

        #[property(get, construct_only)]
        pub library: OnceCell<Library>,

        #[property(get, construct_only)]
        pub player: OnceCell<Player>,

        pub query: OnceCell<LibraryQuery>,
        pub work: OnceCell<Work>,

        #[template_child]
        pub title_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub subtitle_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub structure_heading: TemplateChild<gtk::Label>,
        #[template_child]
        pub structure_flow_box: TemplateChild<gtk::FlowBox>,
        #[template_child]
        pub related_works_flow_box: TemplateChild<gtk::FlowBox>,
        #[template_child]
        pub recordings_flow_box: TemplateChild<gtk::FlowBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for WorkPage {
        const NAME: &'static str = "MusicusWorkPage";
        type Type = super::WorkPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for WorkPage {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj().to_owned();
            let add_to_playlist_action = gio::ActionEntry::builder("add-to-playlist")
                .activate(move |_, _, _| {
                    let program = Program::from_query(obj.imp().query.get().unwrap().clone());
                    obj.player().set_program(program);
                })
                .build();

            let obj = self.obj().to_owned();
            let edit_action = gio::ActionEntry::builder("edit")
                .activate(move |_, _, _| {
                    obj.navigation().push(&WorkEditor::new(
                        &obj.navigation(),
                        &obj.library(),
                        Some(obj.imp().work.get().unwrap()),
                        false,
                    ));
                })
                .build();

            let obj = self.obj().to_owned();
            let delete_action = gio::ActionEntry::builder("delete")
                .activate(move |_, _, _| {
                    if let Err(err) = obj
                        .library()
                        .delete_work(&obj.imp().work.get().unwrap().work_id)
                    {
                        util::error_toast("Failed to delete work", err, &obj.toast_overlay());
                    }
                })
                .build();

            let actions = gio::SimpleActionGroup::new();
            actions.add_action_entries([add_to_playlist_action, edit_action, delete_action]);
            self.obj().insert_action_group("work", Some(&actions));
        }
    }

    impl WidgetImpl for WorkPage {}
    impl NavigationPageImpl for WorkPage {}
}

glib::wrapper! {
    pub struct WorkPage(ObjectSubclass<imp::WorkPage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

#[gtk::template_callbacks]
impl WorkPage {
    /// `query.work` must be `Some`; the rest of `query` (e.g. an active tag filter)
    /// carries over from wherever this page was reached from, scoping the shown
    /// recordings the same way the search page did before work got its own page.
    pub fn new(
        toast_overlay: &adw::ToastOverlay,
        navigation: &adw::NavigationView,
        library: &Library,
        player: &Player,
        query: LibraryQuery,
    ) -> Self {
        let obj: Self = glib::Object::builder()
            .property("toast-overlay", toast_overlay)
            .property("navigation", navigation)
            .property("library", library)
            .property("player", player)
            .build();

        let work = query.work.clone().expect("WorkPage requires query.work");

        obj.imp().title_label.set_label(work.name.get());
        obj.imp().subtitle_label.set_label(
            &work
                .composers_string()
                .unwrap_or_else(|| gettext("No composers")),
        );

        let results = match library.search(&query, "") {
            Ok(results) => results,
            Err(err) => {
                util::error_toast("Search failed", err, toast_overlay);
                Default::default()
            }
        };

        match &results.parent_work {
            Some(parent) => obj.imp().structure_heading.set_label(&format_translated!(
                gettext("Part of {}"),
                parent.name.get()
            )),
            None => obj.imp().structure_heading.set_label(&gettext("Movements")),
        }

        obj.imp()
            .structure_flow_box
            .set_visible(!results.structure.is_empty());
        for part in &results.structure {
            obj.imp()
                .structure_flow_box
                .append(&FacetTile::new(Facet::Work(part.clone())));
        }

        obj.imp()
            .related_works_flow_box
            .set_visible(!results.works.is_empty());
        for related in &results.works {
            obj.imp()
                .related_works_flow_box
                .append(&FacetTile::new(Facet::Work(related.clone())));
        }

        for recording in &results.recordings {
            obj.imp().recordings_flow_box.append(&RecordingTile::new(
                toast_overlay,
                navigation,
                library,
                player,
                recording,
            ));
        }

        obj.imp().work.set(work).unwrap();
        obj.imp().query.set(query).unwrap();

        obj
    }

    #[template_callback]
    fn play_button_clicked(&self) {
        let program = Program::from_query(self.imp().query.get().unwrap().clone());
        self.player().set_program(program);
        self.player().play_from_program();
    }

    #[template_callback]
    fn work_selected(&self, tile: &gtk::FlowBoxChild) {
        let Facet::Work(work) = tile.downcast_ref::<FacetTile>().unwrap().facet().clone() else {
            return;
        };

        let mut new_query = self.imp().query.get().unwrap().clone();
        new_query.work = Some(work);

        self.navigation().push(&WorkPage::new(
            &self.toast_overlay(),
            &self.navigation(),
            &self.library(),
            &self.player(),
            new_query,
        ));
    }

    #[template_callback]
    fn recording_selected(&self, tile: &gtk::FlowBoxChild) {
        let playlist = self
            .player()
            .recording_to_playlist(tile.downcast_ref::<RecordingTile>().unwrap().recording());
        self.player().append_and_play(playlist);
    }
}
