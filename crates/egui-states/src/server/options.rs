//! Native server configuration and asynchronous error handling.

use std::sync::Arc;
use std::time::Duration;

use super::ServerError;

/// Thread-safe callback used for asynchronous server errors.
///
/// It runs on a signal-worker thread. Panics from the handler are caught so
/// they cannot unwind out of that worker.
pub type ErrorHandler = Arc<dyn Fn(ServerError) + Send + Sync + 'static>;

/// Configuration used to construct a [`StateServer`](super::StateServer).
pub struct ServerOptions {
    /// Optional application version required during the client handshake.
    pub version: Option<u64>,
    /// Number of worker threads that invoke signal callbacks.
    ///
    /// Callbacks for different state ids may run concurrently. A slow callback
    /// occupies one worker until it returns.
    pub signal_workers: usize,
    /// How long dropping the server waits for the signal workers to finish the
    /// callback they are running. A worker only observes the shutdown flag
    /// between callbacks, so one that is inside a blocking callback cannot be
    /// waited on unboundedly -- once this elapses it is detached instead. This
    /// also bounds how long dropping the final [`StateServer`](super::StateServer)
    /// handle waits for callback workers.
    pub shutdown_timeout: Duration,
    /// Handler for asynchronous errors, or `None` to print them to stderr.
    pub error_handler: Option<ErrorHandler>,
}

impl ServerOptions {
    /// Returns default server options.
    ///
    /// The server uses three signal workers and does not require an application
    /// version.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for ServerOptions {
    fn default() -> Self {
        Self {
            version: None,
            signal_workers: 3,
            shutdown_timeout: Duration::from_secs(2),
            error_handler: None,
        }
    }
}
