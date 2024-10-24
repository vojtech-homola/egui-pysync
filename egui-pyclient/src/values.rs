use std::marker::PhantomData;
use std::sync::mpsc::Sender;
use std::sync::{Arc, RwLock};

use egui_pytransport::transport::WriteMessage;
use egui_pytransport::values::{ReadValue, ValueMessage, WriteValue};
use egui_pytransport::EnumInt;

pub(crate) trait ValueUpdate: Send + Sync {
    fn update_value(&self, head: &[u8], data: Option<Vec<u8>>) -> Result<(), String>;
}

pub struct Diff<T> {
    pub v: T,
    original: T,
}

impl<T: WriteValue + Clone + PartialEq> Diff<T> {
    pub fn new(value: &Value<T>) -> Self {
        let v = value.get();
        Self {
            v: v.clone(),
            original: v,
        }
    }

    #[inline]
    pub fn set(self, v: &Value<T>, signal: bool) {
        if self.v != self.original {
            v.set(self.v, signal);
        }
    }
}

pub struct DiffEnum<T> {
    pub v: T,
    original: T,
}

impl<T: EnumInt + Clone + PartialEq> DiffEnum<T> {
    pub fn new(value: &ValueEnum<T>) -> Self {
        let v = value.get();
        Self {
            v: v.clone(),
            original: v,
        }
    }

    #[inline]
    pub fn set(self, v: &ValueEnum<T>, signal: bool) {
        if self.v != self.original {
            v.set(self.v, signal);
        }
    }
}

// Value --------------------------------------------
pub struct Value<T> {
    id: u32,
    value: RwLock<T>,
    channel: Sender<WriteMessage>,
}

impl<T> Value<T>
where
    T: WriteValue + Clone,
{
    pub(crate) fn new(id: u32, value: T, channel: Sender<WriteMessage>) -> Arc<Self> {
        Arc::new(Self {
            id,
            value: RwLock::new(value),
            channel,
        })
    }

    pub fn get(&self) -> T {
        self.value.read().unwrap().clone()
    }

    pub fn set(&self, value: T, signal: bool) {
        let message = WriteMessage::Value(self.id, signal, value.clone().into_message());
        let mut w = self.value.write().unwrap();
        *w = value;
        self.channel.send(message).unwrap();
    }
}

impl<T: ReadValue> ValueUpdate for Value<T> {
    fn update_value(&self, head: &[u8], data: Option<Vec<u8>>) -> Result<(), String> {
        let value = T::read_message(head, data)
            .map_err(|e| format!("Parse error: {} for value id: {}", e, self.id))?;

        let mut w = self.value.write().unwrap();
        *w = value;
        self.channel.send(WriteMessage::ack(self.id)).unwrap();
        Ok(())
    }
}

// StaticValue --------------------------------------------
pub struct ValueStatic<T> {
    id: u32,
    value: RwLock<T>,
}

impl<T: Clone> ValueStatic<T> {
    pub(crate) fn new(id: u32, value: T) -> Arc<Self> {
        Arc::new(Self {
            id,
            value: RwLock::new(value),
        })
    }

    pub fn get(&self) -> T {
        self.value.read().unwrap().clone()
    }
}

impl<T: ReadValue> ValueUpdate for ValueStatic<T> {
    fn update_value(&self, head: &[u8], data: Option<Vec<u8>>) -> Result<(), String> {
        let value = T::read_message(head, data)
            .map_err(|e| format!("Parse error: {} for value id: {}", e, self.id))?;

        *self.value.write().unwrap() = value;
        Ok(())
    }
}

// ValueEnum --------------------------------------------
pub struct ValueEnum<T> {
    id: u32,
    value: RwLock<T>,
    channel: Sender<WriteMessage>,
}

impl<T: EnumInt> ValueEnum<T> {
    pub(crate) fn new(id: u32, value: T, channel: Sender<WriteMessage>) -> Arc<Self> {
        Arc::new(Self {
            id,
            value: RwLock::new(value),
            channel,
        })
    }

    pub fn get(&self) -> T {
        self.value.read().unwrap().clone()
    }

    pub fn set(&self, value: T, signal: bool) {
        let val = value.as_int();
        let message = WriteMessage::Value(self.id, signal, ValueMessage::U64(val));
        let mut w = self.value.write().unwrap();
        *w = value;
        self.channel.send(message).unwrap();
    }
}

impl<T: EnumInt> ValueUpdate for ValueEnum<T> {
    fn update_value(&self, head: &[u8], data: Option<Vec<u8>>) -> Result<(), String> {
        let int_val = u64::read_message(&head, data)?;
        let value = T::from_int(int_val)
            .map_err(|_| format!("Invalid enum format for enum id: {}", self.id))?;

        let mut w = self.value.write().unwrap();
        *w = value;
        self.channel.send(WriteMessage::ack(self.id)).unwrap();
        Ok(())
    }
}

// Signal --------------------------------------------
pub struct Signal<T> {
    id: u32,
    channel: Sender<WriteMessage>,
    phantom: PhantomData<T>,
}

impl<T: WriteValue + Clone> Signal<T> {
    pub(crate) fn new(id: u32, channel: Sender<WriteMessage>) -> Arc<Self> {
        Arc::new(Self {
            id,
            channel,
            phantom: PhantomData,
        })
    }

    pub fn set(&self, value: T) {
        let message = value.into_message();
        let message = WriteMessage::Signal(self.id, message);
        self.channel.send(message).unwrap();
    }
}
