# Showcase

This example demonstrates values, static values, signals, custom enums and
structs, vectors and maps, arrays, take transfers, and single and sparse images.
The native and browser GUIs share one application. Python and Rust servers
populate the same deterministic data and implement the same actions.

## Run

Install Rust and `uv`, then run `uv sync` from the repository root. Run the
following commands from this directory (`example/showcase`).

Choose one server:

```sh
# Python: build the GUI to generate bindings, then start the server.
cargo build -p showcase_gui
uv run python python/run.py

# Or Rust (bindings are generated automatically):
cargo run -p showcase_server --bin showcase-server
```

Run the native GUI in another terminal, also in this directory:

```sh
cargo run -p showcase_gui --bin showcase-gui
```

Click **Connect** and expand the feature sections. Edit values and emit signals
while watching the server output. Collection buttons ask the server to mutate
the collection. Both servers seed `[10, -3, 27]` and the same map; appending to an
empty vector inserts `10`. Images show a fixed gradient with a green patch.
Take sections display the last consumed payload; cached data transfers are also
available to newly connected clients.

The server uses loopback port **8091**. Stop one server before switching to the
other language. Close the GUI and press Ctrl+C in the server terminal when done.
Both server commands accept `--port PORT`; the GUI uses 8091.

## Browser GUI

Install Trunk and the WASM target. From the repository root:

```sh
rustup target add wasm32-unknown-unknown
trunk serve
```

Open http://localhost:8090 and click **Connect** while either server is running.
The root `Trunk.toml` points to this example's web entry point and watches this
example's application library. Stop Trunk and the server with Ctrl+C when finished.

## Folders

- `gui-core/`: application library containing state definitions, custom types,
  feature rendering, GUI-only caches, and editable signal inputs.
- `gui/`: native/browser startup, web assets, and automatic Python binding generation.
- `python/`: Python callbacks, initial data, setup, and execution in `run.py`.
- `rust/`: Rust setup and execution, with callbacks and initial data in separate modules.

## Application library and launcher packages

The `showcase_gui_core` package's library crate exports `ShowcaseState` from
[src/state/mod.rs](gui-core/src/state/mod.rs) and `MainApp` from its rendering
code. The `showcase_gui` package contains the GUI binary target and the Python
binding build script. Its manifest declares `showcase_gui_core` under both
`[dependencies]` and `[build-dependencies]`.

The launcher's binary crate imports `MainApp`; its build-script crate imports
`ShowcaseState` and calls `generate_python::<ShowcaseState>`. The application
library does not depend on the launcher, so the build has no dependency cycle.
The Rust server's build script imports `ShowcaseState` from the same library and
calls `generate_rust`.

This demonstrates [approach 3](../../README.md#3-separate-the-application-library-and-launcher-packages)
from the main guide. Generation compiles the application library, including its
eframe dependency, for the host. Building the WASM GUI also compiles that library
for the browser target. The build scripts never create a GUI window.

Generated Python bindings go in `python/showcase_bindings/` and are ignored by
Git. Rust bindings live in the server package's Cargo `OUT_DIR`. Change state
definitions in `gui-core/src/state/` and rebuild to regenerate bindings.
`cargo build -p showcase_gui` also regenerates Python bindings after their output
directory is removed. Do not edit generated files.

The example uses the repository's workspace dependencies and version settings.
It has no dependency on the counter.

## Smoke tests

From the repository root:

```sh
uv run pytest tests/examples -k showcase
cargo test -p example_smoke_tests showcase_setup_and_client
```

These exercise actual setup functions, collection defaults and actions, client
edits, representative data and images, and server startup/shutdown. The native
probe runs without opening a window. The shared test harness builds both GUIs
to generate their Python bindings; normal showcase builds prepare only this
example.
