#![cfg(feature = "build_scripts")]

use std::collections::HashMap;
use std::path::PathBuf;

use egui_states::build_scripts::{generate_python, generate_rust};
use egui_states::{Image, ImageMulti, State, StatesCreator, Value};

/// A state class holding map-valued states, used twice by [`Root`].
struct Leaf;

impl State for Leaf {
    const NAME: &'static str = "Leaf";

    fn new(c: &mut impl StatesCreator) -> Self {
        let _: Value<HashMap<u16, u32>> = c.value(
            "numbers",
            HashMap::from([(10, 10), (1, 1), (20, 20), (2, 2), (3, 3)]),
        );
        let _: Value<HashMap<String, u8>> = c.value(
            "labels",
            HashMap::from([
                ("b".to_string(), 1),
                ("a".to_string(), 2),
                ("c".to_string(), 3),
            ]),
        );
        Self
    }
}

struct Root;

impl State for Root {
    const NAME: &'static str = "Root";

    fn new(c: &mut impl StatesCreator) -> Self {
        let _: Leaf = c.substate("first");
        let _: Leaf = c.substate("second");
        let _: Image = c.image("image");
        let _: ImageMulti = c.image_multi("images");
        Self
    }
}

struct ImageRoot;

impl State for ImageRoot {
    const NAME: &'static str = "KindRoot";

    fn new(c: &mut impl StatesCreator) -> Self {
        let _: Image = c.image("value");
        Self
    }
}

struct ImageMultiRoot;

impl State for ImageMultiRoot {
    const NAME: &'static str = "KindRoot";

    fn new(c: &mut impl StatesCreator) -> Self {
        let _: ImageMulti = c.image_multi("value");
        Self
    }
}

fn output_dir(name: &str, run: usize) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "egui_states_codegen_{}_{name}_{run}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn generate(dir: &PathBuf) -> (String, String) {
    generate_rust::<Root>(dir).unwrap();
    generate_python::<Root>(dir.join("python")).unwrap();
    (
        std::fs::read_to_string(dir.join("mod.rs")).unwrap(),
        std::fs::read_to_string(dir.join("python/__init__.py")).unwrap(),
    )
}

/// A state class reused as a substate must validate, even when it holds a map
/// initial value: `HashMap` iteration order used to make the two instances
/// compare unequal, failing with "defined multiple times with different fields".
#[test]
fn a_state_class_holding_a_map_can_be_reused() {
    let dir = output_dir("reuse", 0);
    let (rust, python) = generate(&dir);

    assert!(rust.contains("pub struct Leaf"));
    assert!(rust.contains("pub first: Leaf"));
    assert!(rust.contains("pub second: Leaf"));
    assert!(python.contains("class Leaf(ISubStates)"));
    assert!(rust.contains("pub image: s::Image"));
    assert!(rust.contains("pub images: s::ImageMulti"));
    assert!(python.contains("self.image: s.Image = s.Image()"));
    assert!(python.contains("self.images: s.ImageMulti = s.ImageMulti()"));

    let _ = std::fs::remove_dir_all(&dir);
}

/// Generated bindings must be byte-identical between runs so that repeated
/// builds do not rewrite the files.
#[test]
fn generated_bindings_are_stable_across_runs() {
    let mut expected: Option<(String, String)> = None;

    for run in 0..16 {
        let dir = output_dir("stable", run);
        let generated = generate(&dir);
        let _ = std::fs::remove_dir_all(&dir);

        match &expected {
            None => expected = Some(generated),
            Some(expected) => assert_eq!(
                *expected, generated,
                "generated bindings changed between runs"
            ),
        }
    }
}

/// Both generators emit map entries ordered by key.
#[test]
fn map_initial_values_are_ordered_by_key() {
    let dir = output_dir("order", 0);
    let (rust, python) = generate(&dir);

    assert!(
        rust.contains("[(1u16, 1u32), (2u16, 2u32), (3u16, 3u32), (10u16, 10u32), (20u16, 20u32)]"),
        "unexpected Rust map ordering:\n{rust}"
    );
    assert!(
        python.contains("{1: 1, 2: 2, 3: 3, 10: 10, 20: 20}"),
        "unexpected Python map ordering:\n{python}"
    );
    assert!(
        python.contains(r#"{"a": 2, "b": 1, "c": 3}"#),
        "unexpected Python string-key ordering:\n{python}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

fn generated_hash(output: &str, marker: &str) -> u64 {
    let start = output.find(marker).expect("generated version hash marker") + marker.len();
    output[start..]
        .split(|character: char| !character.is_ascii_digit())
        .next()
        .expect("generated version hash")
        .parse()
        .expect("numeric generated version hash")
}

#[test]
fn image_multi_codegen_and_layout_hash_are_distinct() {
    let image_dir = output_dir("image_kind", 0);
    let multi_dir = output_dir("image_multi_kind", 0);

    generate_rust::<ImageRoot>(&image_dir).unwrap();
    generate_python::<ImageRoot>(image_dir.join("python")).unwrap();
    generate_rust::<ImageMultiRoot>(&multi_dir).unwrap();
    generate_python::<ImageMultiRoot>(multi_dir.join("python")).unwrap();

    let image_rust = std::fs::read_to_string(image_dir.join("mod.rs")).unwrap();
    let image_python = std::fs::read_to_string(image_dir.join("python/__init__.py")).unwrap();
    let multi_rust = std::fs::read_to_string(multi_dir.join("mod.rs")).unwrap();
    let multi_python = std::fs::read_to_string(multi_dir.join("python/__init__.py")).unwrap();

    let image_hash = generated_hash(&image_rust, "pub const VERSION_HASH: u64 = ");
    let multi_hash = generated_hash(&multi_rust, "pub const VERSION_HASH: u64 = ");
    assert_eq!(
        image_hash,
        generated_hash(&image_python, "VERSION_HASH: int = ")
    );
    assert_eq!(
        multi_hash,
        generated_hash(&multi_python, "VERSION_HASH: int = ")
    );
    assert_ne!(image_hash, multi_hash);
    assert!(multi_rust.contains("pub value: s::ImageMulti"));
    assert!(multi_python.contains("self.value: s.ImageMulti = s.ImageMulti()"));

    let _ = std::fs::remove_dir_all(&image_dir);
    let _ = std::fs::remove_dir_all(&multi_dir);
}
