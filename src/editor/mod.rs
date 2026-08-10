pub mod album;
pub mod default_program;
pub mod ensemble;
pub mod program;
pub mod program_settings;
pub mod recording;
pub mod simple_entity;
pub mod tracks;
pub mod translation;
pub mod translation_entry;
pub mod work;

use adw::prelude::*;
use anyhow::Result;

use crate::util;

pub fn handle_save<T>(page: &impl IsA<gtk::Widget>, result: Result<T>) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(err) => {
            match util::find_toast_overlay(page) {
                Some(toast_overlay) => util::error_toast("Failed to save", err, &toast_overlay),
                None => log::error!("Failed to save: {err:?}"),
            }

            None
        }
    }
}

pub fn has_name(name: &musicus_library::db::TranslatedString) -> bool {
    name.0.values().any(|value| !value.trim().is_empty())
}

pub fn require_name(
    page: &impl IsA<gtk::Widget>,
    name: &musicus_library::db::TranslatedString,
) -> bool {
    if has_name(name) {
        return true;
    }

    let message = gettextrs::gettext("Please enter a name.");
    match util::find_toast_overlay(page) {
        Some(toast_overlay) => toast_overlay.add_toast(adw::Toast::new(&message)),
        None => log::warn!("{message}"),
    }

    false
}
