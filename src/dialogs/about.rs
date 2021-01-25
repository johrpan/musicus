use crate::config;
use gettextrs::gettext;
use gtk::prelude::*;

pub fn show_about_dialog<W: IsA<gtk::Window>>(parent: &W) {
    let dialog = gtk::AboutDialogBuilder::new()
        .transient_for(parent)
        .modal(true)
        .logo_icon_name("de.johrpan.musicus")
        .program_name(&gettext("Musicus"))
        .version(config::VERSION)
        .comments(&gettext("The classical music player and organizer."))
        .website("https://github.com/johrpan/musicus")
        .website_label(&gettext("Further information and source code"))
        .copyright("© 2020 Elias Projahn")
        .license_type(gtk::License::Agpl30)
        .authors(vec![String::from("Elias Projahn <johrpan@gmail.com>")])
        .build();

    dialog.show();
}
