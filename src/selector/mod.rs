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

use crate::util::activatable_row::ActivatableRow;

/// The composer of a work that is about to be created.
#[derive(Debug, Default, Clone)]
pub enum ComposerPrefill {
    /// Nothing is known about the composer.
    #[default]
    Unknown,

    /// A person that already exists within the library.
    Person(Person),

    /// A person that the user chose to create first, whose name is already known.
    New(String),
}

/// Information the user already provided in a selector before choosing to create a new
/// work, so that the editor can start out pre-filled.
#[derive(Debug, Default, Clone)]
pub struct WorkPrefill {
    pub composer: ComposerPrefill,
    pub name: String,
}

/// The work of a recording that is about to be created.
#[derive(Debug, Default, Clone)]
pub enum RecordingWork {
    /// Nothing is known about the work.
    #[default]
    Unknown,

    /// A work that already exists within the library.
    Work(Work),

    /// A work that the user chose to create first.
    New(WorkPrefill),
}

/// Information the user already provided in a selector before choosing to create a new
/// recording, so that the editor can start out pre-filled.
#[derive(Debug, Default, Clone)]
pub struct RecordingPrefill {
    pub work: RecordingWork,
}

/// A row at the end of a selector's list that starts creating a new entity.
pub fn create_row<F: Fn() + 'static>(label: &str, f: F) -> ActivatableRow {
    let create_box = gtk::Box::builder().spacing(12).build();
    create_box.append(&gtk::Image::builder().icon_name("list-add-symbolic").build());
    create_box.append(
        &gtk::Label::builder()
            .label(label)
            .halign(gtk::Align::Start)
            .build(),
    );

    let row = ActivatableRow::new(&create_box);
    row.connect_activated(move |_: &ActivatableRow| f());

    row
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

const ITEM_ROW_INDENT: i32 = 24;

/// Indent once per each `level`.
pub fn item_row_child(text: &str, in_library: bool, level: u32) -> gtk::Widget {
    let margin_start = level as i32 * ITEM_ROW_INDENT;

    let label = gtk::Label::builder()
        .label(text)
        .halign(gtk::Align::Start)
        .ellipsize(pango::EllipsizeMode::Middle)
        .tooltip_text(text)
        .margin_start(margin_start)
        .build();

    if in_library {
        label.upcast()
    } else {
        let import_box = gtk::Box::builder()
            .spacing(12)
            .tooltip_text(text)
            .margin_start(margin_start)
            .build();

        import_box.append(
            &gtk::Image::builder()
                .icon_name("folder-download-symbolic")
                .build(),
        );

        import_box.append(&label);

        import_box.upcast()
    }
}

/// Every part in `parts`, depth-first, together with its nesting depth (1 for a
/// direct part, 2 for a part of a part, and so on).
pub fn flatten_parts(parts: &[Work], depth: usize) -> Vec<(Work, usize)> {
    let mut flattened = Vec::new();

    for part in parts {
        flattened.push((part.clone(), depth));
        flattened.extend(flatten_parts(&part.parts, depth + 1));
    }

    flattened
}
