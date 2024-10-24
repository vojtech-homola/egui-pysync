use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, RwLock};

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use egui_pytransport::transport::WriteMessage;
use egui_pytransport::values::{ReadValue, ValueMessage, WriteValue};
use egui_pytransport::EnumInt;

use crate::py_convert::FromPyValue;
use crate::signals::ChangedValues;
use crate::{Acknowledge, SyncTrait};

pub(crate) trait ProccesValue: Send + Sync {
    fn process_value(&self, head: &[u8], data: Option<Vec<u8>>, signal: bool)
        -> Result<(), String>;
}

pub(crate) trait PyValue: Send + Sync {
    fn get_py(&self, py: Python) -> PyObject;
    fn set_py(&self, value: &Bound<PyAny>, set_signal: bool, update: bool) -> PyResult<()>;
}

pub(crate) trait PyValueStatic: Send + Sync {
    fn get_py(&self, py: Python) -> PyObject;
    fn set_py(&self, value: &Bound<PyAny>, update: bool) -> PyResult<()>;
}

// Value ---------------------------------------------------
pub struct Value<T> {
    id: u32,
    value: RwLock<(T, usize)>,
    channel: Sender<WriteMessage>,
    connected: Arc<AtomicBool>,
    signals: ChangedValues,
}

impl<T> Value<T> {
    pub(crate) fn new(
        id: u32,
        value: T,
        channel: Sender<WriteMessage>,
        connected: Arc<AtomicBool>,
        signals: ChangedValues,
    ) -> Arc<Self> {
        Arc::new(Self {
            id,
            value: RwLock::new((value, 0)),
            channel,
            connected,
            signals,
        })
    }
}

impl<T> PyValue for Value<T>
where
    T: WriteValue + Clone + FromPyValue + ToPyObject,
{
    fn get_py(&self, py: Python) -> PyObject {
        self.value.read().unwrap().0.to_object(py)
    }

    fn set_py(&self, value: &Bound<PyAny>, set_signal: bool, update: bool) -> PyResult<()> {
        let value = T::from_python(value)?;
        if self.connected.load(Ordering::Relaxed) {
            let message = WriteMessage::Value(self.id, update, value.clone().into_message());
            let mut w = self.value.write().unwrap();
            w.0 = value.clone();
            w.1 += 1;
            self.channel.send(message).unwrap();
            if set_signal {
                self.signals.set(self.id, value);
            }
        } else {
            let mut w = self.value.write().unwrap();
            w.0 = value.clone();
            if set_signal {
                self.signals.set(self.id, value);
            }
        }

        Ok(())
    }
}

impl<T> ProccesValue for Value<T>
where
    T: ReadValue + WriteValue + ToPyObject,
{
    fn process_value(
        &self,
        head: &[u8],
        data: Option<Vec<u8>>,
        siganl: bool,
    ) -> Result<(), String> {
        let value = T::read_message(head, data)?;

        let mut w = self.value.write().unwrap();
        if w.1 == 0 {
            w.0 = value.clone();
        }

        if siganl {
            self.signals.set(self.id, value);
        }
        Ok(())
    }
}

impl<T: Sync + Send> Acknowledge for Value<T> {
    fn acknowledge(&self) {
        let mut w = self.value.write().unwrap();
        if w.1 > 0 {
            w.1 -= 1;
        }
    }
}

impl<T: Sync + Send> SyncTrait for Value<T>
where
    T: WriteValue + Clone,
{
    fn sync(&self) {
        let mut w = self.value.write().unwrap();
        w.1 = 1;
        let message = w.0.clone().into_message();
        drop(w);

        let message = WriteMessage::Value(self.id, false, message);
        self.channel.send(message).unwrap();
    }
}

// ValueStatic ---------------------------------------------------
pub struct ValueStatic<T> {
    id: u32,
    value: RwLock<T>,
    channel: Sender<WriteMessage>,
    connected: Arc<AtomicBool>,
}

impl<T> ValueStatic<T> {
    pub(crate) fn new(
        id: u32,
        value: T,
        channel: Sender<WriteMessage>,
        connected: Arc<AtomicBool>,
    ) -> Arc<Self> {
        Arc::new(Self {
            id,
            value: RwLock::new(value),
            channel,
            connected,
        })
    }
}

impl<T> PyValueStatic for ValueStatic<T>
where
    T: WriteValue + Clone + FromPyValue + ToPyObject,
{
    fn get_py(&self, py: Python) -> PyObject {
        self.value.read().unwrap().to_object(py)
    }

    fn set_py(&self, value: &Bound<PyAny>, update: bool) -> PyResult<()> {
        let value = T::from_python(value)?;
        if self.connected.load(Ordering::Relaxed) {
            let message = WriteMessage::Static(self.id, update, value.clone().into_message());
            let mut v = self.value.write().unwrap();
            *v = value;
            self.channel.send(message).unwrap();
        } else {
            *self.value.write().unwrap() = value;
        }

        Ok(())
    }
}

impl<T: Sync + Send> SyncTrait for ValueStatic<T>
where
    T: WriteValue + Clone,
{
    fn sync(&self) {
        let message = self.value.write().unwrap().clone().into_message();
        let message = WriteMessage::Static(self.id, false, message);
        self.channel.send(message).unwrap();
    }
}

// ValueEnum ---------------------------------------------------
pub struct ValueEnum<T> {
    id: u32,
    value: RwLock<(T, usize)>,
    channel: Sender<WriteMessage>,
    connected: Arc<AtomicBool>,
    signals: ChangedValues,
}

impl<T> ValueEnum<T> {
    pub(crate) fn new(
        id: u32,
        value: T,
        channel: Sender<WriteMessage>,
        connected: Arc<AtomicBool>,
        signals: ChangedValues,
    ) -> Arc<Self> {
        Arc::new(Self {
            id,
            value: RwLock::new((value, 0)),
            channel,
            connected,
            signals,
        })
    }
}

impl<T> PyValue for ValueEnum<T>
where
    T: EnumInt,
{
    fn get_py(&self, py: Python) -> PyObject {
        self.value.read().unwrap().0.as_int().to_object(py)
    }

    fn set_py(&self, value: &Bound<PyAny>, set_signal: bool, update: bool) -> PyResult<()> {
        let int_val = value.extract::<u64>()?;
        let value =
            T::from_int(int_val).map_err(|_| PyValueError::new_err("Invalid enum value"))?;

        if self.connected.load(Ordering::Relaxed) {
            let message = WriteMessage::Value(self.id, update, ValueMessage::U64(int_val));
            let mut w = self.value.write().unwrap();
            w.0 = value.clone();
            w.1 += 1;
            self.channel.send(message).unwrap();
            if set_signal {
                self.signals.set(self.id, int_val);
            }
        } else {
            let mut w = self.value.write().unwrap();
            w.0 = value.clone();
            if set_signal {
                self.signals.set(self.id, int_val);
            }
        }

        Ok(())
    }
}

impl<T> ProccesValue for ValueEnum<T>
where
    T: EnumInt,
{
    fn process_value(
        &self,
        head: &[u8],
        data: Option<Vec<u8>>,
        siganl: bool,
    ) -> Result<(), String> {
        let value_int = u64::read_message(head, data)?;
        let value = T::from_int(value_int).map_err(|_| "Invalid enum format".to_string())?;

        let mut w = self.value.write().unwrap();
        if w.1 == 0 {
            w.0 = value.clone();
        }

        if siganl {
            self.signals.set(self.id, value_int);
        }
        Ok(())
    }
}

impl<T: Sync + Send> Acknowledge for ValueEnum<T> {
    fn acknowledge(&self) {
        let mut w = self.value.write().unwrap();
        if w.1 > 0 {
            w.1 -= 1;
        }
    }
}

impl<T: Sync + Send> SyncTrait for ValueEnum<T>
where
    T: EnumInt,
{
    fn sync(&self) {
        let mut w = self.value.write().unwrap();
        w.1 = 1;
        let val_int = w.0.as_int();
        drop(w);

        let message = WriteMessage::Value(self.id, false, ValueMessage::U64(val_int));
        self.channel.send(message).unwrap();
    }
}

// Signal ---------------------------------------------------
pub struct Signal<T> {
    id: u32,
    signals: ChangedValues,
    phantom: PhantomData<T>,
}

impl<T: WriteValue + Clone> Signal<T> {
    pub(crate) fn new(id: u32, signals: ChangedValues) -> Arc<Self> {
        Arc::new(Self {
            id,
            signals,
            phantom: PhantomData,
        })
    }
}

impl<T> ProccesValue for Signal<T>
where
    T: ReadValue + WriteValue + ToPyObject,
{
    fn process_value(
        &self,
        head: &[u8],
        data: Option<Vec<u8>>,
        _signal: bool,
    ) -> Result<(), String> {
        let value = T::read_message(head, data)?;
        self.signals.set(self.id, value);
        Ok(())
    }
}
