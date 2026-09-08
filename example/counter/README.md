# Counter

This is the smallest example of a GUI connected to a Python or Rust server.
The whole synchronized state definition is in [gui/src/lib.rs](gui/src/lib.rs):
`count: Value<i32>` and `reset: Signal<()>`. GUI edits synchronize the count and
invoke a server logging callback. Reset asks the server to restore zero.

## Run

Install Rust and `uv`, then run `uv sync` from the repository root. Run the
following commands from this directory (`example/counter`).

Choose one server:

```sh
# Python: build the GUI to generate bindings, then start the server.
cargo build -p counter_gui
uv run python python/run.py

# Or Rust (bindings are generated automatically):
cargo run -p counter_server --bin counter-server
```

In another terminal, also in this directory:

```sh
cargo run -p counter_gui --bin counter-gui
```

Click **Connect**, edit the number, and watch the server print the new count.
Click **Reset** to return to zero. Close the GUI and press Ctrl+C in the server
terminal to stop it and release its socket. Stop one server before switching to
the other language; both work with the same GUI on loopback port **8092**.
The server commands accept `--port PORT`; the GUI uses 8092. This example has a
native GUI only.

## Folders

- `gui/`: a library target defining the synchronized state, the native GUI binary
  target, and the Python binding build script.
- `python/`: the Python server entry point and generated `counter_bindings/` package.
- `rust/`: the Rust server setup, entry point, and Rust binding build script.

## State sharing through the UI library

The `counter_gui` package contains a library target, a GUI binary target, and
a build script. Its library crate defines `CounterState` directly in `src/lib.rs`.
The GUI binary crate imports `counter_gui::CounterState` from that library.

The UI package's own build script runs before its library crate can be compiled,
so it still loads the library source directly as a module named `state`:

```rust
#[path = "src/lib.rs"]
mod state;
```

The library crate and build-script crate compile the same state source. State
dependencies are declared under both `[dependencies]` and `[build-dependencies]`,
with the `egui_states/build_scripts` feature enabled for generation. The state
definition itself does not use eframe.

Building the GUI invokes `generate_python::<state::CounterState>` and writes
`python/counter_bindings/`. The `counter_server` package declares `counter_gui`
under `[build-dependencies]`. Its build script imports
`counter_gui::CounterState` and invokes `generate_rust`, writing into its
own Cargo `OUT_DIR`. Cargo tracks the library dependency and regenerates Rust
bindings when the state definition changes.

Building the Rust server directly also builds the UI library and runs its build
script, so Python bindings are generated as well. No separate GUI build command
is required for the Rust server. This also compiles the UI package's existing
dependencies, including eframe, for the host; it does not run the GUI binary.
The example keeps those dependencies unconditional for simplicity.

Python bindings are ignored by Git. Change `gui/src/lib.rs` and rebuild the
GUI package to regenerate them; `cargo build -p counter_gui` also regenerates
bindings after their output directory is removed. Do not edit generated files.
The UI build script demonstrates
[approach 1](../../README.md#1-load-the-uis-state-module-into-the-build-script-crate)
from the main guide, while the Rust server and smoke-test probe use the UI
library crate through ordinary package dependencies.

The example uses the repository's workspace dependencies and version settings.
It has no dependency on the showcase.

## Smoke tests

From the repository root:

```sh
uv run pytest tests/examples -k counter
cargo test -p example_smoke_tests counter_setup_and_client
```

These exercise count edits, reset, server startup, and shutdown using the actual
setup functions and a native client without opening a window. The shared test
harness builds both GUIs to generate their Python bindings; normal counter
builds prepare only this example.
