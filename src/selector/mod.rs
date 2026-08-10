pub mod kind;
pub mod performer_role;
pub mod popover;
pub mod recording;
pub mod work;

pub use popover::SelectorPopover;

use gtk::{
    gdk,
    glib::{self, clone, prelude::*},
    pango,
    prelude::*,
};

use musicus_library::db::models::{Person, Work};

/// Information the user already provided in a selector before choosing to create a new
/// work, so that the editor can start out pre-filled.
#[derive(Debug, Default, Clone)]
pub struct WorkPrefill {
    pub composer: Option<Person>,
    pub name: String,
}

/// Information the user already provided in a selector before choosing to create a new
/// recording, so that the editor can start out pre-filled.
#[derive(Debug, Default, Clone)]
pub struct RecordingPrefill {
    pub work: Option<Work>,
}

/// Let the up and down keys move the focus between a selector's search entry and its result list.
pub fn connect_keynav(search_entry: &gtk::SearchEntry, list_box: &gtk::ListBox) {
    let controller = gtk::EventControllerKey::new();
    controller.connect_key_pressed(clone!(
        #[weak]
        list_box,
        #[upgrade_or]
        glib::Propagation::Proceed,
        move |_, key, _, _| {
            if matches!(key, gdk::Key::Down | gdk::Key::KP_Down)
                && list_box.child_focus(gtk::DirectionType::Down)
            {
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        }
    ));
    search_entry.add_controller(controller);

    let controller = gtk::EventControllerKey::new();

    // The list box's own up/down key bindings would consume the key first in the bubble phase.
    controller.set_propagation_phase(gtk::PropagationPhase::Capture);

    controller.connect_key_pressed(clone!(
        #[weak]
        search_entry,
        #[weak]
        list_box,
        #[upgrade_or]
        glib::Propagation::Proceed,
        move |_, key, _, _| {
            if matches!(key, gdk::Key::Up | gdk::Key::KP_Up)
                && list_box.focus_child().is_some()
                && list_box.focus_child() == list_box.first_child()
            {
                search_entry.grab_focus();
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        }
    ));
    list_box.add_controller(controller);
}

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
