// Compile the GUI's state source into the build-script crate as well.
#[path = "src/lib.rs"]
#[allow(dead_code)]
mod state;

fn main() {
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=../python/counter_bindings");
    egui_states::build_scripts::generate_python::<state::CounterState>(
        "../python/counter_bindings",
    )
    .unwrap();
}
