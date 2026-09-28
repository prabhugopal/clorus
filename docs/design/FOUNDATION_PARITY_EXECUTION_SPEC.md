# Foundation, Parity, and Execution Specification

**Status:** Proposed

**Audience:** Clorus language, runtime, compiler, standard-library, and
tooling maintainers

**Purpose:** Define the work required to turn the existing native compiler
into a dependable, Clojure-shaped beta foundation. This is a technical
strategy document, not a claim that every listed feature is implemented.

## 1. Executive decisions

Clorus should be built around five decisions:

1. **Clorus implements its language semantics and standard library.** Rust is
   the host/runtime substrate, not the place where `clojure.core` is silently
   reimplemented.
2. **Rust implements the minimal kernel.** Values, memory ownership,
   persistent-data primitives, OS I/O, scheduling, synchronization, and the
   native ABI belong in Rust.
3. **JIT and AOT are two executors of one compiled program model.** They may
   differ in code installation/linking, but must share parsing, macro
   expansion, lowering, initialization ordering, native declarations, and
   conformance tests.
4. **Clojure parity is a measured compatibility contract.** It is not a
   feature-count claim or an approximation based on names in `core.clr`.
5. **`core.async` is a runtime/compiler feature with a library API.** A
   fixed worker pool running blocking `go` bodies is useful baseline
   concurrency, but is not `core.async` parking semantics.

The goal is a bounded native beta for command-line tools and single-process
services. It is not a claim of drop-in JVM Clojure compatibility.

## 2. Current architecture and observed boundary problems

The current workspace has good separation at the crate level:

```text
clorus-syntax     reader, AST, parser, macro expansion
clorus-codegen    LLVM lowering and native-call emission
clorus-runtime    values, RC, collections, concurrency, I/O, host primitives
clorus-cli        manifests, modules, stdlib loading, JIT/AOT orchestration
clorus-repl       session state and interactive execution
stdlib/clorus     core, transducers, string, set, walk, lazy, HTTP libraries
```

The core pieces exist, but three boundaries are currently too porous:

```text
                Desired                                      Current risk
────────────────────────────────────────────────────────────────────────────
Rust kernel     stable runtime primitives              compiler hard-codes a
               + narrow intrinsics                     growing list of core APIs

Clorus stdlib   semantic functions/macros              some behavior is split
               + namespaces                            between .clr and codegen

Program model   one lowered program                    JIT and build paths each
               + two execution backends               orchestrate stdlib/modules
                                                        separately
```

This specification reduces those seams before expanding the feature surface.

## 3. What belongs in Clorus versus Rust

### 3.1 The rule

The standard library should be implemented in Clorus wherever the operation
is language semantics or composition:

```clojure
(defn map [f coll] ...)
(defn comp [& fs] ...)
(defmacro when [test & body] ...)
(defn update-in [m ks f & args] ...)
```

Rust should provide the substrate that cannot be implemented safely or
efficiently in the language itself:

```text
Value allocation / retain / release
tag inspection and equality/hash primitives
persistent vector/map/set node operations
function invocation ABI and closure storage
string/regex byte handling
file, socket, time, thread, scheduler, and synchronization primitives
native-interop ABI
```

The compiler owns only language special forms and a **small, declared** set
of optimization-preserving intrinsics.

### 3.2 The three-level library model

| Layer | Owner | Examples | Contract |
|---|---|---|---|
| Kernel | Rust runtime | `clorus_conj`, retain/release, channel queue, regex engine | Narrow ABI; no user-facing semantic surprises |
| Intrinsics | compiler + kernel | arithmetic, literal construction, direct tag predicates, `recur` | Exact language semantics; each has a fallback/conformance test |
| Standard library | Clorus | `map`, `filter`, `sort`, transducers, `clorus.string`, `clorus.walk` | Source-visible semantics; normal namespaces and tests |

The existence of a Rust kernel primitive must not force a user-visible API to
be Rust implemented. For example, `clorus_conj` is a kernel primitive while
`map`, `partition-by`, and `update` belong in Clorus.

### 3.3 Intrinsic discipline

`clorus-codegen` currently recognizes a substantial set of names in direct
core-call paths. This is useful for bootstrap and performance, but it must not
become a second undocumented standard library.

Adopt an intrinsic registry with one entry per intrinsic:

```text
name: clorus.core/first
arity: 1
kernel symbol: clorus_first
fallback: stdlib/clorus/core.clr:first
semantic tests: tests/parity/core/first.*
optimization test: tests/compiler/intrinsics/first.*
```

Rules:

1. An intrinsic is permitted only when its semantics are precisely documented.
2. Qualified and unqualified resolution must behave identically.
3. Redefinition/shadowing must follow the chosen Clorus namespace rules; an
   optimization may never silently bypass a user var.
4. Every intrinsic has a cross-backend semantic test and a non-intrinsic
   fallback where practical.
5. New library functions start in Clorus. Promotion to intrinsic requires a
   measured reason and review.

### 3.4 Bootstrap

The bootstrap set should be deliberately tiny:

```text
special forms: if, do, let*, loop*, recur, fn*, def, quote, try/catch
kernel calls: arithmetic, equality, literal construction, first/rest/conj,
              count, get, invoke, throw, type/tag predicates
bootstrap macros: defn, when, cond, ->, ->> only if parser does not own them
```

Everything else loads from versioned Clorus source modules. This makes
stdlib behavior inspectable, patchable, and testable in the language users
write.

## 4. Standard library architecture

### 4.1 Namespace policy

The repository already has Clorus sources for `core`, `transducers`, `string`,
`set`, `walk`, and `lazy`. Make their loading model explicit:

```text
clorus.core            bootstrap-loaded default namespace
clorus.transducers     bootstrap-loaded only if its public API is default
clorus.string          explicit require
clorus.set             explicit require
clorus.walk            explicit require
clorus.lazy            explicit require / experimental until unified
clorus.http            explicit require; service library, not core
```

Do not auto-load every stdlib namespace. It makes startup, symbol ownership,
and shadowing difficult to reason about.

Each file starts with a real namespace declaration and declares its public
surface. A generated manifest records:

```text
namespace, public vars/macros, dependencies, bootstrap tier, source hash
```

This replaces ad-hoc path searching and makes installed and repository
layouts behave the same way.

### 4.2 Remove duplicate definitions by policy

`core.clr` and `transducers.clr` intentionally use overlapping names such as
`map`, `filter`, `take`, and `drop`, but overload-by-load-order is not a
stable module design. Choose one of these designs and encode it in namespace
resolution:

1. `clorus.core/map` is eager and `clorus.transducers/map` is explicitly
   qualified; or
2. `clorus.core/map` follows Clojure's established arity contract, where a
   one-arity form produces a transducer and collection arities perform the
   collection transform.

The recommended path is (2), because it gives users one familiar public name.
The implementation can delegate to internal helpers, but all arities and
error behavior live behind one public var.

### 4.3 Standard-library completion order

Do not add namespaces by popularity alone. Complete semantic foundations in
this order:

1. Reader values and numeric semantics: chars, ratios, integer behavior,
   printing/reading invariants.
2. Sequence contract: `seq`, empty collections, `first/rest/next`, lazy and
   eager behavior, map/set ordering policy, nil behavior.
3. Core collection transforms and transducer arities.
4. `clorus.string`, `clorus.walk`, `clorus.set`, EDN data round-trip.
5. Lifecycle tools: `delay`, `promise`, `future`, `volatile!`.
6. Optional libraries such as HTTP, not before their runtime contracts are
   stable.

### 4.4 Scale and allocation policy

Every eager builder must have a documented complexity target and large-input
test. Minimum acceptance suite:

```text
100,000 elements: map, filter, take, drop, concat, reverse, range, repeat
10,000 elements: partition, partition-by, sort, sort-by
all supported collection kinds: vector, list, map/set sequence views
```

Tests assert correctness, no stack overflow, bounded runtime regression, and
no obvious retain/release leak. Prefer `loop/recur`, transients/internal
builders, or dedicated runtime iterators over recursive append patterns.

## 5. Clojure parity as a compatibility program

### 5.1 Parity tiers

Use feature states that users can understand:

| State | Meaning |
|---|---|
| Compatible | Tested against Clojure behavior for the documented subset |
| Implemented | Clorus code/tests exist, but external semantic comparison is incomplete |
| Partial | Core path works; edge cases, performance, or concurrency semantics remain open |
| Experimental | Available only with explicit limitations |
| Unsupported | Clean diagnostic, never a hang or silent wrong result |

Generated status pages should use this vocabulary and point to the exact
test/fixture evidence.

### 5.2 Differential conformance harness

The existing core-parity harness is a smoke set. Grow it into a deterministic
suite with machine-readable cases:

```json
{
  "name": "map preserves nil values",
  "expr": "(map identity [nil 1 nil])",
  "scope": "clojure.core",
  "requires": ["collections", "functions"],
  "expected": "[nil 1 nil]"
}
```

Run each applicable case in:

```text
reference Clojure
Clorus JIT
Clorus AOT executable
```

Comparison must normalize only documented host differences. Never normalize
away exceptions, printed data shape, numeric behavior, or order without an
explicit rationale.

### 5.3 Priority parity gaps

The current evidence identifies these high-value gaps:

- character literals/type and printing;
- ratio literals/type and numeric promotion policy;
- long-tail macro and namespace semantics;
- deep sequence/lazy-sequence contract;
- full `core.async` parking semantics;
- hardening STM and agent semantics;
- Clojure library namespaces and EDN round trips;
- source locations, exception stack context, and diagnostics.

Do not claim JVM interop, Java class semantics, or all JVM libraries as
parity targets. They are outside Clorus's native-host scope.

## 6. Runtime and collection foundation

### 6.1 Memory safety is a release gate

The runtime uses manually retained `Value*` objects and raw pointers. That
can be performant, but it means every boundary is memory-safety critical:

```text
compiler temporary ownership
collection structural sharing
channel enqueue/dequeue transfer
closure capture and invocation
var rebinding
JIT-generated code lifetime
native bridge handles
```

Required cleanup work:

1. Centralize retain/release conventions in APIs that make transfer explicit:
   `retain_for_store`, `borrowed_result`, `owned_result`, and `take_owned`.
2. Make every exported runtime function document argument and result
   ownership in Rustdoc and the LLVM declaration registry.
3. Replace ad-hoc raw-pointer conversions in new code with narrow internal
   wrappers where possible.
4. Add debug assertions for refcount underflow, invalid tag access, and
   native-handle package/type mismatch.
5. Run sanitizer builds and repeated stress tests in CI on supported Unix
   platforms; investigate every crash rather than skipping it permanently.

### 6.2 Persistent collections

The API may be usable while implementation depth is still evolving. Beta
requires semantic persistence, not merely persistent-looking names:

```clojure
(def a {:x [1 2]})
(def b (assoc-in a [:x 0] 9))
(get-in a [:x 0]) ; must remain 1
(get-in b [:x 0]) ; must be 9
```

Add property tests for persistence, equality/hash consistency, structural
sharing ownership, collision-heavy maps, deep vectors, and map/set iteration
contracts. Avoid exposing host hash iteration as a deterministic language
guarantee unless it is deliberately specified.

### 6.3 Exceptions and dynamic scope

Keep exceptions and dynamic vars in the runtime kernel, but test their
interaction with every execution boundary:

- catch/finally during JIT and AOT;
- dynamic binding restoration after throw, callback failure, and future/go
  completion;
- exception data retaining/releasing nested collections;
- source span and macro-expansion context in error diagnostics.

## 7. One program model; two execution backends

### 7.1 Current risk

The CLI currently has separate orchestration for JIT execution and legacy
compile-and-run/AOT execution. Both load stdlib/modules and construct
initialization wrappers. Such duplication eventually causes one mode to load
a different library set, initialize vars in a different order, or resolve a
native symbol differently.

### 7.2 Target model

Introduce an explicit compiled program package between frontend work and
execution:

```text
sources + manifests + stdlib modules
  -> parse + expand + resolve
  -> ProgramPlan
       ordered modules
       ordered initialization forms
       exported entry point
       native declarations
       source spans and symbol table
  -> LLVM module/bitcode
  -> JIT executor OR AOT linker
```

`ProgramPlan` is backend-neutral. It is the single authority for:

- stdlib module version and order;
- `require` graph and namespace aliases;
- top-level initialization order;
- generated Rust bridge declarations;
- `.clip` dependency identities;
- entry-point and command-line argument policy.

### 7.3 JIT requirements

The JIT backend must:

- resolve all runtime/native symbols through an explicit registry, not
  platform-dependent incidental executable visibility;
- retain the execution engine for as long as generated function pointers or
  closures may be called;
- execute `ProgramPlan` initialization exactly once per program run;
- emit diagnostic IR and symbol maps on demand;
- use the same optimization and ABI assumptions documented for AOT.

### 7.4 AOT requirements

The AOT backend must:

- produce a standalone executable or documented shared-library artifact;
- link the exact runtime/bridge versions recorded in the build manifest;
- have no runtime dependence on repository-relative stdlib source paths;
- embed or install source/manifest metadata needed for diagnostics;
- execute the same initialization plan as JIT;
- expose a reproducible build mode: stable artifact inputs, target triple,
  LLVM version, runtime ABI, and bridge hashes.

### 7.5 JIT/AOT conformance

Every language integration fixture runs in both modes. The minimum matrix is:

```text
core language and macros
stdlib and transducers
namespaces/require
exceptions/dynamic vars
interop primitives and handles
collections and large inputs
concurrency baseline
packaged dependencies
```

One test fixture, one expected observable result. A mode-specific skip is a
release-blocking issue unless the feature is explicitly experimental.

## 8. REPL architecture

The REPL must not rebuild the complete session on every form. This is both a
performance and a correctness concern because generated globals and native
function pointers are tied to execution-engine lifetime.

Target design:

```text
one REPL session
  -> one long-lived execution engine
  -> append/link a small module for each accepted form
  -> retain session globals and symbol registry
  -> explicitly define redefinition semantics
```

Required tests:

```clojure
(def x 10)
x

(defn f [x] (+ x 1))
(f 4)

(defn f [x] (+ x 2))
(f 4)
```

Add a 1,000-form session benchmark and memory budget. Until then, document
the REPL as short-session development tooling rather than a durable Clojure
workspace.

## 9. `core.async` strategy

### 9.1 Separate blocking and parking APIs

Keep these distinct:

```clojure
(>!! ch value)  ; blocking thread operation
(<!! ch)        ; blocking thread operation

(>! ch value)   ; legal only inside go; parks state machine
(<! ch)         ; legal only inside go; parks state machine
(alts! ops)     ; legal only inside go; parks state machine
```

The current fixed-size pool can serve blocking operations and background
work, but it cannot implement parking by running a blocked `go` body on a
worker. That exhausts workers under ordinary workloads.

### 9.2 Target runtime model

```text
go body
  -> compiler/macro lowers body to resumable state machine
  -> state machine registers put/take/alts operation
  -> worker is released when operation cannot proceed
  -> channel scheduler resumes state machine when operation completes
  -> result channel receives return value or structured failure
```

The scheduler owns channel queues and pending operations. The state machine
owns local values, instruction state, dynamic bindings, and cancellation
state. Neither owns raw user closures after cancellation without a documented
retain/release path.

### 9.3 Staged scope

**Stage A — dependable blocking channels**

- fixed, unbuffered, sliding, and dropping buffers;
- close behavior and pending put/take semantics;
- `timeout`, `alts!!`, fairness policy, and cancellation rules;
- contention/close stress tests.

**Stage B — parkable `go` subset**

- `go`, `<!`, `>!`, `alts!`, return/result channel;
- state machine lowering for straight-line expressions, branches, loops, and
  exception cleanup;
- no arbitrary blocking host call inside a `go` body.

**Stage C — full library contract**

- `pipeline`, `pipe`, `mult`, `pub`, `mix`, transducer-aware channels,
  error/cancellation policy, and documented fairness.

Do not advertise `core.async` parity before Stage B has parking tests proving
that thousands of blocked `go` tasks do not consume thousands of threads.

## 10. Codebase cleanup plan

### 10.1 Split concentration points

The largest risk concentration is `clorus-codegen/src/codegen/mod.rs`, which
mixes expression lowering, globals, dispatch, errors, and special cases.
Split by stable language responsibility after behavior tests are pinned:

```text
codegen/
  program.rs       ProgramPlan -> LLVM module
  expr.rs          generic expressions and control flow
  functions.rs     fn/closure/defn/recur
  collections.rs   literals and collection lowering
  calls.rs         normal call resolution
  intrinsics.rs    explicit intrinsic registry
  namespaces.rs    resolved symbols/imports
  exceptions.rs    throw/try/finally lowering
  concurrency.rs   go/dosync lowering
  native.rs        manifest-driven bridge calls
```

No refactor should mix behavior changes with file movement. First create
cross-backend regression tests; then move one responsibility at a time.

### 10.2 Make status truthful and non-duplicative

Keep exactly three document roles:

- generated parity inventory: evidence/status;
- this specification: target architecture and acceptance gates;
- short release notes: user-facing supported subset and known limits.

Archive or mark old plans that advocate automatic arbitrary-crate exposure,
recursive stdlib implementation, or unsupported production claims. Stale
design documents are a release risk because they cause incompatible fixes.

### 10.3 Test harness cleanup

Provide one public verification command:

```bash
make verify-beta
```

It runs, on supported platforms:

1. formatted/linted Rust checks;
2. runtime unit/property/stress tests;
3. JIT and AOT language/stdlib suites;
4. Clojure differential conformance cases;
5. bridge ABI fixtures;
6. REPL multi-evaluation tests;
7. release build/install/run smoke tests;
8. generated-doc freshness checks.

It emits a machine-readable report with commit, OS, architecture, Rust,
LLVM, execution modes, passed/failed/skipped counts, and artifact hashes.

## 11. Delivery order

### Foundation release blockers

1. Establish `ProgramPlan` and eliminate JIT/AOT loading/init divergence.
2. Fix the REPL around one long-lived engine/module-linking model.
3. Pin memory ownership conventions; add sanitizers, property tests, and
   concurrency stress coverage.
4. Establish stdlib namespace/bootstrap/intrinsic policy.
5. Add chars, ratios, and reading/printing conformance before expanding more
   libraries.

### Beta language contract

6. Complete sequence/transducer semantics and large-input tests.
7. Complete exceptions/dynamic vars/namespaces/macro edge tests.
8. Publish the supported native-interop tier from the native interop spec.
9. Deliver dependable blocking channels and explicitly mark parking `go` as
   experimental until Stage B.

### Beta expansion

10. Implement parkable `go` state machines.
11. Add broader libraries and EDN/tooling after core semantic tests are
    stable.
12. Add advanced native features only through separate lifecycle specs.

## 12. Definition of done for a credible beta

Clorus can state the following only when every item is evidenced by CI:

> Clorus builds and runs native command-line tools and single-process services
> with a documented Clojure-shaped core, source-implemented standard library,
> matching JIT/AOT semantics, and an explicit Rust bridge contract. Features
> outside the compatibility matrix—especially parking concurrency, advanced
> native callbacks, and unsupported Clojure/JVM semantics—are experimental or
> rejected with clear diagnostics.

That is a strong, honest foundation. It is more valuable than claiming broad
Clojure parity while JIT/AOT, collection scale, or native lifetime semantics
still diverge.
