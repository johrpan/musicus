use super::*;
use crate::backend::Backend;
use crate::database::*;
use crate::widgets::*;
use gio::prelude::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use libhandy::prelude::*;
use libhandy::HeaderBarExt;
use std::cell::Cell;
use std::convert::TryInto;
use std::rc::Rc;

enum WorkSelectorState {
    Loading,
    Persons(Vec<Person>),
    PersonLoading(Person),
    Person(Vec<WorkDescription>),
}

pub struct WorkSelector<F>
where
    F: Fn(WorkDescription) -> () + 'static,
{
    window: libhandy::Window,
    backend: Rc<Backend>,
    callback: F,
    leaflet: libhandy::Leaflet,
    sidebar_stack: gtk::Stack,
    person_search_entry: gtk::SearchEntry,
    person_list: gtk::ListBox,
    stack: gtk::Stack,
    header: libhandy::HeaderBar,
    search_entry: gtk::SearchEntry,
    content_stack: gtk::Stack,
    work_list: gtk::ListBox,
    person_list_row_activated_handler_id: Cell<Option<glib::SignalHandlerId>>,
    work_list_row_activated_handler_id: Cell<Option<glib::SignalHandlerId>>,
}

impl<F> WorkSelector<F>
where
    F: Fn(WorkDescription) -> () + 'static,
{
    pub fn new<P: IsA<gtk::Window>>(backend: Rc<Backend>, parent: &P, callback: F) -> Rc<Self> {
        use WorkSelectorState::*;

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus_editor/ui/work_selector.ui");

        get_widget!(builder, libhandy::Window, window);
        get_widget!(builder, libhandy::Leaflet, leaflet);
        get_widget!(builder, gtk::Button, add_button);
        get_widget!(builder, gtk::SearchEntry, person_search_entry);
        get_widget!(builder, gtk::Stack, sidebar_stack);
        get_widget!(builder, gtk::ListBox, person_list);
        get_widget!(builder, gtk::Stack, stack);
        get_widget!(builder, libhandy::HeaderBar, header);
        get_widget!(builder, gtk::SearchEntry, search_entry);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, gtk::Stack, content_stack);
        get_widget!(builder, gtk::ListBox, work_list);

        let result = Rc::new(WorkSelector {
            window: window,
            backend: backend,
            callback: callback,
            leaflet: leaflet,
            sidebar_stack: sidebar_stack,
            person_list: person_list,
            person_search_entry: person_search_entry,
            stack: stack,
            header: header,
            search_entry: search_entry,
            content_stack: content_stack,
            work_list: work_list,
            person_list_row_activated_handler_id: Cell::new(None),
            work_list_row_activated_handler_id: Cell::new(None),
        });

        add_button.connect_clicked(clone!(@strong result => move |_| {
            let editor = WorkEditor::new(
                result.backend.clone(),
                &result.window,
                None,
                clone!(@strong result => move |work| {
                    result.window.close();
                    (result.callback)(work);
                }),
            );

            editor.show();
        }));

        back_button.connect_clicked(clone!(@strong result => move |_| {
            result.back();
        }));

        result
            .person_search_entry
            .connect_search_changed(clone!(@strong result => move |_| {
                result.person_list.invalidate_filter();
            }));

        result
            .search_entry
            .connect_search_changed(clone!(@strong result => move |_| {
                result.work_list.invalidate_filter();
            }));

        result.window.set_transient_for(Some(parent));
        result.clone().set_state(Loading);

        result
    }

    pub fn show(&self) {
        self.window.show();
    }

    fn set_state(self: Rc<Self>, state: WorkSelectorState) {
        use WorkSelectorState::*;

        match state {
            Loading => {
                let c = glib::MainContext::default();
                let clone = self.clone();
                c.spawn_local(async move {
                    let persons = clone.backend.get_persons().await.unwrap();
                    clone.clone().set_state(Persons(persons));
                });

                self.sidebar_stack.set_visible_child_name("loading");
                self.stack.set_visible_child_name("empty_screen");
                self.leaflet.set_visible_child_name("sidebar");
            }
            Persons(persons) => {
                for child in self.person_list.get_children() {
                    self.person_list.remove(&child);
                }

                for (index, person) in persons.iter().enumerate() {
                    let label = gtk::Label::new(Some(&person.name_lf()));
                    label.set_halign(gtk::Align::Start);
                    let row = SelectorRow::new(index.try_into().unwrap(), &label);
                    row.show_all();
                    self.person_list.insert(&row, -1);
                }

                match self.person_list_row_activated_handler_id.take() {
                    Some(id) => self.person_list.disconnect(id),
                    None => (),
                }

                let handler_id = self.person_list.connect_row_activated(
                    clone!(@strong self as self_, @strong persons => move |_, row| {
                        let row = row.get_child().unwrap().downcast::<SelectorRow>().unwrap();
                        let index: usize = row.get_index().try_into().unwrap();
                        let person = persons[index].clone();
                        self_.clone().set_state(PersonLoading(person));
                    }),
                );

                self.person_list_row_activated_handler_id
                    .set(Some(handler_id));

                self.person_list.set_filter_func(Some(Box::new(
                    clone!(@strong self as self_, @strong persons => move |row| {
                        let row = row.get_child().unwrap().downcast::<SelectorRow>().unwrap();
                        let index: usize = row.get_index().try_into().unwrap();
                        let search = self_.person_search_entry.get_text().to_string().to_lowercase();

                        search.is_empty() || persons[index]
                            .name_lf()
                            .to_lowercase()
                            .contains(&search)
                    }),
                )));

                self.sidebar_stack.set_visible_child_name("persons_list");
                self.stack.set_visible_child_name("empty_screen");
                self.leaflet.set_visible_child_name("sidebar");
            }
            PersonLoading(person) => {
                self.header.set_title(Some(&person.name_fl()));

                let c = glib::MainContext::default();
                let clone = self.clone();
                c.spawn_local(async move {
                    let works = clone
                        .backend
                        .get_work_descriptions(person.id)
                        .await
                        .unwrap();
                    clone.clone().set_state(Person(works));
                });

                self.content_stack.set_visible_child_name("loading");
                self.stack.set_visible_child_name("person_screen");
                self.leaflet.set_visible_child_name("content");
            }
            Person(works) => {
                for child in self.work_list.get_children() {
                    self.work_list.remove(&child);
                }

                for (index, work) in works.iter().enumerate() {
                    let label = gtk::Label::new(Some(&work.title));
                    label.set_halign(gtk::Align::Start);
                    let row = SelectorRow::new(index.try_into().unwrap(), &label);
                    row.show_all();
                    self.work_list.insert(&row, -1);
                }

                match self.work_list_row_activated_handler_id.take() {
                    Some(id) => self.work_list.disconnect(id),
                    None => (),
                }

                let handler_id = self.work_list.connect_row_activated(
                    clone!(@strong self as self_, @strong works => move |_, row| {
                        let row = row.get_child().unwrap().downcast::<SelectorRow>().unwrap();
                        let index: usize = row.get_index().try_into().unwrap();
                        let work = works[index].clone();
                        (self_.callback)(work);
                        self_.window.close();
                    }),
                );

                self.work_list_row_activated_handler_id
                    .set(Some(handler_id));

                self.work_list.set_filter_func(Some(Box::new(
                    clone!(@strong self as self_, @strong works => move |row| {
                        let row = row.get_child().unwrap().downcast::<SelectorRow>().unwrap();
                        let index: usize = row.get_index().try_into().unwrap();
                        let search = self_.search_entry.get_text().to_string().to_lowercase();

                        search.is_empty() || works[index]
                            .title
                            .to_lowercase()
                            .contains(&search)
                    }),
                )));

                self.content_stack.set_visible_child_name("content");
                self.stack.set_visible_child_name("person_screen");
                self.leaflet.set_visible_child_name("content");
            }
        }
    }

    fn back(&self) {
        self.stack.set_visible_child_name("empty_screen");
        self.leaflet.set_visible_child_name("sidebar");
    }
}
