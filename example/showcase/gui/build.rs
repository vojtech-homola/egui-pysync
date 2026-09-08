use showcase_gui_core::ShowcaseState;

fn main() {
    println!("cargo:rerun-if-changed=../gui-core/src");
    println!("cargo:rerun-if-changed=../python/showcase_bindings");
    egui_states::build_scripts::generate_python::<ShowcaseState>("../python/showcase_bindings")
        .unwrap();
}
