use std::io;
use std::fs;
use std::path;

use log;

use api;
use loggers;
use errors::InitError;

/// This is the base logger configuration in fern.
///
/// All DispatchConfig will do is filter log messages based on level, and then pass on to any
/// number of other loggers.
#[unstable]
pub struct DispatchConfig {
    /// The format for this logger. All log messages coming in will be sent through this closure
    /// before being sent to parent loggers
    pub format: Box<Formatter>,
    /// A list of loggers to send messages to. Any messages that are sent to this logger that
    /// aren't filtered are sent to each of these loggers in turn.
    pub output: Vec<OutputConfig>,
    /// The level of this logger. Any messages which have a lower level than this level won't be
    /// passed on.
    pub level: log::LogLevelFilter,
}

pub type Formatter = Fn(&str, &log::LogLevel, &log::LogLocation) -> String + Sync + Send;

/// This enum contains various outputs that you can send messages to.
///
/// You can use this in conjunction with DispatchConfig for message formating and filtering, or
/// just use this if you don't need to filter or format messages.
#[unstable]
pub enum OutputConfig {
    /// Child logger - another DispatchConfig
    Child(DispatchConfig),
    /// File logger - all messages sent to this will be output into the specified path. Note that
    /// the file will be opened appending, so nothing in the file will be overwritten.
    File(path::PathBuf),
    /// File logger with OpenOptions - all messages will be sent to the specified file. The file
    /// will be opened using the specified OpenOptions.
    FileOptions(path::PathBuf, fs::OpenOptions),
    /// Stdout logger - all messages sent to this will be printed to stdout.
    Stdout,
    /// Stderr logger - all messages sent to this will be printed to stderr.
    Stderr,
    /// Null logger - all messages sent to this logger will simply disappear into the void.
    Null,
    /// Custom logger - all messages sent here will just be sent on to the logger implementation
    /// you provide.
    Custom(Box<api::Logger>),
}

#[unstable]
impl IntoLog for OutputConfig {
    fn into_fern_logger(self) -> io::Result<Box<api::Logger>> {
        return Ok(match self {
            OutputConfig::Child(config) => try!(config.into_fern_logger()),
            OutputConfig::File(ref path) => Box::new(try!(
                loggers::WriterLogger::<fs::File>::with_file(path))) as Box<api::Logger>,
            OutputConfig::FileOptions(ref path, ref options) => Box::new(try!(
                loggers::WriterLogger::<fs::File>::with_file_with_options(path, options)))
                as Box<api::Logger>,
            OutputConfig::Stdout => Box::new(
                loggers::WriterLogger::<io::Stdout>::with_stdout()) as Box<api::Logger>,
            OutputConfig::Stderr => Box::new(
                loggers::WriterLogger::<io::Stderr>::with_stderr()) as Box<api::Logger>,
            OutputConfig::Null => Box::new(loggers::NullLogger) as Box<api::Logger>,
            OutputConfig::Custom(log) => log,
        });
    }

    fn into_log(self) -> io::Result<Box<log::Log>> {
        return Ok(match self {
            OutputConfig::Child(config) => try!(config.into_log()),
            OutputConfig::File(ref path) => Box::new(try!(
                loggers::WriterLogger::<fs::File>::with_file(path))) as Box<log::Log>,
            OutputConfig::FileOptions(ref path, ref options) => Box::new(try!(
                loggers::WriterLogger::<fs::File>::with_file_with_options(path, options)))
                as Box<log::Log>,
            OutputConfig::Stdout => Box::new(
                loggers::WriterLogger::<io::Stdout>::with_stdout()) as Box<log::Log>,
            OutputConfig::Stderr => Box::new(
                loggers::WriterLogger::<io::Stderr>::with_stderr()) as Box<log::Log>,
            OutputConfig::Null => Box::new(loggers::NullLogger) as Box<log::Log>,
            OutputConfig::Custom(log) => Box::new(log) as Box<log::Log>,
        });
    }
}

#[unstable]
impl IntoLog for DispatchConfig {
    fn into_fern_logger(self) -> io::Result<Box<api::Logger>> {
        let DispatchConfig {format, level, output} = self;
        let log = try!(loggers::DispatchLogger::new(format, output, level));
        return Ok(Box::new(log) as Box<api::Logger>);
    }

    fn into_log(self) -> io::Result<Box<log::Log>> {
        let DispatchConfig {format, level, output} = self;
        let log = try!(loggers::DispatchLogger::new(format, output, level));
        return Ok(Box::new(log) as Box<log::Log>);
    }
}

impl log::Log for Box<api::Logger> {
    fn enabled(&self, _level: log::LogLevel, _module: &str) -> bool {
        true
    }
    fn log(&self, record: &log::LogRecord) {
        loggers::log_with_fern_logger(self, record);
    }
}

#[unstable]
/// Trait which represents any logger configuration which can be built into a `fern::Logger` or
/// `log::Log`
pub trait IntoLog {
    /// Builds this config struct into a `fern::Logger` that you can send messages to. Note that
    /// this method is generally *not* useful for external use, you should instead use
    /// `OutputConfig.into_log()` in order to get a `log::Log`. This function is essentially the
    /// same as `into_log()`, except that it outputs a `Box<fern::Logger>` instead of
    /// `Box<log::Log>`.
    fn into_fern_logger(self) -> io::Result<Box<api::Logger>>;

    /// Builds this config struct into a `log::Log` that you can send messages to. This will open
    /// any files, get handles to stdout/stderr, etc. depending on which type of logger this is.
    fn into_log(self) -> io::Result<Box<log::Log>>;
}

pub fn init_global_logger<L: IntoLog>(config: L, global_log_level: log::LogLevelFilter)
        -> Result<(), InitError> {
    let log = try!(config.into_log());
    try!(log::set_logger(|max_log_level| {
        max_log_level.set(global_log_level);
        log
    }));
    return Ok(());
}
