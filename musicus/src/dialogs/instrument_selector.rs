use super::InstrumentEditor;
use crate::backend::Backend;
use crate::database::Instrument;
use crate::widgets::List;
use gettextrs::gettext;
use gio::prelude::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use std::cell::RefCell;
use std::rc::Rc;

/// A dialog for selecting a instrument.
pub struct InstrumentSelector {
    backend: Rc<Backend>,
    window: libhandy::Window,
    server_check_button: gtk::CheckButton,
    stack: gtk::Stack,
    list: Rc<List<Instrument>>,
    selected_cb: RefCell<Option<Box<dyn Fn(Instrument) -> ()>>>,
}

impl InstrumentSelector {
    pub fn new<P>(backend: Rc<Backend>, parent: &P) -> Rc<Self>
    where
        P: IsA<gtk::Window>,
    {
        // Create UI

        let builder = gtk::Builder::from_resource("/de/johrpan/musicus/ui/instrument_selector.ui");

        get_widget!(builder, libhandy::Window, window);
        get_widget!(builder, gtk::Button, add_button);
        get_widget!(builder, gtk::CheckButton, server_check_button);
        get_widget!(builder, gtk::SearchEntry, search_entry);
        get_widget!(builder, gtk::Stack, stack);
        get_widget!(builder, gtk::ScrolledWindow, scroll);
        get_widget!(builder, gtk::Button, try_again_button);

        window.set_transient_for(Some(parent));

        let list = List::<Instrument>::new(&gettext("No instruments found."));
        scroll.add(&list.widget);

        let this = Rc::new(Self {
            backend,
            window,
            server_check_button,
            stack,
            list,
            selected_cb: RefCell::new(None),
        });

        // Connect signals and callbacks

        add_button.connect_clicked(clone!(@strong this => move |_| {
            let editor = InstrumentEditor::new(
                this.backend.clone(),
                &this.window,
                None,
            );

            editor.set_saved_cb(clone!(@strong this => move |instrument| {
                if let Some(cb) = &*this.selected_cb.borrow() {
                    cb(instrument);
                }

                this.window.close();
            }));

            editor.show();
        }));

        search_entry.connect_search_changed(clone!(@strong this => move |_| {
            this.list.invalidate_filter();
        }));

        let load_online = Rc::new(clone!(@strong this => move || {
            this.stack.set_visible_child_name("loading");

            let context = glib::MainContext::default();
            let clone = this.clone();
            context.spawn_local(async move {
                match clone.backend.get_instruments().await {
                    Ok(instruments) => {
                        clone.list.show_items(instruments);
                        clone.stack.set_visible_child_name("content");
                    }
                    Err(_) => {
                        clone.list.show_items(Vec::new());
                        clone.stack.set_visible_child_name("error");
                    }
                }
            });
        }));

        let load_local = Rc::new(clone!(@strong this => move || {
            this.stack.set_visible_child_name("loading");

            let context = glib::MainContext::default();
            let clone = this.clone();
            context.spawn_local(async move {
                let instruments = clone.backend.db().get_instruments().await.unwrap();
                clone.list.show_items(instruments);
                clone.stack.set_visible_child_name("content");
            });
        }));

        this.server_check_button.connect_toggled(
            clone!(@strong this, @strong load_local, @strong load_online => move |_| {
                if this.server_check_button.get_active() {
                    load_online();
                } else {
                    load_local();
                }
            }),
        );

        this.list.set_make_widget(|instrument: &Instrument| {
            let label = gtk::Label::new(Some(&instrument.name));
            label.set_halign(gtk::Align::Start);
            label.set_margin_start(6);
            label.set_margin_end(6);
            label.set_margin_top(6);
            label.set_margin_bottom(6);
            label.upcast()
        });

        this.list
            .set_filter(clone!(@strong search_entry => move |instrument: &Instrument| {
                let search = search_entry.get_text().to_string().to_lowercase();
                search.is_empty() || instrument.name.contains(&search)
            }));

        this.list.set_selected(clone!(@strong this => move |work| {
            if let Some(cb) = &*this.selected_cb.borrow() {
                cb(work.clone());
            }

            this.window.close();
        }));

        try_again_button.connect_clicked(clone!(@strong load_online => move |_| {
            load_online();
        }));

        // Initialize
        load_online();

        this
    }

    /// Set the closure to be called when the user has selected a instrument.
    pub fn set_selected_cb<F: Fn(Instrument) -> () + 'static>(&self, cb: F) {
        self.selected_cb.replace(Some(Box::new(cb)));
    }

    /// Show the instrument selector.
    pub fn show(&self) {
        self.window.show();
    }
}
