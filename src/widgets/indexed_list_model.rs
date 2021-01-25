use glib::prelude::*;
use glib::subclass;
use glib::subclass::prelude::*;
use gio::prelude::*;
use gio::subclass::prelude::*;
use once_cell::sync::Lazy;
use std::cell::Cell;

glib::wrapper! {
    pub struct IndexedListModel(ObjectSubclass<indexed_list_model::IndexedListModel>)
        @implements gio::ListModel;
}

impl IndexedListModel {
    /// Create a new indexed list model, which will be empty initially.
    pub fn new() -> Self {
        glib::Object::new(&[]).unwrap()
    }

    /// Set the length of the list model.
    pub fn set_length(&self, length: u32) {
        let old_length = self.get_property("length").unwrap().get_some::<u32>().unwrap();
        self.set_property("length", &length).unwrap();
        self.items_changed(0, old_length, length);
    }
}

mod indexed_list_model {
    use super::*;

    #[derive(Debug)]
    pub struct IndexedListModel {
        length: Cell<u32>,
    }

    impl ObjectSubclass for IndexedListModel {
        const NAME: &'static str = "IndexedListModel";

        type Type = super::IndexedListModel;
        type ParentType = glib::Object;
        type Interfaces = (gio::ListModel,);
        type Instance = subclass::simple::InstanceStruct<Self>;
        type Class = subclass::simple::ClassStruct<Self>;

        glib::object_subclass!();

        fn new() -> Self {
            Self { length: Cell::new(0) }
        }
    }

    impl ObjectImpl for IndexedListModel {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpec::uint(
                        "length",
                        "Length",
                        "Length",
                        0,
                        std::u32::MAX,
                        0,
                        glib::ParamFlags::READWRITE,
                    ),
                ]
            });

            PROPERTIES.as_ref()
        }

        fn set_property(&self, _obj: &Self::Type, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.get_name() {
                "length" => {
                    let length = value.get().unwrap().unwrap();
                    self.length.set(length);
                }
                _ => unimplemented!(),
            }
        }

        fn get_property(&self, _obj: &Self::Type, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.get_name() {
                "length" => self.length.get().to_value(),
                _ => unimplemented!(),
            }
        }
    }

    impl ListModelImpl for IndexedListModel {
        fn get_item_type(&self, _: &Self::Type) -> glib::Type {
            ItemIndex::static_type()
        }

        fn get_n_items(&self, _: &Self::Type) -> u32 {
            self.length.get()
        }

        fn get_item(&self, _: &Self::Type, position: u32) -> Option<glib::Object> {
            Some(ItemIndex::new(position).upcast())
        }
    }
}

glib::wrapper! {
    pub struct ItemIndex(ObjectSubclass<item_index::ItemIndex>);
}

impl ItemIndex {
    /// Create a new item index.
    pub fn new(value: u32) -> Self {
        glib::Object::new(&[("value", &value)]).unwrap()
    }

    /// Get the value of the item index..
    pub fn get(&self) -> u32 {
        self.get_property("value").unwrap().get_some::<u32>().unwrap()
    }
}

mod item_index {
    use super::*;

    #[derive(Debug)]
    pub struct ItemIndex {
        value: Cell<u32>,
    }

    impl ObjectSubclass for ItemIndex {
        const NAME: &'static str = "ItemIndex";

        type Type = super::ItemIndex;
        type ParentType = glib::Object;
        type Interfaces = ();
        type Instance = subclass::simple::InstanceStruct<Self>;
        type Class = subclass::simple::ClassStruct<Self>;

        glib::object_subclass!();

        fn new() -> Self {
            Self { value: Cell::new(0) }
        }
    }

    impl ObjectImpl for ItemIndex {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpec::uint(
                        "value",
                        "Value",
                        "Value",
                        0,
                        std::u32::MAX,
                        0,
                        glib::ParamFlags::READWRITE,
                    ),
                ]
            });

            PROPERTIES.as_ref()
        }

        fn set_property(&self, _obj: &Self::Type, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.get_name() {
                "value" => {
                    let value = value.get().unwrap().unwrap();
                    self.value.set(value);
                }
                _ => unimplemented!(),
            }
        }

        fn get_property(&self, _obj: &Self::Type, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.get_name() {
                "value" => self.value.get().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}
