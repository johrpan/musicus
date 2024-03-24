use gtk::glib;
use lazy_static::lazy_static;

lazy_static! {
    /// The user's language code.
    pub static ref LANG: String = {
        let lang = match glib::language_names().first() {
            Some(language_name) => match language_name.split('_').next() {
                Some(lang) => lang.to_string(),
                None => "generic".to_string(),
            },
            None => "generic".to_string(),
        };
        
        log::info!("Intialized user language to '{lang}'.");
        lang
    };
}
