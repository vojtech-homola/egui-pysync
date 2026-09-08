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
3. Generate matching Python or Rust server bindings from the same state
   definition, using a build script or a generator executable.
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
  texture; `ImageMulti` provides a sparse, server-controlled collection of
  textures indexed by `u32` keys.

State paths follow the Rust field hierarchy. A field named `counter` on the
root state is registered as `root.counter`; a field inside `controls` becomes
`root.controls.<field>`.

## Runnable examples and tests

Start with the [counter](example/counter/README.md): one native
GUI, two fields, and interchangeable Python and Rust servers. Continue with the
[showcase](example/showcase/README.md) for all supported
features, including the browser GUI. The counter loads its UI-owned state module
into its build-script crate with `#[path]`. The showcase keeps state and rendering
in an application library package used by the GUI launcher's build script.
Neither example depends on the other; building either GUI package generates its
Python bindings.

Run independent Python library tests with `uv run pytest`; they generate their
own fixtures automatically. Example smoke tests are opt-in with
`uv run pytest tests/examples` and `cargo test -p example_smoke_tests`.
See [tests/README.md](tests/README.md) for Rust integration and build checks.

## Feature flags

| Feature | Default | Enables |
| --- | --- | --- |
| `client` | yes | egui client state handles and `ClientBuilder` |
| `server` | no | Native Rust server API in `egui_states::server` |
| `python` | no | PyO3 support used to build the Python extension module |
| `build_scripts` | no | `generate_python` and `generate_rust` binding generators |

## Minimal Python-server workflow

Server generation must use the same state definition as the UI. The workflow
below puts that definition in a separate package, `ui_state`, whose library
crate is a dependency of both the GUI crate and its build-script crate. The
[examples](example/) demonstrate other arrangements. This separate state
package is one option; see
[Organizing state definitions and binding generation](#organizing-state-definitions-and-binding-generation)
for three alternatives that keep the state definitions in the UI package.

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

In the UI package's manifest, the build script needs `egui_states` with the
`build_scripts` feature and the `ui_state` package as build dependencies. The
GUI also needs both packages under `[dependencies]`:

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

After building the UI package, add the generated Python package's parent
directory to Python's import path and run the server:

```python
from states_server import StatesServer

server = StatesServer()
server.states.counter.connect(lambda value: print("counter:", value))
server.start(8091)

server.states.counter.set(41, update=True)
input("Server is running; press Enter to stop.\n")
server.stop()
```

`set_signal` on the client updates the value and invokes connected server
callbacks. The server's `update=True` asks egui to repaint after applying its
new value.

To generate a native Rust server instead, use
`egui_states::build_scripts::generate_rust` and enable the `server` feature in
the Rust server package's manifest:

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
`states_server::StatesServer`, and call `start(port, ip_addr, token)`. See
[`example/counter/rust`](example/counter/rust) for a complete server.

## Organizing state definitions and binding generation

The generator executes `State::new` with a state-description builder; it does
not parse UI source files or inspect a running GUI. Any executable that can
construct that description can generate bindings. The `build_scripts` feature
enables the generators even when they are called outside a build script.

In Rust terminology:

- A **package** is defined by `Cargo.toml` and can contain one library target
  and multiple binary targets.
- A **crate** is a compilation unit. The library target, each binary target,
  and the build script are compiled as separate crates.
- A **module** organizes code within a crate. The same source file can be
  loaded as a module into more than one crate.

Cargo compiles and executes a package's `build.rs` before compiling its library
and binary targets. Adding `src/lib.rs` therefore does not let that package's
build-script crate import its own library crate: that would introduce a build
dependency cycle. Ordinary binary targets can use their package's library crate.
See [Cargo build scripts](https://doc.rust-lang.org/cargo/reference/build-scripts.html)
and [Cargo targets](https://doc.rust-lang.org/cargo/reference/cargo-targets.html).

Besides the separate state package used above, these three arrangements are
available. The layouts and package names below are illustrative.

| Arrangement | State ownership | Generation trigger | Main tradeoff |
| --- | --- | --- | --- |
| Load one source module into the GUI and build-script crates | UI package | Building the GUI package | Source compiled in multiple crate contexts |
| Add a generator binary target | UI package's library crate | Explicit `cargo run` command | Generation is a separate execution step |
| Separate application-library and launcher packages | Application package's library crate | Building the launcher package | Two packages; application dependencies may also be built for the host |

### 1. Load the UI's state module into the build-script crate

Keep the state definition in the UI package's source tree:

```text
gui/
├── Cargo.toml
├── build.rs
└── src/
    ├── main.rs
    ├── app.rs
    └── state.rs
```

The GUI binary crate declares `mod state;` in `src/main.rs`. Its build-script
crate loads the same file using Rust's
[`path` attribute](https://doc.rust-lang.org/reference/items/modules.html#the-path-attribute):

```rust
// gui/build.rs
#[path = "src/state.rs"]
mod state;

fn main() {
    println!("cargo:rerun-if-changed=src/state.rs");
    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    egui_states::build_scripts::generate_python::<state::AppState>(
        output.join("states_server"),
    )
    .unwrap();
}
```

There is one maintained source definition, compiled into separate modules and
types in the two crates. Declare the state module's dependencies in both
`[dependencies]` and `[build-dependencies]`, enabling `egui_states/build_scripts`
for the latter. Keep rendering and application initialization outside this
module, and ensure its module paths work in both crate contexts. If it contains
submodules, track their source directory with `cargo:rerun-if-changed` as well.

Build scripts run on the host, including when the GUI target is WASM. Avoid
target-specific fields or defaults that would make the generated description
differ from the GUI's state definition.

A Rust server package's build script can also load the UI-owned source module
through a relative path and call `generate_rust`. Alternatively, add a library
target to the UI package that exposes `pub mod state;`. The GUI binary imports
the state from that library, and the Rust server package declares the UI package
as a build dependency. Its build script can then import the UI library crate
normally; only the UI's own build script needs `#[path]` to load the state source.

The [counter example](example/counter/README.md) uses this library-target variant,
with `CounterState` defined directly in [gui/src/lib.rs](example/counter/gui/src/lib.rs).
Its UI build script loads that file as a module using `#[path = "src/lib.rs"]`.
Building the Rust server automatically builds the UI library and runs the UI build script,
which also generates Python bindings. This compiles the UI package's dependencies
for the host, including eframe unless made optional and disabled for that build.

### 2. Add a generator binary target to the UI package

A package named `my_ui` can expose the state through its library crate and use
it from both a GUI binary and a generator binary:

```text
gui/
├── Cargo.toml
└── src/
    ├── lib.rs              # pub mod state;
    ├── state.rs
    ├── app.rs
    ├── main.rs             # GUI binary crate root
    └── bin/
        └── generate-server.rs
```

Both binary crates import `my_ui::state::AppState` from the library crate. For
example, the generator binary can contain:

```rust
use my_ui::state::AppState;

fn main() -> Result<(), String> {
    let output = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("python/states_server");
    egui_states::build_scripts::generate_python::<AppState>(output)
}
```

Enable `egui_states/build_scripts` under the package's normal `[dependencies]`
for this executable, optionally through a generation feature. Run it explicitly:

```sh
cargo run -p my_ui --bin generate-server
```

Here `-p` selects the package and `--bin` selects its binary target. A plain
`cargo build` does not execute the generator; a project task can run generation
before building the GUI or server. The generator must run as a host executable,
even if the GUI is subsequently built for WASM. Rendering modules and eframe can
be feature-gated to avoid compiling them for generation. This arrangement suits
projects that prefer ordinary library imports and an explicit generation step.

### 3. Separate the application-library and launcher packages

Put state definitions and application code in the library crate of a package
such as `my_ui_core`. A second package, `my_ui`, contains a thin GUI binary and
its build script. The launcher package declares `my_ui_core` under both
`[dependencies]` and `[build-dependencies]`.

The launcher's binary crate and build-script crate can then import
`my_ui_core::AppState` through ordinary package dependencies. There is no cycle,
provided the application package does not depend on the launcher. Its library
crate can also be used by a Rust server package's build script.

Generation runs automatically when the launcher package is built. The cost is
an additional package and potentially compiling application dependencies for
the host as well as the GUI target. Feature-gating rendering can reduce that
cost. This arrangement fits an application that already separates reusable UI
code from its native or browser entry points. The
[showcase example](example/showcase/README.md) uses this arrangement:
`showcase_gui_core` exports `ShowcaseState` and `MainApp`, and `showcase_gui`
depends on that package for both its GUI binary and its build script.

### Generated output locations

State ownership and output location are separate choices. Cargo recommends
that build scripts write into `OUT_DIR`; the first alternative above follows
that convention. An explicit export step can place Python bindings in a stable
import directory. A generator executable can accept an output path or choose
one relative to its package, as shown in the second alternative.

The current examples automatically write Python bindings into their ignored
`python/*_bindings/` directories for convenient imports. Their Rust server build
scripts generate Rust bindings into their own `OUT_DIR`. Whichever arrangement
you choose, regenerate bindings when the state definition changes and avoid
editing generated files.

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
- An optional authentication token is passed to the client builder and to each
  server `start` call.

The generated state-layout hash is exposed for use as the application version,
but it is not enforced automatically. To reject mismatched state trees, use the
client builder's hash and the generated server's `VERSION_HASH`:

```rust
let builder = ClientBuilder::<AppState>::new();
let layout_version = builder.get_version_hash();
let (states, client) = builder.version(layout_version).build(8091);
```

For Python, construct the generated server with
`StatesServer(version=StatesServer.VERSION_HASH)` and then call
`start(8091)`. For Rust, set
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

- Call `Image::initialize` or `ImageMulti::initialize`, as applicable, on the
  client before connecting or before the server sends image updates. Updates
  received without an initialized texture are acknowledged but cannot become
  visible.
- Keep Rust `CallbackHandle` values alive. Dropping a handle unregisters its
  callback.
- Add an appropriate `cargo:rerun-if-changed` line to generator build scripts,
  or Cargo may not regenerate bindings after the shared state definition
  changes.
- `Value` messages have a serialized size limit. Client `set` rejects an
  oversized value; an in-place `write` keeps the local edit but cannot send it,
  leaving the peers out of sync until a later successful update.
- Passing `None` for `ip_addr` when starting a server binds all IPv4
  interfaces. Pass a specific address when the server should only be reachable
  through that interface.

See [`example/README.md`](example/README.md) for commands to run the complete
examples and [`docs/architecture.md`](docs/architecture.md) for the protocol
and synchronization model.
