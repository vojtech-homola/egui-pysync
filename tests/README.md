# Test suites

`uv run pytest` collects only `tests/python`. Its conftest builds `support` before
collection and regenerates `generated/egui_states_test_bindings` every session,
including when Cargo considers the build fresh. Preparation errors fail the run
with captured diagnostics. `cargo build -p egui_states_test_support --bins` builds
the test client and generator; `cargo run -p egui_states_test_support --bin
prepare-test-bindings` explicitly prepares the Python package. The schema in
`schema` is test-owned and must not import example types or eframe.

The behavioral tests preserve the previous value, callback, lifecycle, array,
image, and take-method coverage. Collection API tests are independent of example
actions. Unexpected callback failures fail teardown; the intentional error-handler
test owns its separate server.

`support/src/scenarios.rs` contains shared native client assertions used by
`cargo test -p egui_states_test_support` and the Python subprocess probe tests:

- Initial synchronization, server updates, client edits, callback results and collection signals.
- ValueTake, DataTake and sparse DataMultiTake payloads, including empty payloads and single consumption.
- A subsequent blocking send remaining pending until consumption acknowledges the first payload.
- Cached data and sparse cached data delivered again to a newly connected client.

Scenarios use ephemeral loopback ports, condition deadlines, a process watchdog,
and captured subprocess output. The only fixed-duration observation is the negative
assertion that a blocking send stays pending before the first take. Client cleanup
waits for disconnection; fixtures stop servers even on assertion failures.

`uv run pytest tests/examples` and `cargo test -p example_smoke_tests` exercise the
real showcase and counter setup functions, using a separate headless example
probe. They verify collection defaults/actions, append-to-empty, counter edits and
reset, representative data and images, and server cleanup. Separate subprocess checks start all four documented server
commands, wait for readiness, send Ctrl+C, and verify the port is released (POSIX).
These tests explicitly depend on examples and are not part of default Python
collection. Their session setup builds both GUI packages to run the Python
binding build scripts; the tests themselves use headless probes.

## Acceptance commands

```sh
uv run pytest
cargo test -p egui_states --features 'server build_scripts' -p egui_states_macros
cargo test -p egui_states_test_support -p example_smoke_tests
uv run pytest tests/examples
cargo build -p showcase_gui -p counter_gui -p showcase_server -p counter_server
cargo build -p showcase_gui --target wasm32-unknown-unknown
trunk build
```

To check independence, remove only generated test/example binding directories,
then run `uv run pytest` with a fresh `CARGO_TARGET_DIR`. Example bindings must
remain absent and `cargo tree -p egui_states_test_support` must contain no eframe.
Rebuild each GUI package and compare tracked source diffs. Its Python binding
build script must only write its own ignored Python package:

```sh
cargo build -p counter_gui
cargo build -p showcase_gui
```

The example probe imports `CounterState` from the `counter_gui` library crate
and `ShowcaseState` from the `showcase_gui_core` library crate. The library test
harness remains independent of both examples and eframe.
Generated Rust files live under `OUT_DIR`.

Manually check both native GUIs and the browser showcase: connect, edit a value,
emit a signal, exercise collection controls, inspect images and cached take data,
then close the GUI and interrupt the server. Headless smoke tests complement this
visual check; they do not replace it.

Existing library bugs discovered by these tests are to be reported separately,
without changing the library implementation in this refactor.
