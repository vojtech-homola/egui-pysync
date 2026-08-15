# egui-states

[![crates.io](https://img.shields.io/crates/v/egui_states.svg)](https://crates.io/crates/egui_states)
[![docs.rs](https://docs.rs/egui_states/badge.svg)](https://docs.rs/egui_states)

`egui-states` synchronizes typed application state between an
[`egui`](https://github.com/emilk/egui) UI and a server. The UI is the Rust
client; the server can be written in Python or Rust. Both native and WebAssembly
egui clients use the same state API and communicate with the server over a
WebSocket connection.

The project is useful when an egui application is primarily a view and control
surface for work performed elsewhere—for example, a Python data-processing
application, an instrument controller, or a separate Rust service. Application
code reads and writes typed state handles instead of defining its own messages,
serialization, and synchronization logic.

## How it works

1. Define the UI's state tree as Rust structs whose fields are `egui-states`
   handles.
2. Derive `State` so the client can construct the tree and calculate its stable
   layout hash.
3. Generate matching Python or Rust server bindings from that same Rust type in
   a build script.
4. Start the server and connect the egui client. During the handshake, both
   peers compare the wire-protocol version and any configured application
   version or token. Updates are then serialized, type-checked, and applied to
   the matching state on the other side.

The main state types describe both the stored data and its direction:

- `Value<T>` is stored on both peers and can be changed in either direction.
- `Static<T>` is controlled by the server and read by the client. It cannot be
  written from the client; use `Value<T>` when both sides may write.
- `ValueAtomic<T>` and `StaticAtomic<T>` provide low-overhead access for small,
  copyable values.
- `Signal<T>` is a transient client-to-server event. Signals can coalesce
  pending values or queue every value.
- `ValueTake<T>` and `DataTake<T>` are one-shot server-to-client transfers.
- `VecState<T>` and `MapState<K, V>` synchronize collections.
- `Data<T>` and `DataMulti<T>` efficiently synchronize numeric buffers; Python
  exposes them as NumPy arrays.
- `DataMultiTake<T>` provides independent one-shot numeric transfers keyed by
  `u32`.
- `Image` synchronizes complete images or rectangular updates to an egui
  texture.

State paths follow the Rust field hierarchy. A field named `counter` on the
root state is registered as `root.counter`; a field inside `controls` becomes
`root.controls.<field>`.

## Feature flags

| Feature | Default | Enables |
| --- | --- | --- |
| `client` | yes | egui client state handles and `ClientBuilder` |
| `server` | no | Native Rust server API in `egui_states::server` |
| `python` | no | PyO3 support used to build the Python extension module |
| `build_scripts` | no | `generate_python` and `generate_rust` binding generators |

## Minimal Python-server workflow

Server generation must import the same Rust state type as the UI. In a real
workspace, put that type in a small shared library crate and use it as both a UI
dependency and a build dependency. The complete arrangement is demonstrated in
[`example/`](example/); the essential pieces are shown below.

Define the state shared by the UI and generator:

```rust
use egui_states::Value;

#[derive(egui_states::State)]
pub struct AppState {
    pub counter: Value<i32>,
}
```

Generate a Python package from it in the UI application's `build.rs`:

```rust
use egui_states::build_scripts::generate_python;
use ui_state::AppState;

fn main() {
    generate_python::<AppState>("python/states_server").unwrap();
}
```

The build script needs `egui_states` with the `build_scripts` feature and the
shared state crate as build dependencies:

```toml
[build-dependencies]
egui_states = { version = "0.15", features = ["build_scripts"] }
ui_state = { path = "../ui-state" }
```

Construct and connect the client from the egui application:

```rust
use egui_states::ClientBuilder;
use ui_state::AppState;

let (states, client) = ClientBuilder::<AppState>::new()
    .context(egui_context.clone())
    .build(8091);
client.connect();

// Inside the UI:
if ui.button("Increment").clicked() {
    states.counter.set_signal(states.counter.get() + 1);
}
```

After building the UI crate, add the generated package to Python's import path
and run the server:

```python
from states_server import StatesServer

server = StatesServer(port=8091)
server.states.counter.connect(lambda value: print("counter:", value))
server.start()

server.states.counter.set(41, update=True)
input("Server is running; press Enter to stop.\n")
server.stop()
```

`set_signal` on the client updates the value and invokes connected server
callbacks. The server's `update=True` asks egui to repaint after applying its
new value.

To generate a native Rust server instead, use
`egui_states::build_scripts::generate_rust` and enable the `server` feature in
the server crate:

```toml
[dependencies]
egui_states = { version = "0.15", default-features = false, features = ["server"] }

[build-dependencies]
egui_states = { version = "0.15", features = ["build_scripts"] }
ui_state = { path = "../ui-state" }
```

Generate the module in `build.rs`:

```rust
use egui_states::build_scripts::generate_rust;
use ui_state::AppState;

fn main() {
    println!("cargo:rerun-if-changed=../ui-state/src/");
    generate_rust::<AppState>("src/states_server").unwrap();
}
```

Then declare `mod states_server;`, construct
`states_server::StatesServer`, and call `start()`. See
[`example/rust`](example/rust) for a complete server.

## Custom types

Use `#[egui_states::typed]` to make a struct or fieldless enum available to
`egui-states`. The attribute implements `Typed` and derives Serde serialization
through the library's Serde re-export, so the consuming crate does not need a
direct `serde` dependency.

```rust
#[egui_states::typed]
#[derive(Clone, Debug, PartialEq)]
struct Settings {
    enabled: bool,
    label: String,
}
```

Use `rust_derive(...)` to add derives to the corresponding custom type in
generated Rust server bindings:

```rust
#[egui_states::typed(rust_derive(Debug, PartialEq, Eq))]
#[derive(Clone, Debug, PartialEq, Eq)]
struct Settings {
    enabled: bool,
    label: String,
}
```

This option affects only generated Rust server types and adds to, rather than
replaces, their automatic derives. Generated enums automatically derive
`Clone`, `Copy`, `Debug`, `PartialEq`, `Eq`, and `Hash`; generated structs
automatically derive only `Clone`. Requests for those automatic derives, or for
`Serialize`, `Deserialize`, and `Typed` already supplied by `typed`, are ignored.
Other qualified derive paths are preserved as distinct macros.

Arbitrary derive paths are accepted, but their macros must be available to the
server crate. Required prerequisite derives and field trait implementations are
not inferred.

Add `egui_states::InitialValue` when the type is used by a `Value` or `Static`
field whose default must be emitted into generated server bindings:

```rust
#[egui_states::typed]
#[derive(Clone, Debug, Default, PartialEq, egui_states::InitialValue)]
struct Settings {
    enabled: bool,
    label: String,
}
```

The `typed` attribute replaces the former
`#[derive(serde::Serialize, serde::Deserialize, egui_states::Typed)]` syntax.

## Compatibility and connection settings

The handshake can reject a connection for three reasons:

- The internal wire-protocol version is always checked. It changes when the
  serialized protocol changes incompatibly.
- An optional application version can be configured with
  `ClientBuilder::version` and the matching `ServerOptions` or generated
  Python-server argument.
- An optional authentication token can be configured in the same places.

The generated state-layout hash is exposed for use as the application version,
but it is not enforced automatically. To reject mismatched state trees, use the
client builder's hash and the generated server's `VERSION_HASH`:

```rust
let builder = ClientBuilder::<AppState>::new();
let layout_version = builder.get_version_hash();
let (states, client) = builder.version(layout_version).build(8091);
```

For Python, construct the generated server with
`StatesServer(port=8091, version=StatesServer.VERSION_HASH)`. For Rust, set
`ServerOptions::version` to `Some(StatesServer::VERSION_HASH)` before calling
`StatesServer::with_options`.

Renaming, reordering, adding, or changing state fields changes the layout hash.
Generate bindings from the same state type and rebuild both sides after any
such change. Handshake failures are reported through server diagnostics and the
client connection state.

## Native and WebAssembly clients

The state API is the same on native and WASM targets. On native targets,
`ClientBuilder::build` creates a background thread with its own Tokio runtime;
the application does not need to provide one. On WASM, the synchronization task
runs on the browser executor.

The included web example uses [Trunk](https://trunkrs.dev/):

```sh
trunk serve
```

This serves the UI on port `8090`; run either example server separately on port
`8091`.

## Troubleshooting

- Call `Image::initialize` on the client before connecting or before the server
  sends image updates. Updates received without an initialized texture are
  acknowledged but cannot become visible.
- Keep Rust `CallbackHandle` values alive. Dropping a handle unregisters its
  callback.
- Add an appropriate `cargo:rerun-if-changed` line to generator build scripts,
  or Cargo may not regenerate bindings after the shared state definition
  changes.
- `Value` messages have a serialized size limit. Client `set` rejects an
  oversized value; an in-place `write` keeps the local edit but cannot send it,
  leaving the peers out of sync until a later successful update.
- `ServerOptions::new` binds all IPv4 interfaces by default. Set `ip_addr` when
  the server should only be reachable through a particular interface.

See [`example/README.md`](example/README.md) for commands to run the complete
examples and [`docs/architecture.md`](docs/architecture.md) for the protocol
and synchronization model.
