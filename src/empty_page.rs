use std::cell::OnceCell;

use adw::{
    prelude::*,
    subclass::{navigation_page::NavigationPageImpl, prelude::*},
};
use gettextrs::gettext;
use glib::clone;
use gtk::{gio, glib, glib::subclass::Signal};
use once_cell::sync::Lazy;

use crate::{
    config, library::Library, process::Process, process_manager::ProcessManager,
    process_row::ProcessRow,
};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "data/ui/empty_page.blp")]
    pub struct EmptyPage {
        pub library: OnceCell<Library>,
        pub process_manager: OnceCell<ProcessManager>,

        #[template_child]
        pub download_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub process_list: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EmptyPage {
        const NAME: &'static str = "MusicusEmptyPage";
        type Type = super::EmptyPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for EmptyPage {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> =
                Lazy::new(|| vec![Signal::builder("ready").build()]);

            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for EmptyPage {}
    impl NavigationPageImpl for EmptyPage {}
}

glib::wrapper! {
    pub struct EmptyPage(ObjectSubclass<imp::EmptyPage>)
        @extends gtk::Widget, adw::NavigationPage;
}

#[gtk::template_callbacks]
impl EmptyPage {
    pub fn new(library: &Library, process_manager: &ProcessManager) -> Self {
        let obj: Self = glib::Object::new();

        for process in process_manager.processes() {
            obj.add_process(&process);
        }

        obj.imp().library.set(library.to_owned()).unwrap();
        obj.imp()
            .process_manager
            .set(process_manager.to_owned())
            .unwrap();

        obj
    }

    pub fn connect_ready<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("ready", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            f(&obj);
            None
        })
    }

    #[template_callback]
    async fn download_library(&self) {
        let dialog = adw::AlertDialog::builder()
            .heading(&gettext("Disclaimer"))
            .body(&gettext("You are about to download a library of audio files. These are from recordings that are in the public domain under EU law and are hosted on a server within the EU. Please ensure that you comply with the copyright laws of you country."))
            .build();

        dialog.add_response("continue", &gettext("Continue"));
        dialog.set_response_appearance("continue", adw::ResponseAppearance::Suggested);
        dialog.add_response("cancel", &gettext("Cancel"));
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");

        let obj = self.to_owned();
        glib::spawn_future_local(async move {
            if dialog.choose_future(&obj).await == "continue" {
                obj.imp().download_button.set_visible(false);

                let settings = gio::Settings::new(config::APP_ID);
                let url = if settings.boolean("use-custom-library-url") {
                    settings.string("custom-library-url").to_string()
                } else {
                    config::LIBRARY_URL.to_string()
                };

                match obj
                    .imp()
                    .library
                    .get()
                    .unwrap()
                    .import_library_from_url(&url)
                {
                    Ok(receiver) => {
                        let process = Process::new(&gettext("Downloading music library"), receiver);

                        process.connect_finished_notify(clone!(
                            #[weak]
                            obj,
                            move |process| {
                                if process.finished() {
                                    if process.error().is_some() {
                                        obj.imp().download_button.set_visible(true);
                                    } else {
                                        obj.emit_by_name::<()>("ready", &[]);
                                    }
                                }
                            }
                        ));

                        obj.imp()
                            .process_manager
                            .get()
                            .unwrap()
                            .add_process(&process);

                        obj.add_process(&process);
                    }
                    Err(err) => log::error!("Failed to download library: {err:?}"),
                }
            }
        });
    }

    fn add_process(&self, process: &Process) {
        let row = ProcessRow::new(process);

        row.connect_remove(clone!(
            #[weak(rename_to = obj)]
            self,
            move |row| {
                obj.imp()
                    .process_manager
                    .get()
                    .unwrap()
                    .remove_process(&row.process());

                obj.imp().process_list.remove(row);

                if obj.imp().process_list.first_child().is_none() {
                    obj.imp().process_list.set_visible(false);
                }
            }
        ));

        self.imp().process_list.append(&row);
        self.imp().process_list.set_visible(true);
    }
}
