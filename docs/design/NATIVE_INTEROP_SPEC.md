# Native Interop Specification

**Status:** Proposed

**Audience:** Clorus compiler, runtime, CLI, and Rust bridge maintainers

**Decision:** Native interop is an in-process, compiled capability boundary.
It is not IPC, RPC, reflection over arbitrary Rust crates, or a second
execution engine.

This document proposes the next-generation Rust interop contract. It keeps
Clorus's direct LLVM architecture while making native libraries feel
first-class, safe by default, debuggable, and predictable to version.

## 1. Goals

Clorus programs should be able to call deliberately designed Rust libraries
as ordinary namespaces and methods:

```clojure
(ns app
  (:rust [acme.db :as db]))

(def conn (db/open url))
(db.Connection/query conn "select * from users")
```

Those calls must be ordinary native calls in the same process:

```text
Clorus source
  -> LLVM-generated call
  -> generated bridge wrapper
  -> Rust function or method
```

The system has these goals:

1. **Native by default.** No socket, subprocess, serialization protocol, or
   service lifecycle is involved in normal interop.
2. **Explicit public surface.** A Rust author decides what becomes callable.
   Clorus never treats all `pub` Rust items as a stable foreign API.
3. **One boundary model.** Functions, methods, data records, opaque objects,
   errors, and cleanup use one manifest and one ABI family.
4. **Easy to use, simple to implement.** Users get imports, methods,
   completion, docs, and useful diagnostics. The runtime gets a small number
   of boring, auditable operations.
5. **Debuggable generated code.** Every foreign call can be traced from the
   Clorus source location to a generated wrapper and the Rust symbol.
6. **AOT and JIT parity.** The same bridge works for `clorus run`, `build`,
   and the REPL; no execution mode has a private interop path.

## 2. Non-goals for the first stable version

The following are valuable future features, but are intentionally outside the
first contract:

- Calling arbitrary crates.io APIs without an opted-in bridge surface.
- General Rust generic inference at the Clorus boundary.
- Borrowed Rust values or `&mut T` escaping a call.
- Rust closures, trait objects, or Clorus callbacks crossing the boundary.
- Async/future integration and cancellation.
- Zero-copy collection sharing.
- General C++ parsing, reflection, or an embedded Clang front end.
- IPC as a substitute for native interop.

These exclusions are what keep ownership, ABI compatibility, and error
handling tractable.

## 3. Design influences

The design takes principles, rather than architecture, from mature systems.

| Source | Adopt | Do not adopt |
|---|---|---|
| jank | Native calls are understood at compile time; interop has source-level diagnostics and namespace/method ergonomics. | A host-language compiler front end inside Clorus. |
| clojurust | Explicit opt-in exports, native object handles, value conversions, and structured error mapping. | Its interpreter/tiered IR/JIT architecture. |
| UniFFI | Generated scaffolding, explicit records/errors/objects, a stable call-status boundary, and inspectable generated code. | A broad multi-language binding generator or separate IDL as the normal user workflow. |
| Python extensions | Versioned stable API, opaque named handles, phased load/initialization, and opt-in buffer access. | Python object/refcount semantics or the GIL. |

Clorus remains a direct LLVM compiler. These ideas affect only the narrow
boundary between Clorus values and native libraries.

## 4. Core model: compiled native capabilities

Each native library exposes a generated **capability manifest**. The manifest
is the sole source of truth for what Clorus can import.

```text
Rust annotations
  -> generated bridge wrapper + capability manifest
  -> Clorus validates imports and calls at build/check time
  -> LLVM calls a dependency-scoped native symbol directly
```

For a bridge named `acme-db`, a simplified manifest might contain:

```text
package: acme.db
abi: clorus-native/1

function open
  (String) -> Result<Handle<Connection>, DbError>

method Connection/query
  (Handle<Connection>, String) -> Result<Vector<Row>, DbError>
```

The manifest is generated from Rust annotations, embedded in or installed
beside the bridge library, and copied into Clorus build metadata. It is not
hand-maintained in the normal case.

### 4.1 Why a manifest instead of arbitrary source discovery

Rust source discovery remains useful for migration help, diagnostics, and
generating a starter bridge. It cannot reliably answer the questions which
define a stable foreign API:

- Is a `pub fn` intended for external callers?
- Is its generic parameter fixed, exposed, or an implementation detail?
- Who owns a returned struct or reference?
- Does a `Result` error have stable data suitable for Clorus?
- Is a method thread-safe or valid after library reload?

The manifest answers these questions once, explicitly. It also lets `clorus
check` report an unknown function or bad arity before starting the program.

## 5. Rust author experience

The default path is annotation-driven and requires no handwritten ABI wrapper.

```rust
use clorus::prelude::*;

#[derive(ClorusType)]
pub struct User {
    pub name: String,
    pub age: i64,
}

#[derive(ClorusError)]
pub enum AppError {
    #[error("user not found: {id}")]
    NotFound { id: i64 },
    #[error("database unavailable")]
    Unavailable,
}

#[clorus_export]
pub fn find_user(id: i64) -> Result<User, AppError> {
    // application code
    # todo!()
}
```

The corresponding Clorus code is ordinary data-oriented code:

```clojure
(def user (app/find-user 42))
(:name user)
;; => "Ada"

(try
  (app/find-user -1)
  (catch e
    (if (= (:type (ex-data e)) :app/not-found)
      nil
      (throw e))))
```

### 5.1 Data values versus opaque native objects

`#[derive(ClorusType)]` is for data-only values. A derived type is copied to
and from a Clorus map/record representation and therefore must have a fully
supported recursive field shape.

Use opaque handles for resources, identity, internal mutation, lifetimes, or
large host-owned state:

```rust
#[clorus_export]
pub struct Connection {
    inner: DriverConnection,
}

#[clorus_export]
impl Connection {
    pub fn open(url: String) -> Result<Self, AppError> {
        # todo!()
    }

    pub fn query(&self, sql: String) -> Result<Vec<User>, AppError> {
        # todo!()
    }
}
```

```clojure
(def conn (db.Connection/open url))
(db.Connection/query conn "select name, age from users")
```

Clorus does not inspect `Connection` fields. It holds an opaque, validated
native handle and invokes the generated method wrapper.

## 6. The Clorus Native ABI

The public ABI is versioned from the beginning:

```text
clorus-native/1
```

Only this ABI is a compatibility promise. Internal runtime layouts, LLVM
module names, generated wrapper implementation details, and non-public
symbols may change freely.

### 6.1 ABI value families

Version 1 supports these families:

| Family | Rust shape | Clorus shape | Transfer rule |
|---|---|---|---|
| Scalar | integer, float, bool | number, boolean | copied |
| Text | `String` | string | owned UTF-8 transfer/copy |
| Optional | `Option<T>` | value or `nil` | recursively converted |
| Result | `Result<T, E>` | value or exception | status + structured error |
| Data | derived struct/enum | map/record/keyword data | recursively converted |
| Collection | `Vec<T>`, `HashMap<String, T>` | vector, map | copied in v1.1+ |
| Handle | exported object | opaque Clorus native value | retained/released by runtime |

Every supported shape has an exact conversion rule. A signature containing an
unsupported type fails bridge compilation with the function, parameter/return
position, discovered Rust type, and supported alternatives.

### 6.2 Ownership rules

There is one default rule: **values crossing the boundary are owned**.

- Scalars are copied.
- Strings and derived data are converted to an owned representation.
- A returned handle is owned by the Clorus native-handle value.
- Rust may borrow a handle only for the duration of its generated call.
- Rust references (`&T`, `&mut T`, slices, and iterators) do not escape a call
  in version 1.
- A Clorus value passed to Rust is valid only during that call unless copied
  by a documented conversion.

This eliminates foreign lifetime inference from the language runtime.

### 6.3 Handle identity and validation

An opaque handle must carry enough information to prevent accidental type or
library confusion:

```text
ABI version
bridge package identifier
exported type identifier
native pointer/storage
retain callback
release callback
```

Before a method call, the generated wrapper validates the ABI, package, and
type identifier. Passing a handle from the wrong library is a normal Clorus
exception, never an unchecked pointer cast.

### 6.4 Call status and panics

No Rust panic may unwind through LLVM-generated Clorus frames. Each generated
wrapper uses a fixed call-status result:

```rust
#[repr(C)]
pub struct ClorusCallStatus {
    pub code: u8, // 0 success, 1 declared error, 2 unexpected failure
    pub error: *mut ClorusNativeError,
}
```

The wrapper catches unwinding panics where the selected Rust panic strategy
permits it. A declared `Result::Err` becomes a structured Clorus exception;
an unexpected failure becomes `:clorus/native-internal-error` with bridge and
symbol context. A panic-abort bridge is rejected or clearly documented as
process-fatal.

## 7. Clorus source experience

### 7.1 Imports

The canonical import form remains `ns`:

```clojure
(ns app.main
  (:rust [acme.db :as db]
         [acme.crypto :refer [hash-password]]))
```

Imports resolve through manifests at `clorus check`/`build` time. The compiler
can validate the namespace, exported symbol, arity, and basic conversion
shape without changing Clorus into a statically typed language.

### 7.2 Native error data

Native errors preserve a stable, inspectable shape:

```clojure
(try
  (db.Connection/open "not-a-url")
  (catch e
    (ex-data e)))
;; => {:type :acme.db/invalid-url
;;     :message "invalid database URL"
;;     :native/package "acme.db"
;;     :native/symbol "Connection/open"}
```

### 7.3 Completion and documentation

The manifest enables `replx`, LSP tooling, and CLI docs to show:

```text
db.Connection/query
  ([connection sql]) -> vector<User>
  Native package: acme.db
  Rust symbol: acme_db_connection_query
```

No runtime reflection or loading of arbitrary Rust source is needed.

## 8. Build and load lifecycle

Interop has two distinct phases.

### Phase A: resolve and validate

`clorus check` and `clorus build`:

1. Resolve the declared bridge crate deterministically through Cargo.
2. Read its generated manifest.
3. Validate ABI version, target triple, bridge package ID, and manifest hash.
4. Validate Clorus imports and call sites.
5. Generate/locate dependency-scoped LLVM declarations.

No bridge application initialization executes during this phase.

### Phase B: initialize and execute

At program startup, the runtime loads or links the bridge, verifies its
manifest identity again, then executes any deliberately registered
initialization hook. Initialization is minimal: registrations and immutable
configuration only. It must not call arbitrary Clorus code.

Static linking is preferred for ordinary AOT applications. Dynamic library
loading may be supported for development and packaged plugins, but it is not
required for native interop to work.

## 9. Debuggability contract

Generated code is an implementation detail, but never a black box.

`clorus build --keep-native-bridge` retains:

```text
target/clorus-native/acme-db/
  manifest.json
  generated_bridge.rs
  generated_bindings.clri
  symbols.txt
  build.log
```

`clorus native explain acme.db/Connection/query` prints:

1. The Clorus import and call signature.
2. The resolved bridge crate/version/path.
3. The ABI and manifest hash.
4. The generated wrapper source location.
5. The exported Rust item and linker symbol.
6. Conversion and ownership rules for every argument and result.

Failures must report both sides of the boundary. For example:

```text
native call error at src/app.clrs:18:3
  db.Connection/query expects (Connection String)
  argument 1 is a handle from package "other.db"
  expected handle: acme.db/Connection (clorus-native/1)
  received handle: other.db/Connection (clorus-native/1)
```

## 10. Performance policy

The default interop path optimizes for correctness and predictability:

- Direct in-process function call after normal JIT/AOT code generation.
- Scalars have no heap conversion cost.
- Handles do not serialize or copy host state.
- Strings, data records, and collections are copied at the boundary.
- There is no IPC overhead, worker process, socket, or wire protocol.

Zero-copy data is deliberately deferred. A later `NativeBuffer` capability may
provide byte-oriented, scoped borrowing for images, files, and numeric data,
but only with explicit pinning/lifetime validation. It must not silently alter
the ownership semantics of ordinary Clorus vectors or maps.

## 11. Advanced capabilities: explicitly later

### 11.1 Callbacks and foreign traits

Callbacks are valuable for event handlers and host extension points, but have
hard lifetime, reentrancy, thread, and error questions. They are post-v1.

When introduced, they use an explicit registration token rather than implicit
conversion of every Clorus function to a Rust closure:

```clojure
(def subscription
  (events/subscribe source
    (native/callback [event] (handle-event event))))

(native/cancel! subscription)
```

The first callback specification must define retention, cancellation,
thread-affinity, reentrancy, exception translation, and shutdown behavior.

### 11.2 Async

Async Rust APIs should first expose a Clorus-owned future/promise handle, not
borrow a Clorus callback or depend on a particular async executor. Cancellation
and result/error conversion must be defined before exposing `Future` values.

### 11.3 Source discovery

`clorus native scan` may inspect a Rust crate and generate:

- an annotated bridge-crate skeleton;
- a suggested `.clri` file;
- a report of unsupported signatures.

It must never silently publish discovered `pub` functions as an application
ABI. Discovery is developer assistance; annotations/manifests define the
contract.

## 12. Delivery plan

### Milestone 0: contract and safety tests

- Publish this ABI/versioning and ownership contract.
- Add ABI conformance fixtures independent of the compiler implementation.
- Add JIT and AOT tests for every supported type and error path.

### Milestone 1: explicit Rust exports

- Create `clorus-interop` and `clorus-export-macro` crates, or equivalents.
- Implement `#[clorus_export]`, `#[derive(ClorusType)]`, and
  `#[derive(ClorusError)]` for scalar/data/error basics.
- Generate manifests, wrappers, and dependency-scoped symbols.
- Preserve existing `.clri` support as a migration path.

### Milestone 2: handles and methods

- Implement validated opaque handles with retain/release callbacks.
- Support exported Rust structs and `impl` methods.
- Add handle misuse, double-release, bridge mismatch, and panic containment
  tests.

### Milestone 3: developer experience

- Add `clorus new --rust-bridge`.
- Add `clorus native explain` and retained generated bridge artifacts.
- Feed manifest signatures/docs into REPL completion and LSP.
- Make diagnostics source-located and deterministic.

### Milestone 4: selected data collections

- Add `Vec<T>` and `HashMap<String, T>` only for recursively supported `T`.
- Benchmark conversion costs and document copy semantics.
- Add large-input, error-path, and ownership regression tests.

### Milestone 5: advanced features

- Propose callbacks, async handles, and `NativeBuffer` separately.
- Each proposal needs a lifecycle design and stress tests before acceptance.

## 13. Release gates

The stable interop tier is not ready until all of the following are true:

- A bridge compiles and behaves identically under JIT and AOT execution.
- ABI mismatch, missing symbol, wrong handle, unsupported conversion, Rust
  error, and Rust panic paths produce deterministic diagnostics or exceptions.
- A bridge built against the previous supported ABI continues to load under
  the documented compatibility policy.
- Ownership tests cover handle cloning/release and collections/data returned
  through errors and success paths.
- Generated wrappers are reproducible and include no machine-specific paths
  in their public manifest.
- A documented bridge template builds on every supported release platform.

## 14. Final decision summary

Clorus should make native libraries easy to use by generating the plumbing,
not by making arbitrary host-language APIs magically callable.

```text
Easy for users:
  annotations, imports, methods, direct calls, generated docs, precise errors

Simple for maintainers:
  one manifest, one ABI, one ownership model, one error model

Native at runtime:
  direct in-process calls; no IPC by default
```

This preserves the Clorus architecture while providing a realistic path to
seamless, production-quality Rust integration.
