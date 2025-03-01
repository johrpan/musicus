use adw::{prelude::*, subclass::prelude::*};

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct DragWidget {}

    #[glib::object_subclass]
    impl ObjectSubclass for DragWidget {
        const NAME: &'static str = "MusicusDragWidget";
        type Type = super::DragWidget;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("dragwidget");
        }
    }

    impl ObjectImpl for DragWidget {}
    impl WidgetImpl for DragWidget {}
    impl BinImpl for DragWidget {}
}

glib::wrapper! {
    /// A simple helper widget for displaying a drag icon for a widget.
    pub struct DragWidget(ObjectSubclass<imp::DragWidget>)
        @extends gtk::Widget, adw::Bin;
}

impl DragWidget {
    pub fn new<W>(widget: &W) -> Self
    where
        W: IsA<gtk::Widget>,
    {
        let obj: Self = glib::Object::new();
        let picture = gtk::Picture::for_paintable(&gtk::WidgetPaintable::new(Some(widget)));
        obj.set_child(Some(&picture));
        obj
    }
}
