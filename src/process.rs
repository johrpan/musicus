use std::{
    cell::{Cell, OnceCell, RefCell},
    ops::ControlFlow,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};

use anyhow::Result;
use gettextrs::gettext;
use gtk::{
    glib::{self, Properties},
    prelude::*,
    subclass::prelude::*,
};
use musicus_library::{
    db::tables::Source,
    library::exchange::{self, ExchangeEvent},
};

use musicus_library::format_translated;

/// A progress update sent from a background exchange operation.
///
/// Exactly one of [`ProcessMsg::Result`] or [`ProcessMsg::Cancelled`] is sent,
/// as the last message before the channel closes.
#[derive(Debug)]
pub enum ProcessMsg {
    Message(String),
    /// A problem that did not stop the operation but that the user should see
    /// anyway, and still see once it has finished.
    Warning(String),
    Progress(f64),
    Result(Result<()>),
    /// The operation stopped because cancellation was requested.
    Cancelled,
}

/// A handle on a running background exchange operation.
pub struct ProcessHandle {
    pub receiver: async_channel::Receiver<ProcessMsg>,
    pub cancellation: Cancellation,
}

/// A shared flag asking a background operation to stop at its next
/// cancellation point.
///
/// Cancellation is cooperative and not instantaneous: an operation only stops
/// between units of work, never in the middle of a database transaction.
#[derive(Clone, Debug, Default)]
pub struct Cancellation(Arc<AtomicBool>);

impl Cancellation {
    pub fn new() -> Self {
        Self::default()
    }

    /// Ask the operation to stop. Returns immediately; the operation stops at
    /// its next cancellation point.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

/// The bridge from a background exchange operation to the process UI.
///
/// The worker thread receives one of these and reports progress, messages and
/// warnings through it; it can also ask whether cancellation was requested.
/// The library's synchronous exchange functions take an observer built from
/// this bridge.
pub struct ExchangeBridge<'a> {
    sender: &'a async_channel::Sender<ProcessMsg>,
    cancellation: &'a Cancellation,
}

impl<'a> ExchangeBridge<'a> {
    /// Report a status message.
    pub fn message(&self, message: String) {
        let _ = self.sender.send_blocking(ProcessMsg::Message(message));
    }

    /// Report progress from 0.0 to 1.0, failing if cancellation was requested.
    pub fn progress(&self, fraction: f64) -> Result<()> {
        let _ = self.sender.send_blocking(ProcessMsg::Progress(fraction));
        self.check()
    }

    /// A cancellation point: fails if cancellation has been requested. The
    /// resulting error is not shown to the user; the runner reports
    /// [`ProcessMsg::Cancelled`] instead.
    pub fn check(&self) -> Result<()> {
        if self.cancellation.is_cancelled() {
            anyhow::bail!("Cancelled");
        }

        Ok(())
    }

    /// Report a library exchange event, returning `Break` once cancelled.
    pub fn observe(&self, event: ExchangeEvent) -> ControlFlow<()> {
        let message = match event {
            ExchangeEvent::Message(message) => ProcessMsg::Message(message),
            ExchangeEvent::Warning(warning) => ProcessMsg::Warning(warning),
            ExchangeEvent::Progress(fraction) => ProcessMsg::Progress(fraction),
        };

        let _ = self.sender.send_blocking(message);

        if self.cancellation.is_cancelled() {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    }
}

/// Run `operation` on a background thread, translating its synchronous events
/// into process messages. The operation is cancelled cooperatively via the
/// returned handle.
pub fn spawn_process(
    operation: impl FnOnce(&mut ExchangeBridge<'_>) -> Result<()> + Send + 'static,
) -> ProcessHandle {
    let (sender, receiver) = async_channel::unbounded::<ProcessMsg>();
    let cancellation = Cancellation::new();

    let thread_cancellation = cancellation.clone();
    thread::spawn(move || {
        let mut bridge = ExchangeBridge {
            sender: &sender,
            cancellation: &thread_cancellation,
        };

        let result = operation(&mut bridge);

        // A cancelled operation fails with a sentinel error that must not be
        // reported to the user as a failure.
        let msg = if thread_cancellation.is_cancelled() {
            ProcessMsg::Cancelled
        } else {
            ProcessMsg::Result(result)
        };

        if let Err(err) = sender.send_blocking(msg) {
            log::error!("Failed to send library action result: {err:?}");
        }
    });

    ProcessHandle {
        receiver,
        cancellation,
    }
}

/// Download a music library archive and import it.
///
/// The download goes to a temporary file first; only a complete download is
/// handed to the library's synchronous import.
pub fn update_library(url: &str, library: &crate::library::Library) -> Result<Process> {
    let context = library.exchange_context();
    let url = url.to_owned();

    let handle = spawn_process(move |bridge| {
        bridge.message(gettext("Downloading").to_owned());

        let file = crate::download::download(&url, None, bridge)?;

        bridge.message(gettext("Importing downloaded library").to_owned());

        let mut observer = |event: ExchangeEvent| bridge.observe(event);
        exchange::import_library_zip(
            &context.connection,
            &context.folder,
            file.path(),
            Source::Metadata,
            &mut observer,
        )
        .map(|_| ())
    });

    Ok(Process::new(&gettext("Updating music library"), handle))
}

/// Import a music library from a local archive.
pub fn import_library(
    path: std::path::PathBuf,
    library: &crate::library::Library,
) -> Result<Process> {
    let context = library.exchange_context();

    let description = format_translated!(
        gettext("Importing music library from {}"),
        path.file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| gettext("archive"))
    );

    let handle = spawn_process(move |bridge| {
        let mut observer = |event: ExchangeEvent| bridge.observe(event);
        exchange::import_library_zip(
            &context.connection,
            &context.folder,
            &path,
            Source::Import,
            &mut observer,
        )
        .map(|_| ())
    });

    Ok(Process::new(&description, handle))
}

/// Export the library to a local archive.
pub fn export_library(
    path: std::path::PathBuf,
    library: &crate::library::Library,
) -> Result<Process> {
    let context = library.exchange_context();

    let description = format_translated!(
        gettext("Exporting music library to {}"),
        path.file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| gettext("archive"))
    );

    let handle = spawn_process(move |bridge| {
        let mut observer = |event: ExchangeEvent| bridge.observe(event);
        exchange::export_library_zip(&context.connection, &context.folder, &path, &mut observer)
    });

    Ok(Process::new(&description, handle))
}

/// Download the metadata database and install it.
///
/// The download is validated and applied by the library before the cached
/// database file is replaced.
pub fn update_metadata(url: &str, library: &crate::library::Library) -> Result<Process> {
    let context = library.exchange_context();
    let url = url.to_owned();

    let handle = spawn_process(move |bridge| {
        bridge.message(gettext("Downloading").to_owned());

        let file = crate::download::download(&url, None, bridge)?;

        bridge.message(gettext("Importing downloaded library").to_owned());

        let mut observer = |event: ExchangeEvent| bridge.observe(event);
        exchange::install_metadata(
            &context.connection,
            &context.cache_dir,
            file.path(),
            &mut observer,
        )
    });

    Ok(Process::new(&gettext("Updating metadata"), handle))
}

mod imp {
    use super::*;

    #[derive(Properties, Default, Debug)]
    #[properties(wrapper_type = super::Process)]
    pub struct Process {
        #[property(get, construct_only)]
        pub description: OnceCell<String>,
        #[property(get, set, nullable)]
        pub message: RefCell<Option<String>>,
        /// A problem that did not stop the process. Unlike the message, this
        /// outlives the process, because it is what the user has to act on.
        #[property(get, set, nullable)]
        pub warning: RefCell<Option<String>>,
        #[property(get, set)]
        pub progress: Cell<f64>,
        #[property(get, set)]
        pub finished: Cell<bool>,
        #[property(get, set)]
        pub error: RefCell<Option<String>>,
        /// Whether the process stopped because the user cancelled it.
        #[property(get, set)]
        pub cancelled: Cell<bool>,
        pub cancellation: OnceCell<Cancellation>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Process {
        const NAME: &'static str = "MusicusProcess";
        type Type = super::Process;
    }

    #[glib::derived_properties]
    impl ObjectImpl for Process {}
}

glib::wrapper! {
    pub struct Process(ObjectSubclass<imp::Process>);
}

impl Process {
    pub fn new(description: &str, handle: ProcessHandle) -> Self {
        let obj: Self = glib::Object::builder()
            .property("description", description)
            .build();

        let ProcessHandle {
            receiver,
            cancellation,
        } = handle;

        let _ = obj.imp().cancellation.set(cancellation);

        let obj_clone = obj.clone();
        glib::spawn_future_local(async move {
            while let Ok(msg) = receiver.recv().await {
                match msg {
                    ProcessMsg::Message(message) => {
                        obj_clone.set_message(Some(message));
                    }
                    ProcessMsg::Warning(warning) => {
                        log::warn!(
                            "Process \"{}\" reported a problem: {warning}",
                            obj_clone.description()
                        );
                        obj_clone.set_warning(Some(warning));
                    }
                    ProcessMsg::Progress(fraction) => {
                        obj_clone.set_progress(fraction);
                    }
                    ProcessMsg::Result(result) => {
                        obj_clone.set_message(None::<String>);

                        if let Err(err) = result {
                            log::error!("Process \"{}\" failed: {err:?}", obj_clone.description());
                            obj_clone.set_error(err.to_string());
                        }

                        obj_clone.set_finished(true);
                    }
                    ProcessMsg::Cancelled => {
                        log::info!("Process \"{}\" was cancelled", obj_clone.description());
                        obj_clone.set_message(None::<String>);
                        obj_clone.set_cancelled(true);
                        obj_clone.set_finished(true);
                    }
                }
            }
        });

        obj
    }

    /// Whether this process can still be cancelled.
    pub fn is_cancellable(&self) -> bool {
        !self.finished() && !self.cancellation_requested()
    }

    pub fn cancellation_requested(&self) -> bool {
        self.imp()
            .cancellation
            .get()
            .is_some_and(|cancellation| cancellation.is_cancelled())
    }

    /// Ask the background operation to stop. It stops at its next cancellation
    /// point, so `finished` may only become true some time afterwards.
    pub fn cancel(&self) {
        if let Some(cancellation) = self.imp().cancellation.get() {
            cancellation.cancel();
        }
    }
}
