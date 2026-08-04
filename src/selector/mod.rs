pub mod ensemble;
pub mod instrument;
pub mod performer_role;
pub mod person;
pub mod recording;
pub mod role;
pub mod work;

use gtk::{glib::prelude::*, pango, prelude::*};

pub fn item_row_child(text: &str, in_library: bool) -> gtk::Widget {
    let label = gtk::Label::builder()
        .label(text)
        .halign(gtk::Align::Start)
        .ellipsize(pango::EllipsizeMode::Middle)
        .tooltip_text(text)
        .build();

    if in_library {
        label.upcast()
    } else {
        let import_box = gtk::Box::builder().spacing(12).tooltip_text(text).build();

        import_box.append(
            &gtk::Image::builder()
                .icon_name("folder-download-symbolic")
                .build(),
        );

        import_box.append(&label);

        import_box.upcast()
    }
}
