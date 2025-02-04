use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, RwLock};

use serde::Deserialize;

use crate::transport::{deserialize, MessageData};

#[derive(Deserialize)]
enum DictMessage<K, V>
where
    K: Eq + Hash,
{
    All(HashMap<K, V>),
    Set(K, V),
    Remove(K),
}

pub(crate) trait DictUpdate: Sync + Send {
    fn update_dict(&self, data: MessageData) -> Result<(), String>;
}

pub struct ValueDict<K, V> {
    _id: u32,
    dict: RwLock<HashMap<K, V>>,
}

impl<K, V> ValueDict<K, V>
where
    K: Clone + Hash + Eq,
    V: Clone,
{
    pub(crate) fn new(id: u32) -> Arc<Self> {
        Arc::new(Self {
            _id: id,
            dict: RwLock::new(HashMap::new()),
        })
    }

    #[inline]
    pub fn get(&self) -> HashMap<K, V> {
        self.dict.read().unwrap().clone()
    }

    #[inline]
    pub fn get_item(&self, key: &K) -> Option<V> {
        self.dict.read().unwrap().get(key).cloned()
    }

    pub fn process<R>(&self, op: impl Fn(&HashMap<K, V>) -> R) -> R {
        let d = self.dict.read().unwrap();
        op(&*d)
    }
}

impl<K, V> DictUpdate for ValueDict<K, V>
where
    K: for<'a> Deserialize<'a> + Eq + Hash + Send + Sync,
    V: for<'a> Deserialize<'a> + Send + Sync,
{
    fn update_dict(&self, data: MessageData) -> Result<(), String> {
        let message: DictMessage<K, V> = deserialize(data).map_err(|e| e.to_string())?;
        match message {
            DictMessage::All(dict) => {
                *self.dict.write().unwrap() = dict;
            }
            DictMessage::Set(key, value) => {
                self.dict.write().unwrap().insert(key, value);
            }
            DictMessage::Remove(key) => {
                self.dict.write().unwrap().remove(&key);
            }
        }
        Ok(())
    }
}

// SERVER ---------------------------------------------------
// ----------------------------------------------------------
#[cfg(feature = "server")]
pub(crate) mod server {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc::Sender;

    use pyo3::exceptions::PyKeyError;
    use pyo3::prelude::*;
    use pyo3::types::PyDict;
    use serde::Serialize;

    use crate::python_convert::ToPython;
    use crate::server::SyncTrait;
    use crate::transport::{serialize, WriteMessage};

    #[derive(Serialize)]
    enum DictMessageRef<'a, K, V>
    where
        K: Eq + Hash,
    {
        All(&'a HashMap<K, V>),
        Set(&'a K, &'a V),
        Remove(&'a K),
    }

    pub(crate) trait PyDictTrait: Send + Sync {
        fn get_py<'py>(&self, py: Python<'py>) -> Bound<'py, PyDict>;
        fn get_item_py<'py>(&self, key: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>>;
        fn set_py(&self, dict: &Bound<PyAny>, update: bool) -> PyResult<()>;
        fn set_item_py(
            &self,
            key: &Bound<PyAny>,
            value: &Bound<PyAny>,
            update: bool,
        ) -> PyResult<()>;
        fn del_item_py(&self, key: &Bound<PyAny>, update: bool) -> PyResult<()>;
        fn len_py(&self) -> usize;
    }

    pub(crate) struct PyValueDict<K, V> {
        id: u32,
        dict: RwLock<HashMap<K, V>>,
        channel: Sender<WriteMessage>,
        connected: Arc<AtomicBool>,
    }

    impl<K, V> PyValueDict<K, V> {
        pub(crate) fn new(
            id: u32,
            channel: Sender<WriteMessage>,
            connected: Arc<AtomicBool>,
        ) -> Arc<Self> {
            Arc::new(Self {
                id,
                dict: RwLock::new(HashMap::new()),
                channel,
                connected,
            })
        }
    }

    impl<K, V> PyDictTrait for PyValueDict<K, V>
    where
        K: Serialize + ToPython + for<'py> FromPyObject<'py> + Eq + Hash,
        V: Serialize + ToPython + for<'py> FromPyObject<'py>,
    {
        fn get_py<'py>(&self, py: Python<'py>) -> Bound<'py, PyDict> {
            let dict = self.dict.read().unwrap();

            let py_dict = pyo3::types::PyDict::new(py);
            for (key, value) in dict.iter() {
                let key = key.to_python(py);
                let value = value.to_python(py);
                py_dict.set_item(key, value).unwrap();
            }
            py_dict
        }

        fn get_item_py<'py>(&self, key: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
            let dict_key = key.extract()?;
            let dict = self.dict.read().unwrap();

            match dict.get(&dict_key) {
                Some(value) => Ok(value.to_python(key.py())),
                None => Err(PyKeyError::new_err("Key not found.")),
            }
        }

        fn del_item_py(&self, key: &Bound<PyAny>, update: bool) -> PyResult<()> {
            let dict_key: K = key.extract()?;

            let mut d = self.dict.write().unwrap();
            if self.connected.load(Ordering::Relaxed) {
                let data = serialize(DictMessageRef::Remove::<K, V>(&dict_key));
                let message = WriteMessage::Dict(self.id, update, data);
                self.channel.send(message).unwrap();
            }
            d.remove(&dict_key);

            Ok(())
        }

        fn set_item_py(
            &self,
            key: &Bound<PyAny>,
            value: &Bound<PyAny>,
            update: bool,
        ) -> PyResult<()> {
            let dict_key: K = key.extract()?;
            let dict_value: V = value.extract()?;

            let mut d = self.dict.write().unwrap();

            if self.connected.load(Ordering::Relaxed) {
                let data = serialize(DictMessageRef::Set::<K, V>(&dict_key, &dict_value));
                let message = WriteMessage::Dict(self.id, update, data);
                self.channel.send(message).unwrap();
            }

            d.insert(dict_key, dict_value);
            Ok(())
        }

        fn set_py(&self, dict: &Bound<PyAny>, update: bool) -> PyResult<()> {
            let dict = dict.downcast::<pyo3::types::PyDict>()?;
            let mut new_dict = HashMap::new();

            for (key, value) in dict {
                let key = key.extract()?;
                let value = value.extract()?;
                new_dict.insert(key, value);
            }

            let mut d = self.dict.write().unwrap();

            if self.connected.load(Ordering::Relaxed) {
                dict.py().allow_threads(|| {
                    let data = serialize(DictMessageRef::All(&new_dict));
                    let message = WriteMessage::Dict(self.id, update, data);
                    self.channel.send(message).unwrap();
                });
            }

            *d = new_dict;

            Ok(())
        }

        fn len_py(&self) -> usize {
            self.dict.read().unwrap().len()
        }
    }

    impl<K, V> SyncTrait for PyValueDict<K, V>
    where
        K: Serialize + Send + Sync + Eq + Hash,
        V: Serialize + Send + Sync,
    {
        fn sync(&self) {
            let dict = self.dict.read().unwrap();
            let data = serialize(DictMessageRef::All(&dict));
            let message = WriteMessage::Dict(self.id, false, data);
            self.channel.send(message).unwrap();
        }
    }
}
