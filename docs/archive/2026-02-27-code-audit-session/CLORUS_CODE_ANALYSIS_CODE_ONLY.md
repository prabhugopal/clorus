# Clorus Codebase Analysis (Code-Only)

Scope: this analysis is based **only on source code** in the repo (no existing `.md` docs). It is intended to help you extend the code, fix bugs, and understand Clojure parity, stdlib coverage, and code quality.

## Executive Summary
- The frontend (lexer/parser/AST) is broad and supports most Clojure-like forms, including macros, destructuring, protocols, records, multimethods, `try`, `throw`, `loop/recur`, `dosync`, `ns`, and `require`. Key gaps: metadata is parsed but effectively ignored; some forms exist in AST but are not codegen’d.
- The runtime is robust for **values, vectors, lists, strings, basic arithmetic, concurrency (atoms/refs/agents/channels/go)** and FFI wiring, but **maps/sets are mutable and hash-by-`u64` only**, causing correctness mismatches vs Clojure.
- Equality and hashing are **pointer-based** for collections and many complex types, so structural equality is missing.
- The stdlib is **non-trivial**, but includes duplicate or stubbed functions and mismatches vs builtins (e.g., `string?`, `map?` stubs in `stdlib/core.clr`).
- To run most Clojure code “as-is” (minus JVM interop), you’ll need to fix **persistent maps/sets, structural equality/hash, full symbol semantics, and `swap!` retry**, plus a few reader/metadata behaviors.

## Architecture Map (File-Wise)

Frontend (syntax + macros)
- `crates/clorus-syntax/src/lexer.rs` — Tokenization for s-expressions, numbers (long/double), strings, keywords, reader macros (`'`, `` ` ``, `~`, `~@`, `#(`, `#{`, `#'`, `@`). Missing: chars, ratios, regex, reader conditionals.  
- `crates/clorus-syntax/src/parser.rs` — Parses special forms (`def`, `defn`, `fn`, `let`, `letfn`, `if`, `do`, `quote`, `loop`, `recur`, `try`, `throw`, `defmacro`, `defrecord`, `deftype`, `defprotocol`, `extend-type`, `defmulti`, `defmethod`, `dosync`, `ns`, `require`, `use`, `declare`). Metadata `^` is recognized but skipped (no AST data persisted).  
- `crates/clorus-syntax/src/ast.rs` — Core AST types and destructuring patterns (`Pattern`, `MapPatternKey`).  
- `crates/clorus-syntax/src/macros.rs` — Macro expansion for threading, control flow, `lazy-seq`, `with-open`, `doseq`, `dotimes`, `while`, `cond`, `case`, `and`, `or`, etc. Also user-defined macros.

Code generation (LLVM)
- `crates/clorus-codegen/src/codegen.rs` — Heart of compiler: expression compilation, special forms, builtins, FFI calls, closure capture, `loop/recur` phi nodes, `try/throw`, `go` blocks.  
- `crates/clorus-codegen/src/ffi_codegen.rs` — Mapping Clorus FFI types to LLVM types.  
- `crates/clorus-codegen/src/namespace_context.rs` — Namespace/require/alias tracking.

Runtime (values + collections + concurrency)
- `crates/clorus-runtime/src/value.rs` — Tagged pointer `Value`, refcounting, boxing/unboxing, equality (`clorus_equals`), type predicates.  
- `crates/clorus-runtime/src/vector.rs` — Persistent vector (32-way trie).  
- `crates/clorus-runtime/src/list.rs` — Persistent list (singly linked).  
- `crates/clorus-runtime/src/map.rs` — Mutable `HashMap` wrapper (hash-only keys).  
- `crates/clorus-runtime/src/set.rs` — Mutable `HashSet` wrapper (hash-only keys).  
- `crates/clorus-runtime/src/collections.rs` — Polymorphic collection ops (`get`, `nth`, `first`, `rest`, `last`, `count`, `concat`, `distinct`, etc).  
- `crates/clorus-runtime/src/string.rs` — String ops (`str`, `subs`, `split`, `join`, case/trim/replace, predicates).  
- `crates/clorus-runtime/src/arithmetic.rs` — Typed arithmetic/comparison.  
- `crates/clorus-runtime/src/atom.rs` — Atoms; `swap!` is simplified (no CAS retry).  
- `crates/clorus-runtime/src/ref_type.rs` + `transaction.rs` — STM refs with MVCC and transaction context.  
- `crates/clorus-runtime/src/agent.rs` + `thread_pool.rs` — Agents with async execution over a thread pool.  
- `crates/clorus-runtime/src/channel.rs` — CSP channels with blocking put/take and optional buffer.  
- `crates/clorus-runtime/src/go_block.rs` — `go` blocks on thread pool.  
- `crates/clorus-runtime/src/protocols.rs` — Protocol method registry (lookup by type/protocol/method).  
- `crates/clorus-runtime/src/keyword.rs` — Keyword interning.  
- `crates/clorus-runtime/src/var.rs` — Vars with root + metadata (string keys).
- `crates/clorus-runtime/src/transducer.rs` — Reduced values only (minimal transducer support).

FFI + std wrappers
- `crates/clorus-ffi-gen/src/lib.rs` + `analyzer.rs` — Rust FFI generation (string-based and canonical type system).  
- `crates/clorus-cli/src/rust_ffi.rs` + `modern_ffi.rs` — Wrapper generation + JSON metadata.  
- `crates/clorus-std/src/fs.rs` — Rust stdlib FS wrappers.  
- `crates/clorus-core/src/io.rs` + `string_utils.rs` — Core convenience functions (slurp/spit, line utils).

Stdlib (pure Clorus)
- `stdlib/core.clr`, `stdlib/set.clr`, `stdlib/lazy.clr`, `stdlib/transducers.clr`

Tests (code-level coverage)
- `tests/features/**`, `tests/language/**`, `tests/stdlib/**`, `tests/integration/**`, `tests/compiler/**`

## Frontend: What Parses vs What Compiles

Parses (lexer + parser):
- Most Clojure-ish forms parse into AST: `def`, `defn`, `fn`, `let`, `letfn`, `if`, `do`, `quote`, `syntax-quote`, `unquote`, `unquote-splicing`, `loop`, `recur`, `try`, `throw`, `defmacro`, `defrecord`, `deftype`, `defprotocol`, `extend-type`, `defmulti`, `defmethod`, `dosync`, `ns`, `require`, `use`, `declare`, `deref`, `var` (`#'`), destructuring.  
  Files: `crates/clorus-syntax/src/parser.rs`, `crates/clorus-syntax/src/ast.rs`.

Codegen coverage (LLVM):
- Core expression compilation includes `let`, `letfn`, `def`, `defn`, `fn`, `defmacro` (register only), `defrecord`, `deftype`, `defprotocol`, `extend-type`, `defmulti`, `defmethod`, `if`, `do`, `quote`, `syntax-quote`, `loop`, `recur`, `try`, `throw`, `dosync`, `ns`, `require`, `use`, `apply`, `reduce`, `go`, `chan` ops, `atom/ref/agent`, and builtins for arithmetic and collections.  
  File: `crates/clorus-codegen/src/codegen.rs`.

Key mismatches and gaps:
- Metadata `^` is lexed but **ignored** in parser (`Token::Meta` consumed and dropped), so metadata does not flow into AST or codegen.  
  File: `crates/clorus-syntax/src/parser.rs`.
- `Expr::Var` exists in AST but **no codegen** for `var` or `#'`.  
  File: `crates/clorus-syntax/src/ast.rs`, no `Expr::Var` in `codegen.rs`.
- `Expr::Binding` exists but **no codegen** for `binding`.  
  File: `crates/clorus-syntax/src/ast.rs`, no `Expr::Binding` in `codegen.rs`.
- Destructuring defaults and `:as` are **parsed but ignored** in codegen (pattern fields exist, but codegen uses only direct key lookups).  
  Files: `crates/clorus-syntax/src/ast.rs`, `crates/clorus-codegen/src/codegen.rs`.
- `quote`/`syntax-quote` produce **vectors**, not list data, and `unquote-splicing` is treated as regular unquote. Symbols inside quotes become strings.  
  File: `crates/clorus-codegen/src/codegen.rs`.

## Runtime Semantics: Core Observations

Value system and memory:
- Values are heap-allocated with manual refcounting (`AtomicU64`) and tagged `ValueTag`. Small values are boxed, not unboxed.  
  File: `crates/clorus-runtime/src/value.rs`.
- `clorus_equals` is **structural only for primitives/strings/keywords**; all collections and complex types compare by **pointer identity**.  
  File: `crates/clorus-runtime/src/value.rs`.
- Symbols are not properly implemented: `clorus_symbol` returns a keyword; equality for symbols is pointer-based.  
  File: `crates/clorus-runtime/src/value.rs`.

Collections:
- **Vector is persistent** (32-way trie + tail), Clojure-like.  
  File: `crates/clorus-runtime/src/vector.rs`.
- **List is persistent** but `count` is `O(n)`.  
  File: `crates/clorus-runtime/src/list.rs`.
- **Map/Set are mutable and hash-only**: `HashMap<u64, (key,val)>` and `HashSet<u64>`; key equality uses hash only. This causes collisions to overwrite and violates Clojure’s structural equality/lookup semantics.  
  Files: `crates/clorus-runtime/src/map.rs`, `crates/clorus-runtime/src/set.rs`.
- `contains?` is implemented **only for sets**, not for maps/vectors like Clojure.  
  File: `crates/clorus-codegen/src/codegen.rs`.
- `conj` only supports vector/list/set (no map merge).  
  File: `crates/clorus-runtime/src/collections.rs`.

Concurrency:
- Atoms use `AtomicPtr`; `swap!` lacks CAS retry loop, which breaks correctness under contention.  
  File: `crates/clorus-runtime/src/atom.rs`.
- STM refs have MVCC-like tracking, but commit uses pointer-based locking and simplified validation.  
  Files: `crates/clorus-runtime/src/ref_type.rs`, `crates/clorus-runtime/src/transaction.rs`.
- Agents execute actions asynchronously on a thread pool; error handling is present but minimal.  
  File: `crates/clorus-runtime/src/agent.rs`.
- Channels are blocking with optional buffer; `alts!!` is wired in codegen but check runtime specifics.  
  File: `crates/clorus-runtime/src/channel.rs`.
- `go` blocks run on a thread pool and return a channel of results.  
  File: `crates/clorus-runtime/src/go_block.rs`.

Protocols and polymorphism:
- Protocol registry is present, but relies on explicit registration; runtime lookup returns function pointers.  
  File: `crates/clorus-runtime/src/protocols.rs`.
- `defrecord`, `deftype`, `defprotocol`, `extend-type`, `defmulti`, `defmethod` compile to runtime hooks.  
  File: `crates/clorus-codegen/src/codegen.rs`.

Transducers:
- Runtime only supports `reduced` wrappers.  
  File: `crates/clorus-runtime/src/transducer.rs`.
- `stdlib/transducers.clr` reimplements `map`, `filter`, `take`, `transduce`, `into` in pure Clorus but does not integrate with runtime reduced markers beyond `reduced?` tests.

## Builtins Wired in Codegen (Not Exhaustive)

These are *compiler-special* and bypass the stdlib definitions:
- Arithmetic/comparison: `+`, `-`, `*`, `/`, `<`, `>`, `=`, and more in `arithmetic.rs`.  
  Files: `crates/clorus-codegen/src/codegen.rs`, `crates/clorus-runtime/src/arithmetic.rs`.
- Collections: `get`, `nth`, `first`, `rest`, `last`, `count`, `assoc`, `dissoc`, `keys`, `vals`, `merge`, `conj`, `cons`, `disj`, `contains?`, `concat`, `take`, `drop`, `distinct`, `dedupe`, `flatten`, `interleave`, `interpose`, `get-in`, `assoc-in`.  
  Files: `crates/clorus-codegen/src/codegen.rs`, `crates/clorus-runtime/src/collections.rs`.
- Strings: `str`, `subs`, `split`, `join`, `upper-case`, `lower-case`, `trim`, `trim-left`, `trim-right`, `replace`, `replace-first`, predicates (`string?`, `starts-with?`, `ends-with?`, `includes?`).  
  Files: `crates/clorus-codegen/src/codegen.rs`, `crates/clorus-runtime/src/string.rs`.
- Concurrency: `atom`, `swap!`, `reset!`, `deref`, `ref`, `ref-set`, `alter`, `commute`, `dosync`, `agent`, `send`, `await`, `await-for`, `chan`, `>!!`, `<!!`, `close!`, `alts!!`, `go`.  
  Files: `crates/clorus-codegen/src/codegen.rs`, `crates/clorus-runtime/src/atom.rs`, `ref_type.rs`, `transaction.rs`, `agent.rs`, `channel.rs`, `go_block.rs`.

## Stdlib Coverage (Pure Clorus)

`stdlib/core.clr` implements:
- Function utilities: `inc`, `dec`, `int`, `partial`, `identity`, `cons`, `not`, `reverse`, `take`, `drop`, `map`, `filter`, `comp`, `constantly`, `complement`, `juxt`.  
- Predicates: `nil?`, `some?`, `zero?`, `pos?`, `neg?`, `even?`, `odd?`, `empty?`, `not-empty`.  
- Collection helpers: `conj` (via `concat`, vector-focused), `zipmap`, `frequencies`, `group-by`, `partition`, `partition-all`, `take-while`, `drop-while`, `split-at`, `split-with`.  
- Math: `abs`, `min`, `max`, `sum`, `product`, `quot` (non-truncating), `rem`, `floor`, `ceil`, `round`.  
- Sequence helpers: `second`, `third`, `ffirst`, `nfirst`, `fnext`, `nnext`, `butlast`, `sort`, `sort-by`, `range`, `repeat`, `repeatedly`, `cycle`, `iterate`.  
- Map helpers: `update`, `select-keys`, `rename-keys`, `invert-map`.  
- Stubs: `keys`, `vals`, `string?`, `char?`, `keyword?`, `symbol?`, `vector?`, `map?`, `set?`, `seq?`, `coll?`, `fn?` (some are duplicated and return constants).  
  File: `stdlib/core.clr`.

`stdlib/set.clr` implements set operations:
- `union`, `intersection`, `difference`, `subset?`, `superset?`.  
  File: `stdlib/set.clr`.

`stdlib/lazy.clr` implements:
- Lazy seq constructors and operations (`make-lazy`, `force`, `lazy-cons`, `lazy-first`, `lazy-rest`, `lazy-range`, `lazy-repeat`, `lazy-cycle`, `lazy-iterate`, etc.).  
  File: `stdlib/lazy.clr`.

`stdlib/transducers.clr` implements:
- `reduced?`, `reduced`, `unreduced`, `ensure-reduced`
- Transducer versions of `map`, `filter`, `take`
- `transduce`, `into`  
  File: `stdlib/transducers.clr`.

Stdlib caveats (code-derived):
- Many predicates are **stubs** or return constants; codegen already wires real runtime predicates.  
  File: `stdlib/core.clr`.
- `conj` uses `concat`, which always returns a **vector**, not a list or set; differs from Clojure semantics.  
  File: `stdlib/core.clr`.
- Duplicated definitions (`odd?`, `even?`, `zero?`, `neg?` are defined twice).  
  File: `stdlib/core.clr`.

## Test Coverage (Code-Only Signals)

There is non-trivial test surface:
- Language basics: `tests/language/*.clr` (syntax, arity, closures, vectors, comparisons, etc).
- Feature suites: `tests/features/**` (collections, functions, macros, sequences, concurrency, exceptions, polymorphism).
- Stdlib tests: `tests/stdlib/*.clr`.
- Integration: `tests/integration/*.clr`.
- Compiler and codegen regression cases: `tests/compiler/**`.  

This indicates breadth, but not depth (no coverage metrics). Some tests are archived in `tests/_archived`.

## Clojure Parity Gaps (Code-Based)

Correctness gaps:
- **Map/Set key equality is hash-only**; collisions overwrite entries and break `get`, `contains?`, `dissoc` semantics.  
  Files: `crates/clorus-runtime/src/map.rs`, `crates/clorus-runtime/src/set.rs`.
- **Collections use pointer equality** in `clorus_equals`, so structural equality (`=`, `hash`) is not Clojure-compatible.  
  File: `crates/clorus-runtime/src/value.rs`.
- **`swap!` lacks CAS retry**; concurrent updates can lose changes.  
  File: `crates/clorus-runtime/src/atom.rs`.

Language/reader gaps:
- **Symbols are not real** (treated like keywords); quoted symbols become strings.  
  File: `crates/clorus-runtime/src/value.rs`, `crates/clorus-codegen/src/codegen.rs`.
- **Chars, ratios, regex, reader conditionals** not lexed.  
  File: `crates/clorus-syntax/src/lexer.rs`.
- **Metadata ignored** in parser/codegen; `Var` metadata exists but is not wired from reader.  
  Files: `crates/clorus-syntax/src/parser.rs`, `crates/clorus-runtime/src/var.rs`.
- **`binding` and `var` are parsed in AST but not compiled.**  
  File: `crates/clorus-syntax/src/ast.rs`, no codegen in `codegen.rs`.

Stdlib/builtin mismatches:
- `contains?` only supports sets.  
  File: `crates/clorus-codegen/src/codegen.rs`.
- `conj` doesn’t support maps; Clojure’s map `conj` supports entry/vector pairs.  
  File: `crates/clorus-runtime/src/collections.rs`.
- `update` is compiled to `clorus_map_update`, but runtime `clorus_map_update` is a stub.  
  Files: `crates/clorus-codegen/src/codegen.rs`, `crates/clorus-runtime/src/collections.rs`.
- `keys`/`vals` in stdlib are stubbed and inconsistent with builtins.  
  File: `stdlib/core.clr`.

Macro/quote mismatches:
- `quote`/`syntax-quote` return vectors for lists; unquote-splicing isn’t true splicing.  
  File: `crates/clorus-codegen/src/codegen.rs`.

## Code Quality Review

Strengths
- Clear module boundaries: syntax → codegen → runtime is cleanly separated.
- Runtime data structures use explicit refcounting and are easy to reason about.
- Codegen is explicit and mostly readable, with many inline comments describing semantics.
- FFI pipeline is robust and has both legacy and type-safe analyzers.

Risks / Technical Debt
- **Correctness risk**: Hash-only map/set semantics are a major mismatch and can corrupt data silently.
- **Concurrency correctness**: Atom `swap!` semantics are incomplete under contention.
- **Semantics drift**: stdlib redefines builtins (e.g., `conj`, `string?`) and may contradict compiler behavior.
- **Symbol + metadata**: reader/runtime mismatch (metadata ignored; symbols treated as keywords).
- **Quote semantics**: lists become vectors in quoted data.

Maintainability concerns
- `crates/clorus-codegen/src/codegen.rs` is large and monolithic; adding features risks regressions.
- Destructuring patterns are parsed but partially ignored (defaults, `:as`).
- Some logic is duplicated in stdlib (e.g., predicate definitions).

## KISS/DRY Improvements (Actionable)

1. Replace stubbed stdlib predicates with calls into runtime builtins (or remove them).  
   File: `stdlib/core.clr`.
2. Remove duplicate definitions in `stdlib/core.clr` (e.g., `odd?`, `even?`, `zero?`, `neg?`).
3. Centralize collection equality and hashing in runtime (single source of truth).  
   File: `crates/clorus-runtime/src/value.rs`, `map.rs`, `set.rs`.
4. Split `codegen.rs` into modules (special forms, builtins, collections, concurrency) to reduce complexity.
5. Expand destructuring support in a single helper (codegen) to handle defaults + `:as` for both `let` and params.

## Priority Fixes to Run Clojure Code “As-Is” (Non-JVM)

1. **Persistent maps/sets with structural hashing + equality**  
   Replace hash-only map/set with persistent HAMT/CHAMP or similar. Ensure `get`, `assoc`, `contains?` use `=` and hash.  
   Files: `crates/clorus-runtime/src/map.rs`, `set.rs`, `value.rs`.
2. **Structural `=` and `hash` for collections**  
   Implement deep equality in `clorus_equals` for vectors/lists/maps/sets, and matching hash functions.  
   File: `crates/clorus-runtime/src/value.rs`.
3. **`swap!` CAS retry loop**  
   Implement retry on CAS failure using function re-application.  
   File: `crates/clorus-runtime/src/atom.rs`.
4. **Proper symbol type + symbol equality**  
   Implement symbol interning or a symbol value type distinct from keywords.  
   File: `crates/clorus-runtime/src/value.rs`.
5. **Reader + metadata**  
   Parse `^` metadata and wire into `def`/`defn` and `Var` metadata.  
   Files: `crates/clorus-syntax/src/parser.rs`, `crates/clorus-runtime/src/var.rs`.
6. **Quote semantics**  
   Preserve list structure in quoted data. Fix unquote-splicing for syntax-quote.  
   File: `crates/clorus-codegen/src/codegen.rs`.

## Suggested Next Steps

1. Decide on persistent map/set structure (HAMT or CHAMP) and implement correct hashing + equality.  
2. Fix `swap!` and structural `=` first; those will unlock correctness for concurrency and collections.  
3. Clean up stdlib stubs and use builtins consistently.  

If you want, I can also produce a focused patch plan with sequencing, tests to add, and suggested module splits for `codegen.rs`.
