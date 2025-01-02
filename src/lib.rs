pub mod build;

pub mod client;
pub mod client_state;
pub mod dict;
pub mod graphs;
pub mod image;
pub mod list;
pub mod values;

mod commands;
mod event;
mod nohash;
mod py_server;
mod python_convert;
mod server;
mod signals;
mod states_creator;
mod states_server;
mod transport;

use pyo3::prelude::*;

pub use dict::ValueDict;
pub use graphs::ValueGraphs;
pub use image::ValueImage;
pub use list::ValueList;
pub use states_creator::ValuesCreator;
pub use states_server::ServerValuesCreator;
pub use values::{Diff, Signal, Value, ValueStatic};

// traits for EnumValue -------------------------------------------------------
pub use egui_pysync_macros::EnumImpl;

pub trait EnumInt: Sized + Send + Sync + Copy {
    fn as_int(&self) -> u64;
    fn from_int(value: u64) -> Result<Self, ()>;
}

// python -----------------------------------------------------------------------
pub use python_convert::ToPython;

// nohash -----------------------------------------------------------------------
pub use nohash::{NoHashMap, NoHashSet};

// general traits --------------------------------------------------------------
pub(crate) trait SyncTrait: Sync + Send {
    fn sync(&self);
}

pub(crate) trait Acknowledge: Sync + Send {
    fn acknowledge(&self);
}

// server -----------------------------------------------------------------------
pub fn init_module(
    m: &Bound<PyModule>,
    create_function: fn(&mut states_server::ServerValuesCreator),
) -> PyResult<()> {
    py_server::CREATE_HOOK.set(create_function).map_err(|_| {
        pyo3::exceptions::PyRuntimeError::new_err("Failed to inicialize state server module.")
    })?;

    m.add_class::<py_server::StateServerCore>()?;

    Ok(())
}
