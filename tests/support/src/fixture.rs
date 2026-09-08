//! Deterministic server setup for the integration-only fixture schema.
use crate::bindings::StatesServer;
use egui_states::server::{self as s, CallbackHandle};
use std::sync::{Arc, Mutex};

pub struct Fixture {
    pub server: StatesServer,
    callbacks: Vec<CallbackHandle>,
    errors: Arc<Mutex<Vec<String>>>,
}

impl Fixture {
    pub fn new(port: u16) -> s::Result<Self> {
        let errors = Arc::new(Mutex::new(Vec::new()));
        let on_error = errors.clone();
        let server = StatesServer::with_options(s::ServerOptions {
            error_handler: Some(Arc::new(move |error| {
                on_error.lock().unwrap().push(error.to_string())
            })),
            ..s::ServerOptions::default()
        })?;
        let states = &server.states.integration;
        states.value.set(7, false)?;
        states.items.set(vec![1, 2], false)?;
        states.cached.set(&[8, 9], false, false, true)?;
        states
            .cached_multi
            .set(91, &[500, 600], false, false, true)?;
        let on_error = errors.clone();
        let mut callbacks = vec![server.logging.add_logger(s::LogLevel::Error, move |error| {
            on_error.lock().unwrap().push(error.to_string())
        })];
        let callback_value = states.callback_value.clone();
        let on_error = errors.clone();
        callbacks.push(states.value.connect(move |value| {
            if let Err(e) = callback_value.set(value, true) {
                on_error.lock().unwrap().push(e.to_string());
            }
        }));
        let st = states.clone();
        let on_error = errors.clone();
        callbacks.push(states.command.connect(move |command| {
            let result = (|| -> s::Result<()> {
                match command {
                    1 => st.value.set(21, true)?,
                    2 => {
                        st.items.add_item(st.value.get()?, true)?;
                        st.map.set_item(9, 900, true)?;
                    }
                    3 => {
                        st.take.set("payload".into(), false, true)?;
                        st.empty.set((), false, true)?;
                        st.data.set(&[0, 1, 255], false, true, false)?;
                        st.multi.set(7, &[100, 200], false, true, false)?;
                        st.multi.set(42, &[], false, true, false)?;
                    }
                    4 => {
                        st.take.set(String::new(), false, true)?;
                        st.data.set(&[], false, true, false)?;
                    }
                    10..=13 => {
                        match command {
                            10 => st.take.set("first".into(), true, true)?,
                            11 => st.empty.set((), true, true)?,
                            12 => st.data.set(&[1, 2], true, true, false)?,
                            _ => st.multi.set(73, &[100, 200], true, true, false)?,
                        }
                        st.phase.set(1, true)?;
                        match command {
                            10 => st.take.set(String::new(), true, true)?,
                            11 => st.empty.set((), true, true)?,
                            12 => st.data.set(&[], true, true, false)?,
                            _ => st.multi.set(73, &[], true, true, false)?,
                        }
                        st.phase.set(2, true)?;
                    }
                    99 => st.phase.set(99, true)?,
                    _ => panic!("unknown fixture command"),
                }
                Ok(())
            })();
            if let Err(error) = result {
                on_error.lock().unwrap().push(error.to_string());
            }
        }));
        server.start(port, Some(std::net::Ipv4Addr::LOCALHOST), None)?;
        Ok(Self {
            server,
            callbacks,
            errors,
        })
    }

    pub fn assert_no_errors(&self) {
        assert!(self.errors.lock().unwrap().is_empty(), "{:?}", self.errors);
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.server.stop();
        self.callbacks.clear();
    }
}
