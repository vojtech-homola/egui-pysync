use std::collections::HashMap;
use std::hash::Hash;
use std::sync::mpsc::Sender;
use std::sync::Arc;

use egui_pysync_common::collections::ItemWriteRead;
use egui_pysync_common::values::{ReadValue, WriteValue};
use egui_pysync_common::EnumInt;

use crate::dict::{DictUpdate, ValueDict};
use crate::graphs::{GraphType, GraphUpdate, ValueGraph};
use crate::image::{ImageUpdate, ImageValue};
use crate::list::{ListUpdate, ValueList};
use crate::transport::WriteMessage;
use crate::values::{Signal, Value, ValueEnum, ValueStatic, ValueUpdate};

#[derive(Clone)]
pub(crate) struct ValuesList {
    pub(crate) values: HashMap<u32, Arc<dyn ValueUpdate>>,
    pub(crate) static_values: HashMap<u32, Arc<dyn ValueUpdate>>,
    pub(crate) images: HashMap<u32, Arc<dyn ImageUpdate>>,
    pub(crate) dicts: HashMap<u32, Arc<dyn DictUpdate>>,
    pub(crate) lists: HashMap<u32, Arc<dyn ListUpdate>>,
    pub(crate) graphs: HashMap<u32, Arc<dyn GraphUpdate>>,
}

impl ValuesList {
    fn new() -> Self {
        Self {
            values: HashMap::new(),
            static_values: HashMap::new(),
            images: HashMap::new(),
            dicts: HashMap::new(),
            lists: HashMap::new(),
            graphs: HashMap::new(),
        }
    }

    fn shrink(&mut self) {
        self.values.shrink_to_fit();
        self.static_values.shrink_to_fit();
        self.images.shrink_to_fit();
        self.dicts.shrink_to_fit();
        self.lists.shrink_to_fit();
        self.graphs.shrink_to_fit();
    }
}

pub struct ValuesCreator {
    counter: u32,
    val: ValuesList,
    channel: Sender<WriteMessage>,
}

impl ValuesCreator {
    pub(crate) fn new(channel: Sender<WriteMessage>) -> Self {
        Self {
            counter: 10, // first 10 values are reserved for special values
            val: ValuesList::new(),
            channel,
        }
    }

    fn get_id(&mut self) -> u32 {
        let count = self.counter;
        self.counter += 1;
        count
    }

    pub(crate) fn get_values(self) -> ValuesList {
        let mut val = self.val;
        val.shrink();
        val
    }

    pub fn add_value<T>(&mut self, value: T) -> Arc<Value<T>>
    where
        T: WriteValue + ReadValue + 'static,
    {
        let id = self.get_id();
        let value = Value::new(id, value, self.channel.clone());

        self.val.values.insert(id, value.clone());
        value
    }

    pub fn add_static_value<T>(&mut self, value: T) -> Arc<ValueStatic<T>>
    where
        T: ReadValue + 'static,
    {
        let id = self.get_id();
        let value = ValueStatic::new(id, value);

        self.val.static_values.insert(id, value.clone());
        value
    }

    pub fn add_image(&mut self) -> Arc<ImageValue> {
        let id = self.get_id();
        let value = ImageValue::new(id);

        self.val.images.insert(id, value.clone());
        value
    }

    pub fn add_enum<T: EnumInt + 'static>(&mut self, value: T) -> Arc<ValueEnum<T>> {
        let id = self.get_id();
        let value = ValueEnum::new(id, value, self.channel.clone());

        self.val.values.insert(id, value.clone());
        value
    }

    pub fn add_signal<T: WriteValue + Clone + 'static>(&mut self) -> Arc<Signal<T>> {
        let id = self.get_id();
        let signal = Signal::new(id, self.channel.clone());

        signal
    }

    pub fn add_dict<K, V>(&mut self) -> Arc<ValueDict<K, V>>
    where
        K: ItemWriteRead + Hash + Eq,
        V: ItemWriteRead,
    {
        let id = self.get_id();
        let value = ValueDict::new(id);

        self.val.dicts.insert(id, value.clone());
        value
    }

    pub fn add_list<T>(&mut self) -> Arc<ValueList<T>>
    where
        T: ItemWriteRead,
    {
        let id = self.get_id();
        let value = ValueList::new(id);

        self.val.lists.insert(id, value.clone());
        value
    }

    pub fn add_graph<T: GraphType + 'static>(&mut self) -> Arc<ValueGraph<T>> {
        let id = self.get_id();
        let value = ValueGraph::new(id);

        self.val.graphs.insert(id, value.clone());
        value
    }
}
