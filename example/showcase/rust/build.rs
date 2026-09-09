fn main() {
    println!("cargo:rerun-if-changed=../gui-core/src");
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    println!("cargo:rerun-if-changed={}", out.join("bindings").display());
    egui_states::build_scripts::generate_rust::<showcase_gui_core::ShowcaseState>(
        out.join("bindings"),
    )
    .unwrap();
    std::fs::write(
        out.join("bindings_module.rs"),
        format!(
            "#[path = {:?}] pub mod bindings;",
            out.join("bindings/mod.rs")
        ),
    )
    .unwrap();
}
