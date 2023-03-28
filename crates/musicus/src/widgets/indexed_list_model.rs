use gio::prelude::*;
use gio::subclass::prelude::*;

use std::cell::Cell;

glib::wrapper! {
    /// A thin list model managing only indices to an external data source.
    pub struct IndexedListModel(ObjectSubclass<indexed_list_model_imp::IndexedListModel>)
        @implements gio::ListModel;
}

impl IndexedListModel {
    /// Set the length of the list model.
    pub fn set_length(&self, length: u32) {
        let old_length = self.n_items();
        self.set_n_items(length);
        self.items_changed(0, old_length, length);
    }
}

impl Default for IndexedListModel {
    fn default() -> Self {
        glib::Object::new()
    }
}

mod indexed_list_model_imp {
    use glib::Properties;

    use super::*;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::IndexedListModel)]
    pub struct IndexedListModel {
        #[property(get, set)]
        n_items: Cell<u32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for IndexedListModel {
        const NAME: &'static str = "IndexedListModel";
        type Type = super::IndexedListModel;
        type ParentType = glib::Object;
        type Interfaces = (gio::ListModel,);
    }

    impl ObjectImpl for IndexedListModel {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec)
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }
    }

    impl ListModelImpl for IndexedListModel {
        fn item_type(&self) -> glib::Type {
            ItemIndex::static_type()
        }

        fn n_items(&self) -> u32 {
            self.n_items.get()
        }

        fn item(&self, position: u32) -> Option<glib::Object> {
            Some(ItemIndex::new(position).upcast())
        }
    }
}

glib::wrapper! {
    /// A simple GObject holding just one integer.
    pub struct ItemIndex(ObjectSubclass<item_index_imp::ItemIndex>);
}

impl ItemIndex {
    /// Create a new item index.
    pub fn new(value: u32) -> Self {
        let object = glib::Object::new::<Self>();
        object.set_value(value);
        object
    }

    /// Get the value of the item index..
    pub fn get(&self) -> u32 {
        self.property("value")
    }
}

mod item_index_imp {
    use glib::Properties;

    use super::*;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::ItemIndex)]
    pub struct ItemIndex {
        #[property(get, set)]
        value: Cell<u32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ItemIndex {
        const NAME: &'static str = "ItemIndex";
        type Type = super::ItemIndex;
        type ParentType = glib::Object;
        type Interfaces = ();
    }

    impl ObjectImpl for ItemIndex {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec)
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }
    }
}
