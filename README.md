# Clorus

Clorus is a Clojure-inspired programming language, implemented from scratch in Rust and compiled natively via LLVM — no JVM, no bytecode interpreter. It aims to give Clojure developers the language and REPL workflow they already know, with native performance and first-class, two-way Rust interop.

**Status: alpha.** The core language, standard library, and Rust interop all work end to end (see [Language](#language) below for runnable examples), but there are real gaps and open bugs — see [Known limitations](#known-limitations) before you rely on this for anything beyond experimentation.

## Why Clorus

- **Clojure syntax and semantics**, not a Clojure-flavored DSL — s-expressions, immutable-by-default data structures, `def`/`defn`/`let`/`loop`-`recur`, macros with syntax-quote, protocols, multimethods, atoms/refs/agents.
- **No JVM.** Source compiles to LLVM IR and runs as native machine code, either JIT'd for fast iteration (`clorus run`) or ahead-of-time to a standalone executable (`clorus build`).
- **Rust interop as a first-class feature, not an afterthought.** Public Rust functions and `impl` methods are discovered directly from source (no hand-written bindings for common shapes) and linked in at build time.

## Quick start

### Prerequisites

- Rust (edition 2024 crates require rustc 1.85+)
- LLVM 18.x — Clorus links against LLVM via [inkwell](https://github.com/TheDan64/inkwell) with the `llvm18-1` feature. If you have multiple LLVM versions installed, point inkwell at the right one, e.g. `export LLVM_SYS_181_PREFIX=$(brew --prefix llvm@18)`.

### Build

```bash
git clone https://github.com/prabhugopal/clorus.git
cd clorus
cargo build --release
```

This builds the workspace, including:

| Binary | Crate | Purpose |
|---|---|---|
| `clorus` | `clorus-cli` | Main CLI: project scaffolding, build, run, REPL, packaging |
| `repl-dev` | `clorus-repl` | Standalone development REPL |
| `replx` | `clorus-replx` | Extended/adaptive REPL |

Put `target/release` on your `PATH`, or install to `~/.clorus`:

```bash
make install
export PATH="$HOME/.clorus/bin:$PATH"
export CLORUS_HOME="$HOME/.clorus"
```

### Your first project

```bash
clorus new hello-world
cd hello-world
```

`src/main.clrs`:

```clojure
(ns main)

(defn factorial [n]
  (if (< n 2)
    1
    (* n (factorial (- n 1)))))

(defn -main [& _args]
  (println (factorial 5)))
```

```bash
clorus run     # JIT-compile and execute — fast, for development
# => 120

clorus build   # ahead-of-time compile to a standalone native executable
./target/hello-world
# => 120
```

### REPL

```bash
clorus repl
```

The REPL is project-aware (loads namespace/config from `Clorus.toml` when run inside a project), supports TAB-completion and persistent history, and mirrors Clojure's `#'namespace/name` convention for `def`/`defn` output.

## Language

Clorus targets close to full Clojure parity — s-expressions, immutable persistent collections, destructuring, macros with syntax-quote, protocols/records/multimethods with automatic dispatch, atoms/refs/STM/agents/channels, regex, transducers, laziness, exceptions. A couple of representative, verified-working snippets:

```clojure
(defprotocol Greet (hello [this]))
(defrecord Person [name])
(extend-type Person Greet
  (hello [this] (str "Hi, " (:name this))))
(hello (->Person "Ada"))          ; => "Hi, Ada"

(def task (go (+ 100 200)))
(<!! task)                        ; => 300
```

The full language reference lives in [`docs/reference/LANGUAGE_SPEC.md`](docs/reference/LANGUAGE_SPEC.md); treat its status/parity percentages as directional rather than exact — [`docs/generated/PARITY_STATUS.md`](docs/generated/PARITY_STATUS.md) is the canonical, continuously-regenerated source for what's actually implemented.

## Rust interop

Clorus discovers public Rust functions and `impl` methods directly from source and generates the FFI bridge automatically — you don't hand-write bindings for the common cases:

- Free functions and `impl` methods (`&self` / `&mut self`) over primitives, `String`/`&str`, and opaque pointer/reference types
- `Option<T>` and `Result<T, E>` are bridged automatically — a Rust `Err` becomes a catchable Clorus exception
- Crate dependencies are resolved the normal Cargo way via `Clorus.toml`

```clojure
(ns main
  (:rust [mylib :as lib]))

(def counter (lib.Counter/new))
(lib.Counter/bump counter)
```

This is a generated, AST-driven bridge (not a hand-maintained per-type list), which is deliberate — see [`docs/RUST_INTEROP_STATUS.md`](docs/RUST_INTEROP_STATUS.md) for exactly what's auto-mapped today and what still requires a hand-authored `.clri` interface file (generics, trait objects, by-value structs, and standard collections like `Vec`/`HashMap` aren't auto-mapped yet).

## Known limitations

This is an honest list, not a comprehensive changelog — see [`docs/PRODUCTION_PLAN.md`](docs/PRODUCTION_PLAN.md) and [`docs/generated/PARITY_STATUS.md`](docs/generated/PARITY_STATUS.md) for the canonical, continuously-updated status.

- **No character or ratio literals.** `\a`-style chars and `22/7`-style ratios aren't implemented — both now fail with a clean compile error (previously, char literal syntax hung the compiler; fixed).
- **Concurrency is real but not `core.async`-grade yet.** Atoms, refs/STM, agents, channels, `go`, and `alts!!` all work, but `go` blocks run on a fixed-size worker pool (sized to CPU count) rather than as lightweight suspended state machines — a program with more concurrently-blocked `go` blocks than CPU cores can exhaust the pool. STM retry/validation under heavy contention is also not yet hardened.
- **Rust interop doesn't yet cover generics, trait objects, closures/fn-pointers, by-value structs, or standard collections** (`Vec`, `HashMap`) across the FFI boundary — only the shapes listed above are auto-bridged.
- **The REPL recompiles its entire session history on every evaluated form** (O(n²)) — sessions on nontrivial projects slow down, and can effectively hang, as history grows. The fix is architecturally trickier than it looks (see `docs/PRODUCTION_PLAN.md` P0-3) and is not yet done.

## Project structure

```
clorus/
├── crates/
│   ├── clorus-syntax/    # Lexer, parser, AST, macro expansion
│   ├── clorus-codegen/   # LLVM IR generation (via inkwell)
│   ├── clorus-runtime/   # Value representation, persistent collections, atoms/refs/agents/channels/STM
│   ├── clorus-std/       # Standard library (Rust-side primitives)
│   ├── clorus-core/      # Core language plumbing
│   ├── clorus-errors/    # Diagnostics
│   ├── clorus-ffi-gen/   # Rust → Clorus FFI bridge generator
│   ├── clorus-types/     # Canonical type system shared across codegen/FFI
│   ├── clorus-macros/    # Rust proc-macros used internally
│   ├── clorus/           # Library crate re-exporting the pipeline
│   ├── clorus-cli/       # `clorus` binary
│   ├── clorus-repl/      # `repl-dev` binary
│   └── clorus-replx/     # `replx` binary
├── stdlib/clorus/        # Standard library written in Clorus itself
├── examples/
├── tests/
└── docs/
```

## CLI reference

```
clorus new <name>   Create a new Clorus project
clorus build        Compile the current project (ahead-of-time, standalone executable)
clorus run          Compile and run (JIT by default)
clorus check        Check syntax without building
clorus clean        Remove build artifacts (target/)
clorus repl         Start a project-aware interactive REPL
clorus replx        Start the extended/adaptive REPL
clorus pack         Package the project as a .clip library
clorus install      Install a .clip package
clorus version      Print version information
```

## Documentation

- [`docs/LANGUAGE.md`](docs/LANGUAGE.md), [`docs/reference/LANGUAGE_SPEC.md`](docs/reference/LANGUAGE_SPEC.md) — language reference
- [`docs/generated/PARITY_STATUS.md`](docs/generated/PARITY_STATUS.md), [`docs/PRODUCTION_PLAN.md`](docs/PRODUCTION_PLAN.md) — canonical, up-to-date implementation status
- [`docs/RUST_INTEROP_STATUS.md`](docs/RUST_INTEROP_STATUS.md), [`docs/guides/RUST_INTEROP_GUIDE.md`](docs/guides/RUST_INTEROP_GUIDE.md) — Rust interop
- [`BUILD.md`](BUILD.md) — build, install, and distribution details
- [`docs/design/ARCHITECTURE.md`](docs/design/ARCHITECTURE.md), [`docs/COMPILER_ARCHITECTURE.md`](docs/COMPILER_ARCHITECTURE.md) — compiler/runtime architecture

Some documents under `docs/` are historical design notes or superseded status snapshots rather than current state; where a doc and the generated status pages disagree, trust `docs/generated/` and `docs/PRODUCTION_PLAN.md`.

## Contributing

Clorus is under active development as an alpha-stage project. Issues and pull requests are welcome.

## License

MIT
