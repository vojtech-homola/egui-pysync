//! The complete state shared by the counter GUI and either server.
use egui_states::{Signal, Value};

#[derive(egui_states::State)]
pub struct CounterState {
    pub count: Value<i32>,
    pub reset: Signal<()>,
}
