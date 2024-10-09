use std::sync::Arc;

use egui_pysync_client;
use egui_pysync_server;

struct States {
    value_int64: Arc<egui_pysync_client::Value<i64>>,
}

impl States {
    pub fn new(c: &mut egui_pysync_client::ValuesCreator) -> Self {
        Self {
            value_int64: c.add_value(0),
        }
    }
}

fn create_values(c: &mut egui_pysync_server::ValuesCreator) {
    c.add_value(0i64);
}

#[test]
fn fake() {
    assert!(true);
}
