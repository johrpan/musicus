use log::{Level, LevelFilter, Log, Metadata, Record};
use std::{fmt::Display, sync::Mutex};

/// Register the custom logger. This will panic if called more than once.
pub fn register() {
    log::set_boxed_logger(Box::new(Logger::default()))
        .map(|()| log::set_max_level(LevelFilter::Info))
        .unwrap();
}

/// A simple logging handler that prints out all messages and caches them for
/// later access by the user interface.
struct Logger {
    /// All messages since the start of the program.
    messages: Mutex<Vec<LogMessage>>,
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
struct LogMessage {
    pub level: String,
    pub module: String,
    pub message: String,
}

impl<'a> From<&Record<'a>> for LogMessage {
    fn from(record: &Record<'a>) -> Self {
        Self {
            level: record.level().to_string(),
            module: String::from(record.module_path().unwrap_or_else(|| record.target())),
            message: format!("{}", record.args()),
        }
    }
}

impl Display for LogMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({}): {}", self.module, self.level, self.message)
    }
}
