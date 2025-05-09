pub mod activatable_row;
pub mod drag_widget;
pub mod error_dialog;

use std::sync::LazyLock;

use gettextrs::gettext;
use gtk::glib::{self, clone};

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

/// Create and show an error toast. This will also log the error to the console.
pub fn error_toast(msgid: &str, err: anyhow::Error, toast_overlay: &adw::ToastOverlay) {
    log::error!("{msgid}: {err:?}");

    let toast = adw::Toast::builder()
        .title(&gettext(msgid))
        .button_label("Details")
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
