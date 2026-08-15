#![cfg(feature = "build_scripts")]

use std::collections::HashMap;
use std::path::PathBuf;

use egui_states::build_scripts::{generate_python, generate_rust};
use egui_states::{State, StatesCreator, Value};

#[egui_states::typed(rust_derive(Debug, PartialEq, Eq, Hash))]
#[derive(Clone, Default, PartialEq, Eq, Hash, egui_states::InitialValue)]
struct GeneratedInner {
    enabled: bool,
}

#[egui_states::typed(rust_derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    Hash,
    egui_states::serde::Serialize,
    egui_states::Typed,
    custom_macros::Debug
))]
#[derive(Clone, Default, PartialEq, Eq, Hash, egui_states::InitialValue)]
struct GeneratedOuter {
    inner: Option<GeneratedInner>,
}

#[egui_states::typed(rust_derive(Debug, PartialOrd, Ord))]
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, egui_states::InitialValue)]
enum GeneratedEnum {
    #[default]
    First,
    Second,
}

mod first_collision {
    #[egui_states::typed(rust_derive(Debug))]
    #[derive(Clone, Default, egui_states::InitialValue)]
    pub(super) struct Collision {
        pub(super) value: bool,
    }
}

mod second_collision {
    #[egui_states::typed(rust_derive(Hash))]
    #[derive(Clone, Default, egui_states::InitialValue)]
    pub(super) struct Collision {
        pub(super) value: bool,
    }
}

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
        let _: Value<Option<GeneratedOuter>> = c.value(
            "custom",
            Some(GeneratedOuter {
                inner: Some(GeneratedInner { enabled: true }),
            }),
        );
        let _: Value<GeneratedEnum> = c.value("custom_enum", GeneratedEnum::First);
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

#[test]
fn requested_rust_derives_are_emitted_for_nested_types() {
    let dir = output_dir("rust_derives", 0);
    generate_rust::<Root>(&dir).unwrap();
    let structs = std::fs::read_to_string(dir.join("structs.rs")).unwrap();
    let enums = std::fs::read_to_string(dir.join("enums.rs")).unwrap();

    assert!(
        structs.contains(
            "#[derive(Clone, Debug, PartialEq, Eq, Hash, custom_macros :: Debug)]\npub struct GeneratedOuter"
        )
    );
    assert!(
        structs.contains("#[derive(Clone, Debug, PartialEq, Eq, Hash)]\npub struct GeneratedInner")
    );
    assert_eq!(structs.matches("Clone, Clone").count(), 0);
    assert!(!structs.contains("Serialize"));
    assert!(!structs.contains("Typed)]"));
    assert!(enums.contains(
        "#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]\npub enum GeneratedEnum"
    ));

    let _ = std::fs::remove_dir_all(&dir);
}

struct ConflictingDerives;

impl State for ConflictingDerives {
    const NAME: &'static str = "ConflictingDerives";

    fn new(c: &mut impl StatesCreator) -> Self {
        let _: Value<first_collision::Collision> = c.value("first", Default::default());
        let _: Value<second_collision::Collision> = c.value("second", Default::default());
        Self
    }
}

#[test]
#[should_panic(
    expected = "struct Collision declared with inconsistent Rust derives: [\"Debug\"] vs [\"Hash\"]"
)]
fn inconsistent_rust_derives_for_one_generated_name_are_rejected() {
    let dir = output_dir("conflicting_rust_derives", 0);
    let _ = generate_rust::<ConflictingDerives>(&dir);
}

#[test]
fn inconsistent_rust_derives_do_not_affect_python_generation() {
    let dir = output_dir("python_conflicting_rust_derives", 0);
    generate_python::<ConflictingDerives>(&dir).unwrap();

    assert!(dir.join("structs.py").is_file());
    let _ = std::fs::remove_dir_all(&dir);
}
