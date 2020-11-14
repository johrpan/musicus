use glib::prelude::*;
use glib::subclass;
use glib::subclass::prelude::*;
use glib::translate::*;
use glib::{glib_object_impl, glib_object_subclass, glib_wrapper};
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use std::cell::{Cell, RefCell};

glib_wrapper! {
    pub struct SelectorRow(
        Object<subclass::simple::InstanceStruct<SelectorRowPriv>,
            subclass::simple::ClassStruct<SelectorRowPriv>,
            SelectorRowClass>
    ) @extends gtk::Bin, gtk::Container, gtk::Widget;

    match fn {
        get_type => || SelectorRowPriv::get_type().to_glib(),
    }
}

impl SelectorRow {
    pub fn new<T: IsA<gtk::Widget>>(index: u64, child: &T) -> Self {
        glib::Object::new(
            Self::static_type(),
            &[("index", &index), ("child", child.upcast_ref())],
        )
        .expect("Failed to create SelectorRow GObject!")
        .downcast()
        .expect("SelectorRow GObject is of the wrong type!")
    }

    pub fn get_index(&self) -> u64 {
        self.get_property("index").unwrap().get().unwrap().unwrap()
    }
}

pub struct SelectorRowPriv {
    index: Cell<u64>,
    child: RefCell<Option<gtk::Widget>>,
}

static PROPERTIES: [subclass::Property; 2] = [
    subclass::Property("index", |name| {
        glib::ParamSpec::uint64(
            name,
            "Index",
            "Index",
            0,
            u64::MAX,
            0,
            glib::ParamFlags::READWRITE,
        )
    }),
    subclass::Property("child", |name| {
        glib::ParamSpec::object(
            name,
            "Child",
            "Child",
            gtk::Widget::static_type(),
            glib::ParamFlags::READWRITE,
        )
    }),
];

impl ObjectSubclass for SelectorRowPriv {
    const NAME: &'static str = "SelectorRow";
    type ParentType = gtk::Bin;
    type Instance = subclass::simple::InstanceStruct<Self>;
    type Class = subclass::simple::ClassStruct<Self>;

    glib_object_subclass!();

    fn class_init(klass: &mut Self::Class) {
        klass.install_properties(&PROPERTIES);
    }

    fn new() -> Self {
        Self {
            index: Cell::new(0),
            child: RefCell::new(None),
        }
    }
}

impl ObjectImpl for SelectorRowPriv {
    glib_object_impl!();

    fn constructed(&self, object: &glib::Object) {
        self.parent_constructed(object);

        let row = object.downcast_ref::<SelectorRow>().unwrap();

        let child = self.child.borrow();
        match child.as_ref() {
            Some(child) => row.add(child),
            None => (),
        }
    }

    fn set_property(&self, object: &glib::Object, id: usize, value: &glib::Value) {
        let prop = &PROPERTIES[id];

        match *prop {
            subclass::Property("index", ..) => {
                let index = value
                    .get_some()
                    .expect("Wrong type for SelectorRow GObject index property!");
                self.index.set(index);
            }
            subclass::Property("child", ..) => {
                let child = value
                    .get()
                    .expect("Wrong type for SelectorRow GObject child property!");

                let row = object.downcast_ref::<SelectorRow>().unwrap();

                {
                    let old = self.child.borrow();
                    match old.as_ref() {
                        Some(old) => row.remove(old),
                        None => (),
                    }
                }

                self.child.replace(child.clone());
                match child {
                    Some(child) => row.add(&child),
                    None => (),
                }
            }
            _ => unimplemented!(),
        }
    }

    fn get_property(&self, _obj: &glib::Object, id: usize) -> Result<glib::Value, ()> {
        let prop = &PROPERTIES[id];

        match *prop {
            subclass::Property("index", ..) => Ok(self.index.get().to_value()),
            subclass::Property("child", ..) => Ok(self.child.borrow().to_value()),
            _ => unimplemented!(),
        }
    }
}

impl WidgetImpl for SelectorRowPriv {}
impl ContainerImpl for SelectorRowPriv {}
impl BinImpl for SelectorRowPriv {}
