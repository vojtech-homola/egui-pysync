# Running the examples

The examples share the state definition and egui application in `gui-core`.
Building `gui` generates Python bindings in `python/states_server`; building
`rust_server_example` generates native Rust bindings in
`rust/src/states_server`. Generated files are build artifacts—change the state
definition and rebuild instead of editing them by hand.

All variants use WebSocket port `8091`. Start one server and one client.

## Native GUI with the Python server

From the repository root, build or run the GUI once to generate the Python
package:

```sh
cargo run -p gui --bin GuiTest
```

In another terminal, start the Python server:

```sh
uv run python example/python/run.py
```

Use the Connect button in the GUI to establish the connection.

## Native GUI with the Rust server

Start the generated Rust server:

```sh
cargo run -p rust_server_example --bin RustServerExample
```

Then run the GUI in another terminal:

```sh
cargo run -p gui --bin GuiTest
```

## WebAssembly GUI

Install Trunk and the Rust WASM target, then run this command from the
repository root:

```sh
rustup target add wasm32-unknown-unknown
trunk serve
```

Open the URL printed by Trunk (configured as port `8090`) and run either server
above on port `8091`. The browser client currently connects to that fixed port.

## Python tests

After building the extension module, run:

```sh
uv run pytest
```
