use std::cell::RefCell;

use gtk::{
    glib::{self, clone, subclass::Signal},
    prelude::*,
    subclass::prelude::*,
};
use once_cell::sync::Lazy;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct ActivatableRow {
        pub previous_parent: RefCell<Option<gtk::ListBox>>,
        pub previous_signal_handler_id: RefCell<Option<glib::SignalHandlerId>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ActivatableRow {
        const NAME: &'static str = "MusicusActivatableRow";
        type Type = super::ActivatableRow;
        type ParentType = gtk::ListBoxRow;
    }

    impl ObjectImpl for ActivatableRow {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            obj.connect_parent_notify(clone!(
                #[weak]
                obj,
                move |_: &super::ActivatableRow| {
                    let previous_parent = obj.imp().previous_parent.borrow_mut().take();
                    let previous_signal_handler_id =
                        obj.imp().previous_signal_handler_id.borrow_mut().take();
                    if let (Some(previous_parent), Some(previous_signal_handler_id)) =
                        (previous_parent, previous_signal_handler_id)
                    {
                        previous_parent.disconnect(previous_signal_handler_id);
                    }

                    if let Some(parent) = obj.parent().and_downcast::<gtk::ListBox>() {
                        let signal_handler_id = parent.connect_row_activated(clone!(
                            #[weak]
                            obj,
                            move |_: &gtk::ListBox, row: &gtk::ListBoxRow| {
                                if *row == obj {
                                    obj.activate();
                                }
                            }
                        ));

                        obj.imp().previous_parent.replace(Some(parent));
                        obj.imp()
                            .previous_signal_handler_id
                            .replace(Some(signal_handler_id));
                    }
                }
            ));
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> =
                Lazy::new(|| vec![Signal::builder("activated").build()]);

            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for ActivatableRow {}

    impl ListBoxRowImpl for ActivatableRow {
        fn activate(&self) {
            self.obj().emit_by_name::<()>("activated", &[]);
        }
    }
}

glib::wrapper! {
    /// A simple helper widget for connecting a signal handler to a single [`gtk::ListBoxRow`] for
    /// handling activation.
    pub struct ActivatableRow(ObjectSubclass<imp::ActivatableRow>)
        @extends gtk::Widget, gtk::ListBoxRow;
}

impl ActivatableRow {
    pub fn new<W>(child: &W) -> Self
    where
        W: IsA<gtk::Widget>,
    {
        let obj: Self = glib::Object::builder()
            .property("activatable", true)
            .property("selectable", false)
            .build();

        obj.set_child(Some(child));
        obj
    }

    pub fn connect_activated<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("activated", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            f(&obj);
            None
        })
    }
}
