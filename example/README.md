# Examples

Each example owns its state definitions, GUI, servers, and run instructions.
Both remain members of the repository's Cargo workspace and use the library from
`crates/egui-states`; neither example depends on the other.

- [Counter](counter/README.md): start here. A small native GUI with a count and a
  reset signal, backed by either a Python or Rust server. Its UI library defines
  `CounterState` for the GUI binary and Rust server's build script. The UI's own
  build script uses `#[path]` to load the library source and generate Python bindings.
- [Showcase](showcase/README.md): explore all state kinds, collections, arrays,
  take transfers, and images through a native or browser GUI. Its application
  library contains state and rendering; the launcher package's build script
  imports that library to generate Python bindings.

Library tests have an independent schema under `tests/schema`. Run them from the
repository root with `uv run pytest`. Example smoke tests are opt-in:
`uv run pytest tests/examples` and `cargo test -p example_smoke_tests`.
See [the test guide](../tests/README.md) for coverage and validation commands.
