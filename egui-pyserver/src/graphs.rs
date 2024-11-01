use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, RwLock};

use pyo3::buffer::{Element, PyBuffer};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyTuple};

use egui_pytransport::graphs::{Graph, GraphElement, GraphMessage, XAxis};
use egui_pytransport::nohash::NoHashMap;
use egui_pytransport::transport::WriteMessage;

use crate::SyncTrait;

pub(crate) trait PyGraph: Send + Sync {
    fn set_py(
        &self,
        idx: u16,
        object: &Bound<PyAny>,
        range: Option<Bound<PyAny>>,
        update: bool,
    ) -> PyResult<()>;
    fn add_points_py(
        &self,
        idx: u16,
        object: &Bound<PyAny>,
        range: Option<Bound<PyAny>>,
        update: bool,
    ) -> PyResult<()>;
    fn get_py<'py>(&self, py: Python<'py>, idx: u16) -> PyResult<PyObject>;
    fn len_py(&self, idx: u16) -> PyResult<usize>;
    fn remove_py(&self, idx: u16, update: bool);
    fn count_py(&self) -> u16;
    fn clear_py(&self, update: bool);
}

pub struct ValueGraphs<T> {
    id: u32,
    graphs: RwLock<NoHashMap<u16, Graph<T>>>,

    channel: Sender<WriteMessage>,
    connected: Arc<AtomicBool>,
}

impl<T> ValueGraphs<T> {
    pub(crate) fn new(
        id: u32,
        channel: Sender<WriteMessage>,
        connected: Arc<AtomicBool>,
    ) -> Arc<Self> {
        let graphs = RwLock::new(NoHashMap::default());

        Arc::new(Self {
            id,
            graphs,
            channel,
            connected,
        })
    }
}

impl<T> PyGraph for ValueGraphs<T>
where
    T: GraphElement + Element + for<'py> FromPyObject<'py> + ToPyObject,
{
    fn set_py(
        &self,
        idx: u16,
        object: &Bound<PyAny>,
        range: Option<Bound<PyAny>>,
        update: bool,
    ) -> PyResult<()> {
        let buffer = PyBuffer::<T>::extract_bound(object)?;
        let range = match range {
            Some(range) => Some(range.extract::<[T; 2]>()?),
            None => None,
        };
        let graph = buffer_to_graph(&buffer, range, None)?.unwrap();

        let mut w = self.graphs.write().unwrap();
        if self.connected.load(Ordering::Relaxed) {
            let graph_data = graph.to_graph_data(None);
            let message = GraphMessage::Set(idx, graph_data);
            self.channel
                .send(WriteMessage::Graph(self.id, update, Box::new(message)))
                .unwrap();
        }
        w.insert(idx, graph);
        Ok(())
    }

    fn add_points_py(
        &self,
        idx: u16,
        object: &Bound<PyAny>,
        range: Option<Bound<PyAny>>,
        update: bool,
    ) -> PyResult<()> {
        let buffer = PyBuffer::<T>::extract_bound(object)?;
        let range = match range {
            Some(range) => Some(range.extract::<[T; 2]>()?),
            None => None,
        };

        let mut w = self.graphs.write().unwrap();
        let graph = w
            .get_mut(&idx)
            .ok_or_else(|| PyValueError::new_err("Graph not found"))?;
        let _ = buffer_to_graph(&buffer, range, Some(graph));

        if self.connected.load(Ordering::Relaxed) {
            let message = GraphMessage::AddPoints(idx, graph.to_graph_data(None));
            self.channel
                .send(WriteMessage::Graph(self.id, update, Box::new(message)))
                .unwrap();
        }

        Ok(())
    }

    fn get_py<'py>(&self, py: Python<'py>, idx: u16) -> PyResult<PyObject> {
        let w = self.graphs.read().unwrap();
        let graph = w
            .get(&idx)
            .ok_or_else(|| PyValueError::new_err(format!("Graph with id {} not found", idx)))?;

        match graph.x {
            XAxis::X(ref x) => {
                let size = (x.len() + graph.y.len()) * size_of::<T>();
                let bytes = PyBytes::new_bound_with(py, size, |buf| {
                    let mut ptr = buf.as_mut_ptr() as *mut T;
                    unsafe {
                        std::ptr::copy_nonoverlapping(x.as_ptr(), ptr, x.len());
                        ptr = ptr.add(x.len());
                        std::ptr::copy_nonoverlapping(graph.y.as_ptr(), ptr, graph.y.len());
                    };
                    Ok(())
                })?;

                let shape = (2usize, graph.y.len(), size_of::<T>());
                Ok((bytes, shape, None::<Bound<PyTuple>>).to_object(py))
            }
            XAxis::Range(range) => {
                let size = graph.y.len() * size_of::<T>();
                let data =
                    unsafe { std::slice::from_raw_parts(graph.y.as_ptr() as *const u8, size) };
                let bytes = PyBytes::new_bound(py, data);
                let range = PyTuple::new_bound(py, [range[0], range[1]]);
                Ok((bytes, (graph.y.len(), size_of::<T>()), Some(range)).to_object(py))
            }
        }
    }

    fn len_py(&self, idx: u16) -> PyResult<usize> {
        let size = self
            .graphs
            .read()
            .unwrap()
            .get(&idx)
            .ok_or(PyValueError::new_err(format!(
                "Graph with id {} not found",
                idx
            )))?
            .y
            .len();

        Ok(size)
    }

    fn remove_py(&self, idx: u16, update: bool) {
        let mut w = self.graphs.write().unwrap();
        if self.connected.load(Ordering::Relaxed) {
            let message = GraphMessage::<T>::Remove(idx);
            self.channel
                .send(WriteMessage::Graph(self.id, update, Box::new(message)))
                .unwrap();
        }
        w.remove(&idx);
    }

    fn count_py(&self) -> u16 {
        self.graphs.read().unwrap().len() as u16
    }

    fn clear_py(&self, update: bool) {
        let mut w = self.graphs.write().unwrap();

        if self.connected.load(Ordering::Relaxed) {
            let message = GraphMessage::<T>::Reset;
            self.channel
                .send(WriteMessage::Graph(self.id, update, Box::new(message)))
                .unwrap();
        }
        w.clear();
    }
}

impl<T: GraphElement> SyncTrait for ValueGraphs<T> {
    fn sync(&self) {
        let w = self.graphs.read().unwrap();

        self.channel
            .send(WriteMessage::Graph(
                self.id,
                false,
                Box::new(GraphMessage::<T>::Reset),
            ))
            .unwrap();

        for (idx, graph) in w.iter() {
            let message = GraphMessage::Set(*idx, graph.to_graph_data(None));
            self.channel
                .send(WriteMessage::Graph(self.id, false, Box::new(message)))
                .unwrap();
        }
    }
}

fn buffer_to_graph_add<'py, T>(buffer: &PyBuffer<T>, range: Option<[T; 2]>, graph: &mut Graph<T>)
where
    T: GraphElement + Element + FromPyObject<'py>,
{
    
}

fn buffer_to_graph<'py, T>(
    buffer: &PyBuffer<T>,
    range: Option<[T; 2]>,
    graph: Option<&mut Graph<T>>,
) -> PyResult<Option<Graph<T>>>
where
    T: GraphElement + Element + FromPyObject<'py>,
{
    let shape = buffer.shape();

    let graph = match range {
        Some(range) => {
            if shape.len() != 1 {
                return Err(PyValueError::new_err(
                    "Graph data with range must have 1 dimension.",
                ));
            }

            let points = shape[0];
            let ptr = buffer.get_ptr(&[0]) as *const T;

            match graph {
                Some(graph) => {
                    if let XAxis::X(_) = graph.x {
                        return Err(PyValueError::new_err(
                            "Graph data with range must have the same x axis type.",
                        ));
                    }

                    let original_len = graph.y.len();
                    graph.y.resize(original_len + points, T::zero());
                    unsafe {
                        std::ptr::copy_nonoverlapping(
                            ptr,
                            graph.y[original_len..].as_mut_ptr(),
                            points,
                        )
                    };

                    None
                }
                None => {
                    if shape[0] < 2 {
                        return Err(PyValueError::new_err(
                            "Graph data with range must have at least 2 points.",
                        ));
                    }

                    let mut y = vec![T::zero(); points];
                    unsafe { std::ptr::copy_nonoverlapping(ptr, y.as_mut_ptr(), points) };

                    Some(Graph {
                        y,
                        x: XAxis::Range(range),
                    })
                }
            }
        }
        None => {
            if shape.len() != 2 {
                return Err(PyValueError::new_err("Graph data must have 2 dimensions."));
            }

            if shape[0] != 2 {
                return Err(PyValueError::new_err(
                    "Graph data must have at 2 lines (x, y).",
                ));
            }

            let points = shape[1];

            match graph {
                Some(graph) => {
                    match graph.x {
                        XAxis::X(ref mut x) => {
                            let original_len = x.len();
                            x.resize(points + original_len, T::zero());
                            let ptr = buffer.get_ptr(&[0, 0]) as *const T;
                            unsafe {
                                std::ptr::copy_nonoverlapping(
                                    ptr,
                                    x[original_len..].as_mut_ptr(),
                                    points,
                                )
                            };
                        }
                        XAxis::Range(_) => {
                            return Err(PyValueError::new_err(
                                "Graph data with range must have the same x axis type.",
                            ));
                        }
                    }

                    let original_len = graph.y.len();
                    graph.y.resize(points + original_len, T::zero());
                    let ptr = buffer.get_ptr(&[1, 0]) as *const T;
                    unsafe {
                        std::ptr::copy_nonoverlapping(
                            ptr,
                            graph.y[original_len..].as_mut_ptr(),
                            points,
                        )
                    };

                    None
                }
                None => {
                    if shape[1] < 2 {
                        return Err(PyValueError::new_err(
                            "Graph data must have at least 2 points.",
                        ));
                    }

                    let mut x = vec![T::zero(); points];
                    let ptr = buffer.get_ptr(&[0, 0]) as *const T;
                    unsafe { std::ptr::copy_nonoverlapping(ptr, x.as_mut_ptr(), points) };

                    let mut y = vec![T::zero(); points];
                    let ptr = buffer.get_ptr(&[1, 0]) as *const T;
                    unsafe { std::ptr::copy_nonoverlapping(ptr, y.as_mut_ptr(), points) };

                    Some(Graph { y, x: XAxis::X(x) })
                }
            }
        }
    };

    Ok(graph)
}
