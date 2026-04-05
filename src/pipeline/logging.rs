//! Logging utilities for the text extraction pipeline.
//!
//! These macros forward to the `log` crate so the configured log level
//! (via `log::set_max_level`, `env_logger`, `pyo3_log`, etc.) is honored.
//! When the `logging` feature is disabled, all logging is compiled out.

/// Log an INFO level message. Forwards to `log::info!`.
#[macro_export]
macro_rules! extract_log_info {
    ($($arg:tt)*) => {
        #[cfg(feature = "logging")]
        ::log::info!($($arg)*);
    };
}

/// Log a WARN level message. Forwards to `log::warn!`.
#[macro_export]
macro_rules! extract_log_warn {
    ($($arg:tt)*) => {
        #[cfg(feature = "logging")]
        ::log::warn!($($arg)*);
    };
}

/// Log a DEBUG level message. Forwards to `log::debug!`.
#[macro_export]
macro_rules! extract_log_debug {
    ($($arg:tt)*) => {
        #[cfg(feature = "logging")]
        ::log::debug!($($arg)*);
    };
}

/// Log a TRACE level message. Forwards to `log::trace!`.
#[macro_export]
macro_rules! extract_log_trace {
    ($($arg:tt)*) => {
        #[cfg(feature = "logging")]
        ::log::trace!($($arg)*);
    };
}

/// Log an ERROR level message. Forwards to `log::error!`.
#[macro_export]
macro_rules! extract_log_error {
    ($($arg:tt)*) => {
        #[cfg(feature = "logging")]
        ::log::error!($($arg)*);
    };
}

// Re-export the macros for convenience
pub use extract_log_debug;
pub use extract_log_error;
pub use extract_log_info;
pub use extract_log_trace;
pub use extract_log_warn;
