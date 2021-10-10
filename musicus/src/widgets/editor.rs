use super::Widget;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;

/// Common UI elements for an editor.
pub struct Editor {
    /// The actual GTK widget.
    pub widget: gtk::Stack,

    /// The button to switch to the previous screen.
    back_button: gtk::Button,

    /// The title widget within the header bar.
    window_title: adw::WindowTitle,

    /// The button to save the edited item.
    save_button: gtk::Button,

    /// The box containing the content.
    content_box: gtk::Box,

    /// The status page for the error screen.
    status_page: adw::StatusPage,
}

impl Editor {
    /// Create a new screen.
    pub fn new() -> Self {
        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/editor.ui");

        get_widget!(builder, gtk::Stack, widget);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, adw::WindowTitle, window_title);
        get_widget!(builder, gtk::Button, save_button);
        get_widget!(builder, gtk::Box, content_box);
        get_widget!(builder, adw::StatusPage, status_page);
        get_widget!(builder, gtk::Button, try_again_button);

        try_again_button.connect_clicked(clone!(@strong widget => move |_| {
            widget.set_visible_child_name("content");
        }));

        Self {
            widget,
            back_button,
            window_title,
            save_button,
            content_box,
            status_page,
        }
    }

    /// Set a closure to be called when the back button is pressed.
    pub fn set_back_cb<F: Fn() + 'static>(&self, cb: F) {
        self.back_button.connect_clicked(move |_| cb());
    }

    /// Show a title in the header bar.
    pub fn set_title(&self, title: &str) {
        self.window_title.set_title(title);
    }

    /// Set whether the user should be able to click the save button.
    pub fn set_may_save(&self, save: bool) {
        self.save_button.set_sensitive(save);
    }

    pub fn set_save_cb<F: Fn() + 'static>(&self, cb: F) {
        self.save_button.connect_clicked(move |_| cb());
    }

    /// Show a loading page.
    pub fn loading(&self) {
        self.widget.set_visible_child_name("loading");
    }

    /// Show an error page. The page contains a button to get back to the
    /// actual editor.
    pub fn error(&self, title: &str, description: &str) {
        self.status_page.set_title(title);
        self.status_page.set_description(Some(description));
        self.widget.set_visible_child_name("error");
    }

    /// Add content to the bottom of the content area.
    pub fn add_content<W: Widget>(&self, content: &W) {
        self.content_box.append(&content.get_widget());
    }
}
