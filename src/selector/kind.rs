use std::cell::RefCell;

use anyhow::Result;
use gettextrs::gettext;
use gtk::glib;
use musicus_library::db::models::{Ensemble, Instrument, Person, Role, Tag};

use crate::library::{Library, SearchItem};

/// One row of a selector's result list.
pub struct SelectorRow {
    pub text: String,
    /// Whether the item is already in the library. Items that are not are shown
    /// with a download icon and imported when selected.
    pub in_library: bool,
}

/// A type of item that can be selected.
pub trait SelectorKind: 'static {
    type Item: Clone + ToString + 'static;

    fn search(library: &Library, search: &str) -> Result<Vec<SearchItem<Self::Item>>>;

    /// Copy an item that only exists in the metadata database into the library.
    fn import(library: &Library, item: &Self::Item) -> Result<Self::Item>;

    fn search_placeholder() -> String;
    fn create_label() -> String;

    /// The label of the reset button, for selectors that have a default value.
    /// `None` hides the button.
    fn reset_tooltip() -> Option<String> {
        None
    }
}

/// The type-erased half of a [`SelectorKind`], so that the popover itself does
/// not have to be generic — generic `GObject` subclasses are painful, and the
/// selected value crosses a glib signal boundary as a `BoxedAnyObject` anyway.
pub trait SelectorSource {
    fn search_placeholder(&self) -> String;
    fn create_label(&self) -> String;
    fn reset_tooltip(&self) -> Option<String>;

    /// Search, remember the results, and return what to display.
    fn search(&self, library: &Library, search: &str) -> Result<Vec<SelectorRow>>;

    /// Whether the last search returned anything.
    fn is_empty(&self) -> bool;

    /// The item at `index` of the last search, importing it into the library
    /// first if it only exists in the metadata database.
    fn select(&self, library: &Library, index: usize) -> Result<glib::BoxedAnyObject>;
}

/// Adapts any [`SelectorKind`] to [`SelectorSource`], holding the results of
/// the most recent search so that a selection can be resolved back to a value.
pub struct KindSource<K: SelectorKind> {
    results: RefCell<Vec<SearchItem<K::Item>>>,
}

impl<K: SelectorKind> Default for KindSource<K> {
    fn default() -> Self {
        Self {
            results: RefCell::new(Vec::new()),
        }
    }
}

impl<K: SelectorKind> SelectorSource for KindSource<K> {
    fn search_placeholder(&self) -> String {
        K::search_placeholder()
    }

    fn create_label(&self) -> String {
        K::create_label()
    }

    fn reset_tooltip(&self) -> Option<String> {
        K::reset_tooltip()
    }

    fn search(&self, library: &Library, search: &str) -> Result<Vec<SelectorRow>> {
        let results = K::search(library, search)?;

        let rows = results
            .iter()
            .map(|result| SelectorRow {
                text: result.item.to_string(),
                in_library: result.in_library,
            })
            .collect();

        self.results.replace(results);

        Ok(rows)
    }

    fn is_empty(&self) -> bool {
        self.results.borrow().is_empty()
    }

    fn select(&self, library: &Library, index: usize) -> Result<glib::BoxedAnyObject> {
        let result = self.results.borrow()[index].clone();

        let item = if result.in_library {
            result.item
        } else {
            K::import(library, &result.item)?
        };

        Ok(glib::BoxedAnyObject::new(item))
    }
}

pub struct PersonKind;

impl SelectorKind for PersonKind {
    type Item = Person;

    fn search(library: &Library, search: &str) -> Result<Vec<SearchItem<Person>>> {
        library.search_persons(search)
    }

    fn import(library: &Library, item: &Person) -> Result<Person> {
        library.import_metadata_person(&item.person_id)
    }

    fn search_placeholder() -> String {
        gettext("Search persons…")
    }

    fn create_label() -> String {
        gettext("Create new person")
    }
}

pub struct EnsembleKind;

impl SelectorKind for EnsembleKind {
    type Item = Ensemble;

    fn search(library: &Library, search: &str) -> Result<Vec<SearchItem<Ensemble>>> {
        library.search_ensembles(search)
    }

    fn import(library: &Library, item: &Ensemble) -> Result<Ensemble> {
        library.import_metadata_ensemble(&item.ensemble_id)
    }

    fn search_placeholder() -> String {
        gettext("Search ensembles…")
    }

    fn create_label() -> String {
        gettext("Create new ensemble")
    }
}

pub struct InstrumentKind;

impl SelectorKind for InstrumentKind {
    type Item = Instrument;

    fn search(library: &Library, search: &str) -> Result<Vec<SearchItem<Instrument>>> {
        library.search_instruments(search)
    }

    fn import(library: &Library, item: &Instrument) -> Result<Instrument> {
        library.import_metadata_instrument(&item.instrument_id)
    }

    fn search_placeholder() -> String {
        gettext("Search instruments…")
    }

    fn create_label() -> String {
        gettext("Create new instrument")
    }
}

pub struct RoleKind;

impl SelectorKind for RoleKind {
    type Item = Role;

    fn search(library: &Library, search: &str) -> Result<Vec<SearchItem<Role>>> {
        library.search_roles(search)
    }

    fn import(library: &Library, item: &Role) -> Result<Role> {
        library.import_metadata_role(&item.role_id)
    }

    fn search_placeholder() -> String {
        gettext("Search roles…")
    }

    fn create_label() -> String {
        gettext("Create new role")
    }

    fn reset_tooltip() -> Option<String> {
        Some(gettext("Reset to default role"))
    }
}

pub struct TagKind;

impl SelectorKind for TagKind {
    type Item = Tag;

    fn search(library: &Library, search: &str) -> Result<Vec<SearchItem<Tag>>> {
        library.search_tags(search)
    }

    fn import(library: &Library, item: &Tag) -> Result<Tag> {
        library.import_metadata_tag(&item.tag_id)
    }

    fn search_placeholder() -> String {
        gettext("Search tags…")
    }

    fn create_label() -> String {
        gettext("Create new tag")
    }
}
