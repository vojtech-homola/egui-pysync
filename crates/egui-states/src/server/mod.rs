//! Native Rust server API, available with the `server` feature.
//!
//! Applications normally use bindings produced by
//! `generate_rust`. For manual construction, create a
//! [`StateServer`](crate::server::StateServer), register every state handle,
//! call [`StateServer::finalize`](crate::server::StateServer::finalize), and
//! then [`StateServer::start`](crate::server::StateServer::start). Retain every
//! [`CallbackHandle`](crate::server::CallbackHandle) for as long as its
//! callback should remain registered.

mod callbacks;
mod collections;
mod data;
mod error;
mod image;
mod logging;
mod options;
mod state_server;
mod values;

pub use callbacks::CallbackHandle;
pub use collections::{MapState, VecState};
pub use data::{Data, DataElement, DataMulti, DataMultiTake, DataTake};
pub use error::{Result, ServerError};
pub use image::{Image, ImageColor, ImageFormat, ImageMulti};
pub use logging::{LogLevel, LoggingSignal};
pub use options::{ErrorHandler, ServerOptions};
pub use state_server::StateServer;
pub use values::{Signal, Static, Value, ValueTake};
