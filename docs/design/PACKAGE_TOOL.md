# Clorus Package Tool

A Cargo-like build tool for the Clorus programming language.

## Installation

Build the Clorus CLI tool:
```bash
cargo build --release --bin clorus
```

Optionally install it:
```bash
cargo install --path crates/clorus-cli
```

## Quick Start

### Create a New Project
```bash
clorus new my-project
cd my-project
```

This creates:
```
my-project/
├── Clorus.toml      # Project manifest
├── .gitignore       # Git ignore file
└── src/
    └── main.clrs    # Main source file
```

### Run Your Project
```bash
clorus run
```

Output:
```
   Compiling my-project v0.1.0
    Finished dev [unoptimized] target(s) in 0.00s
     Running `src/main.clrs`

=> 30
```

## Commands

### `clorus new <name>`
Create a new Clorus project with the given name.

```bash
clorus new hello-world
```

### `clorus build`
Compile the current project (syntax check in JIT mode).

```bash
clorus build
```

### `clorus run`
Compile and run the current project.

```bash
clorus run
```

### `clorus check`
Check for syntax errors without running.

```bash
clorus check
```

### `clorus repl`
Information about starting an interactive REPL.

```bash
clorus repl
```

### `clorus help`
Display help information.

```bash
clorus help
```

### `clorus version`
Display version information.

```bash
clorus version
```

## Project Manifest (Clorus.toml)

```toml
[package]
name = "my-project"
version = "0.1.0"
authors = ["Your Name <you@example.com>"]

[build]
entry = "src/main.clrs"  # Entry point file
```

## Examples

### Simple Arithmetic
```clojure
; src/main.clrs
(def x 10)
(def y 20)
(+ x y)
```

Run with:
```bash
$ clorus run
=> 30
```

### Functions
```clojure
; src/main.clrs
(defn square [x]
  (* x x))

(defn sum-of-squares [a b]
  (+ (square a) (square b)))

(sum-of-squares 3 4)
```

Run with:
```bash
$ clorus run
=> 25
```

### Recursion - Fibonacci
```clojure
; src/main.clrs
(def n 10)

(defn fib [x]
  (if (< x 2)
    x
    (+ (fib (- x 1)) (fib (- x 2)))))

(fib n)
```

Run with:
```bash
$ clorus run
=> 55
```

### Recursion - Factorial
```clojure
; src/main.clrs
(defn factorial [n]
  (if (< n 2)
    1
    (* n (factorial (- n 1)))))

(factorial 5)
```

Run with:
```bash
$ clorus run
=> 120
```

### Control Flow
```clojure
; src/main.clrs
(def age 25)

(if (< age 18)
  (def status 100)  ; Minor
  (def status 200)) ; Adult

status
```

Run with:
```bash
$ clorus run
=> 200
```

## Current Features

✅ **Project scaffolding** - `clorus new`
✅ **JIT compilation** - Fast development cycle
✅ **Syntax checking** - `clorus check`
✅ **Global variables** - `def`
✅ **Functions** - `defn` with parameters
✅ **Recursion** - Full support for recursive functions
✅ **Control flow** - `if` expressions
✅ **Local bindings** - `let` expressions
✅ **Arithmetic** - `+`, `-`, `*`, `/`
✅ **Comparisons** - `<`, `>`, `=`

## Future Features (Roadmap)

### Phase 1: Language Features
- [ ] More data types (integers, booleans, strings)
- [ ] Lists and vectors
- [ ] Maps/hashmaps
- [ ] Loop constructs (`loop`, `recur`)
- [ ] Pattern matching
- [ ] Macros

### Phase 2: Build Tool Features
- [ ] AOT compilation to native executables
- [ ] Optimization levels (`--release`)
- [ ] Dependency management
- [ ] Package registry
- [ ] Testing framework (`clorus test`)
- [ ] Documentation generation (`clorus doc`)
- [ ] Benchmarking (`clorus bench`)

### Phase 3: Advanced Features
- [ ] FFI (Foreign Function Interface) with Rust/C
- [ ] Standard library
- [ ] Module system
- [ ] Package publishing
- [ ] Cross-compilation
- [ ] Self-hosting (compiler written in Clorus)

## Comparison with Cargo

| Feature | Cargo | Clorus | Status |
|---------|-------|--------|--------|
| `new` | ✅ | ✅ | Complete |
| `build` | ✅ | ✅ | JIT only |
| `run` | ✅ | ✅ | Complete |
| `check` | ✅ | ✅ | Complete |
| `test` | ✅ | ❌ | Planned |
| `bench` | ✅ | ❌ | Planned |
| `doc` | ✅ | ❌ | Planned |
| Dependencies | ✅ | ❌ | Planned |
| Workspaces | ✅ | ❌ | Planned |
| Publishing | ✅ | ❌ | Planned |
| Native executables | ✅ | ❌ | Planned |

## Implementation Details

### JIT Mode (Current)
- Uses LLVM JIT compilation
- Compiles and runs code in memory
- No executable files created
- Fast iteration during development
- Requires Clorus toolchain to run programs

### AOT Mode (Future)
- Ahead-of-time compilation to native code
- Creates standalone executables
- Can distribute binaries without Clorus toolchain
- Better performance with optimizations
- Standard linking with system libraries

## Contributing

The Clorus project is under active development. Contributions are welcome!

## License

MIT OR Apache-2.0
