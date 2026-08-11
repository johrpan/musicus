pub mod activatable_row;
pub mod drag_widget;
pub mod error_dialog;

use std::sync::LazyLock;

use gettextrs::gettext;
use gtk::{
    glib::{self, clone},
    prelude::*,
};
use musicus_library::{EntityKind, LibraryError};

use error_dialog::ErrorDialog;

/// The user's language code.
pub static LANG: LazyLock<String> = LazyLock::new(|| {
    let lang = match glib::language_names().first() {
        Some(language_name) => match language_name.split('_').next() {
            Some(lang) => lang.to_string(),
            None => "generic".to_string(),
        },
        None => "generic".to_string(),
    };

    log::info!("Intialized user language to '{lang}'.");
    lang
});

pub fn find_toast_overlay(widget: &impl IsA<gtk::Widget>) -> Option<adw::ToastOverlay> {
    let mut current = widget.as_ref().parent();

    while let Some(widget) = current {
        if let Ok(overlay) = widget.clone().downcast::<adw::ToastOverlay>() {
            return Some(overlay);
        }

        current = widget.parent();
    }

    None
}

/// The message to show for a failure the user can do something about.
///
/// These are written out per entity rather than composed from fragments, so
/// that translators get whole sentences.
fn expected_error_message(err: &LibraryError) -> Option<String> {
    let message = match err {
        LibraryError::StillReferenced(kind) => match kind {
            EntityKind::Person => gettext("This person is still used elsewhere in the library."),
            EntityKind::Role => gettext("This role is still used elsewhere in the library."),
            EntityKind::Instrument => {
                gettext("This instrument is still used elsewhere in the library.")
            }
            EntityKind::Tag => {
                gettext("This tag is still assigned to works or recordings in the library.")
            }
            EntityKind::Work => gettext("This work still has recordings in the library."),
            EntityKind::Ensemble => {
                gettext("This ensemble is still used elsewhere in the library.")
            }
            EntityKind::Recording => {
                gettext("This recording is still used by tracks or albums in the library.")
            }
            EntityKind::Album => gettext("This album is still used elsewhere in the library."),
            EntityKind::Medium => gettext("This medium is still used elsewhere in the library."),
            EntityKind::Track => gettext("This track is still used elsewhere in the library."),
        },
        LibraryError::SchemaTooNew { .. } => gettext(
            "This library was created by a newer version of Musicus. \
             Please update Musicus to open it.",
        ),
        LibraryError::Other(_) => return None,
    };

    Some(message)
}

/// Create and show an error toast. This will also log the error to the console.
pub fn error_toast(msgid: &str, err: impl Into<anyhow::Error>, toast_overlay: &adw::ToastOverlay) {
    let err = err.into();
    log::error!("{msgid}: {err:?}");

    // Failures the user can act on get their own message and no details button:
    // there is nothing useful behind it and they are not bugs.
    // The whole chain is searched because callers add context on the way up.
    if let Some(message) = err
        .chain()
        .find_map(|cause| cause.downcast_ref::<LibraryError>())
        .and_then(expected_error_message)
    {
        toast_overlay.add_toast(adw::Toast::new(&message));
        return;
    }

    let toast = adw::Toast::builder()
        .title(gettext(msgid))
        .button_label(gettext("Details"))
        .build();

    toast.connect_button_clicked(clone!(
        #[weak]
        toast_overlay,
        move |_| {
            ErrorDialog::present(&err, &toast_overlay);
        }
    ));

    toast_overlay.add_toast(toast);
}
