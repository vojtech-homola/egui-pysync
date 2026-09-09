# Tests

Install Rust and `uv`, then run `uv sync`. Run all commands from the repository
root. Python tests build their native helpers and generate bindings automatically.

## Library tests

```sh
uv run pytest
cargo test -p egui_states --features 'server build_scripts' -p egui_states_macros
cargo test -p egui_states_test_support
```

The default Python suite lives in [python/](python/). It shares the test-owned
[schema](schema/src/lib.rs) and [native scenarios](support/src/scenarios.rs) with
Rust integration tests, independently of the examples and eframe.

## Example smoke tests

```sh
uv run pytest tests/examples
cargo test -p example_smoke_tests
```

These opt-in suites exercise the counter and showcase servers with headless
clients. The Python suite also checks server startup and shutdown commands.
See [examples/](examples/) and [example-support/](example-support/) for coverage.

## Build and visual checks

```sh
cargo build -p showcase_gui -p counter_gui -p showcase_server -p counter_server
rustup target add wasm32-unknown-unknown
cargo build -p showcase_gui --target wasm32-unknown-unknown
trunk build
```

The browser checks require Trunk. Use the [example guides](../example/README.md)
to check the GUIs manually: connect, edit values, emit signals, and inspect
collections and images. Headless tests do not check rendering.
