use gio::prelude::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;

/// A general framework for screens. Screens have a header bar with at least
/// a button to go back and a scrollable content area that clamps its content.
pub struct Screen {
    /// The actual GTK widget.
    pub widget: gtk::Box,

    /// The button to switch to the previous screen.
    back_button: gtk::Button,

    /// The title widget within the header bar.
    window_title: adw::WindowTitle,

    /// The action menu.
    menu: gio::Menu,

    /// The entry for searching.
    search_entry: gtk::SearchEntry,

    /// The stack to switch to the loading page.
    stack: gtk::Stack,

    /// The box containing the content.
    content_box: gtk::Box,

    /// The actions for the menu.
    actions: gio::SimpleActionGroup,
}

impl Screen {
    /// Create a new screen.
    pub fn new() -> Self {
        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/screen.ui");

        get_widget!(builder, gtk::Box, widget);
        get_widget!(builder, gtk::Button, back_button);
        get_widget!(builder, adw::WindowTitle, window_title);
        get_widget!(builder, gio::Menu, menu);
        get_widget!(builder, gtk::ToggleButton, search_button);
        get_widget!(builder, gtk::SearchEntry, search_entry);
        get_widget!(builder, gtk::Stack, stack);
        get_widget!(builder, gtk::Box, content_box);

        let actions = gio::SimpleActionGroup::new();
        widget.insert_action_group("widget", Some(&actions));

        search_button.connect_toggled(clone!(@strong search_entry => move |search_button| {
            if search_button.is_active() {
                search_entry.grab_focus();
            }
        }));

        Self {
            widget,
            back_button,
            window_title,
            menu,
            search_entry,
            stack,
            content_box,
            actions,
        }
    }

    /// Set a closure to be called when the back button is pressed.
    pub fn set_back_cb<F: Fn() + 'static>(&self, cb: F) {
        self.back_button.connect_clicked(move |_| cb());
    }

    /// Show a title in the header bar.
    pub fn set_title(&self, title: &str) {
        self.window_title.set_title(Some(title));
    }

    /// Show a subtitle in the header bar.
    pub fn set_subtitle(&self, subtitle: &str) {
        self.window_title.set_subtitle(Some(subtitle));
    }

    /// Add a new item to the action menu and register a callback for it.
    pub fn add_action<F: Fn() + 'static>(&self, label: &str, cb: F) {
        let name = rand::random::<u64>().to_string();
        let action = gio::SimpleAction::new(&name, None);
        action.connect_activate(move |_, _| cb());

        self.actions.add_action(&action);
        self.menu
            .append(Some(label), Some(&format!("widget.{}", name)));
    }

    /// Set the closure to be called when the search string has changed.
    pub fn set_search_cb<F: Fn() + 'static>(&self, cb: F) {
        self.search_entry.connect_search_changed(move |_| cb());
    }

    /// Get the current search string.
    pub fn get_search(&self) -> String {
        self.search_entry.text().to_string().to_lowercase()
    }

    /// Hide the loading page and switch to the content.
    pub fn ready(&self) {
        self.stack.set_visible_child_name("content");
    }

    /// Add content to the bottom of the content area.
    pub fn add_content<W: IsA<gtk::Widget>>(&self, content: &W) {
        self.content_box.append(content);
    }
}
