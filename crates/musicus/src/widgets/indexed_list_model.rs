use gio::prelude::*;
use gio::subclass::prelude::*;
use once_cell::sync::Lazy;
use std::cell::Cell;

glib::wrapper! {
    /// A thin list model managing only indices to an external data source.
    pub struct IndexedListModel(ObjectSubclass<indexed_list_model_imp::IndexedListModel>)
        @implements gio::ListModel;
}

impl IndexedListModel {
    /// Set the length of the list model.
    pub fn set_length(&self, length: u32) {
        let old_length = self.property("length");
        self.set_property("length", &length);
        self.items_changed(0, old_length, length);
    }
}

impl Default for IndexedListModel {
    fn default() -> Self {
        glib::Object::new(&[])
    }
}

mod indexed_list_model_imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct IndexedListModel {
        length: Cell<u32>,
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
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecUInt::new(
                    "length",
                    "Length",
                    "Length",
                    0,
                    std::u32::MAX,
                    0,
                    glib::ParamFlags::READWRITE,
                )]
            });

            PROPERTIES.as_ref()
        }

        fn set_property(&self, _: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "length" => {
                    let length = value.get::<u32>().unwrap();
                    self.length.set(length);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "length" => self.length.get().to_value(),
                _ => unimplemented!(),
            }
        }
    }

    impl ListModelImpl for IndexedListModel {
        fn item_type(&self) -> glib::Type {
            ItemIndex::static_type()
        }

        fn n_items(&self) -> u32 {
            self.length.get()
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
        glib::Object::new(&[("value", &value)])
    }

    /// Get the value of the item index..
    pub fn get(&self) -> u32 {
        self.property("value")
    }
}

mod item_index_imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct ItemIndex {
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
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecUInt::new(
                    "value",
                    "Value",
                    "Value",
                    0,
                    std::u32::MAX,
                    0,
                    glib::ParamFlags::READWRITE,
                )]
            });

            PROPERTIES.as_ref()
        }

        fn set_property(&self, _: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "value" => {
                    let value = value.get::<u32>().unwrap();
                    self.value.set(value);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "value" => self.value.get().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}
