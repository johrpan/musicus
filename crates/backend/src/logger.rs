use chrono::{Local, DateTime};
use log::{Level, LevelFilter, Log, Metadata, Record};
use std::{fmt::Display, sync::{Arc, Mutex}};

/// Register the custom logger. This will panic if called more than once.
pub fn register() -> Arc<Logger> {
    let logger = Arc::new(Logger::default());

    log::set_boxed_logger(Box::new(Arc::clone(&logger)))
        .map(|()| log::set_max_level(LevelFilter::Info))
        .unwrap();

    logger
}

/// A simple logging handler that prints out all messages and caches them for
/// later access by the user interface.
pub struct Logger {
    /// All messages since the start of the program.
    messages: Mutex<Vec<LogMessage>>,
}

impl Logger {
    pub fn messages(&self) -> Vec<LogMessage> {
        self.messages.lock().unwrap().clone()
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self {
            messages: Mutex::new(Vec::new()),
        }
    }
}

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if record.level() <= Level::Info {
            let message = record.into();
            println!("{}", message);
            self.messages.lock().unwrap().push(message);
        }
    }

    fn flush(&self) {}
}

/// A simplified representation of a [`Record`].
#[derive(Clone)]
pub struct LogMessage {
    pub time: DateTime<Local>,
    pub level: String,
    pub module: String,
    pub message: String,
}

impl<'a> From<&Record<'a>> for LogMessage {
    fn from(record: &Record<'a>) -> Self {
        Self {
            time: Local::now(),
            level: record.level().to_string(),
            module: String::from(record.module_path().unwrap_or_else(|| record.target())),
            message: format!("{}", record.args()),
        }
    }
}

impl Display for LogMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} ({}): {}", self.time, self.module, self.level, self.message)
    }
}
