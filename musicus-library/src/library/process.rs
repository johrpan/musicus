use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};

use anyhow::{bail, Result};

/// Run `operation` on a background thread, reporting its outcome as the final
/// message on the returned handle's channel.
pub fn spawn_process(
    operation: impl FnOnce(&async_channel::Sender<ProcessMsg>, &Cancellation) -> Result<()>
        + Send
        + 'static,
) -> ProcessHandle {
    let (sender, receiver) = async_channel::unbounded::<ProcessMsg>();
    let cancellation = Cancellation::new();

    let thread_cancellation = cancellation.clone();
    thread::spawn(move || {
        let result = operation(&sender, &thread_cancellation);

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

/// A progress update sent from a background library operation.
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

/// A handle on a running background library operation.
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

    /// A cancellation point: fails if cancellation has been requested.
    ///
    /// The resulting error is not shown to the user; the caller reports
    /// [`ProcessMsg::Cancelled`] instead.
    pub(crate) fn check(&self) -> Result<()> {
        if self.is_cancelled() {
            bail!("Cancelled");
        }

        Ok(())
    }
}
