# Clorus Beta Readiness: End-to-End Gap and Fix Plan

**Status:** proposed execution plan  
**Audience:** maintainers and early adopters  
**Target:** a bounded, dependable beta for native CLI tools and services—not yet a blanket claim of full Clojure parity or arbitrary Rust-crate interoperability.

## 1. Executive decision

Clorus has a credible compiler and runtime foundation for beta. It is not ready to describe itself as a generally production-ready language yet.

The beta should be a **production-preview beta** with an explicit supported subset:

- Native JIT and ahead-of-time (AOT) execution of ordinary application code.
- Core expressions, functions, closures, recursion, persistent-style collections, macros, namespaces, protocols, multimethods, exceptions, metadata, and dynamic vars.
- Rust dependencies through a stable bridge-crate/interface workflow and the published supported type matrix.
- Single-process command-line tools and services that do not rely on experimental concurrency semantics.

The beta must explicitly exclude or mark experimental:

- Full `core.async` compatibility, parking `go` blocks, and high-contention STM behavior.
- Rust generics, traits/trait objects, closures/function pointers, by-value structs, `Vec`, `HashMap`, and arbitrary third-party crate APIs across the FFI boundary.
- Long-running REPL sessions until incremental compilation is fixed.
- Character and ratio literals, and any other feature listed as missing in generated parity status.

This scope creates a useful, honest beta while the remaining production platform work proceeds.

## 2. Evidence baseline

This plan was based on the repository state inspected on 2026-09-24. **Updated same-day** after a follow-up session did a full pass of direct execution-based verification (not just reading docs/code) and fixed several real bugs found that way -- see the per-item corrections below and the changelog in Section 10.

| Area | Evidence | Assessment |
|---|---|---|
| Compiler architecture | `clorus-syntax` → `clorus-codegen` → LLVM → `clorus-runtime`; JIT and legacy/AOT paths | Strong foundation |
| Rust tests | `cargo test --workspace` completed successfully | Green, but warning-heavy |
| Language suite | Full dual-engine runner now completed to a saved report (`tests/run_all_tests.sh`): 129/129 passing. Stdlib suite (`scripts/test-stdlib.sh`, itself repaired this session -- see Section 10): 125/125 passing across language/stdlib/compiler/integration categories. | Full-suite report now captured; both green |
| Test inventory | 82 language, 31 stdlib, 8 compiler, 4 integration suites (current counts, `scripts/test-stdlib.sh` category breakdown) | Good starting breadth; uneven depth |
| Measured feature inventory | 15 implemented, 5 partial, 2 missing, 1 deliberately out of scope | Beta-capable with clear limits |
| Runtime implementation | Manual reference-counted values and raw pointers in runtime | Powerful, but this session found the real risk wasn't ownership/memory-safety -- it was **missing collection-type cases** in polymorphic dispatch (several `ValueTag` matches only handled Vector/List, silently no-oping or returning nil for HashMap/HashSet). See Section 10. |
| Tooling | CLI, REPL, manifests, FFI wrapper generation, packaging exist | Functional, not yet polished/reliable enough for broad production use |

Canonical status inputs should remain:

- [Generated parity status](generated/PARITY_STATUS.md)
- [Generated coverage summary](generated/COVERAGE_SUMMARY.md)
- [Rust interop status](RUST_INTEROP_STATUS.md)
- [Production plan](PRODUCTION_PLAN.md)

## 3. What is already solid

### 3.1 Compiler and language core

The project is a native compiler, rather than a thin interpreter or transpiler. The architecture is well separated at the crate boundary:

```text
.clrs source
  → lexer / parser / AST / macro expansion
  → LLVM IR code generation
  → JIT execution or AOT native executable
  → reference-counted runtime values and collections
```

The strongest supported language areas are functions and closures (including multi-arity and variadics), tail `recur`, maps/vectors/sets, collection operations, macros, metadata, namespaces, protocols, multimethods, exceptions, and dynamic vars. These form a practical core for application code.

### 3.2 Rust interop foundation

The FFI path is structurally sound:

1. `Clorus.toml` declares a Rust dependency.
2. Cargo metadata locates the dependency.
3. Clorus analyzes source or an explicit `.clri` interface.
4. The wrapper generator emits ABI-safe exports.
5. Codegen emits calls to dependency-scoped symbols.
6. JIT/AOT flow links the generated bridge.

The support is strongest for numeric primitives, booleans, `String`, `&str`, `()`, and the documented raw-pointer carriers. This is sufficient for carefully-designed bridge crates.

### 3.3 Testing intent

The repository has a valuable dual-engine test model: language tests run in both JIT and legacy/AOT modes, and tests include expected compile-error assertions. That is exactly the right conformance direction for a native language implementation.

## 4. Beta gaps and required fixes

Priority is driven by user-visible correctness and the ability to make a reliable release claim, not by feature count.

### P0 — release blockers

#### P0.1: Make collection and sequence operations safe at realistic scale

**Problem (corrected 2026-09-24 after direct verification).** The original diagnosis here, inherited from `PRODUCTION_PLAN.md`, turned out to be imprecise. Direct testing found:

- `map`, `filter`, `take`, `drop`, `range`, and `repeat` are already `loop`/`recur`-based in `stdlib/clorus/core.clr` -- **no stack-overflow risk** from naive recursion, contrary to the original claim.
- `reverse` was not a scale problem at all -- it was **flat-out broken for any input**, including tiny ones (`(reverse [1 2 3])` built nested garbage, `[3 [2 [1 []]]]`-shaped, instead of reversing). Root cause: `conj`'s arguments were backwards in the fold. **Fixed** (see Section 10).
- A **real, still-unresolved** performance issue exists, but it's narrower and stranger than "naive recursion": `map`/`filter` loaded from the real `stdlib/clorus/core.clr` file show clearly super-linear (roughly quadratic) growth on large vectors, while the *exact same source code* defined locally in a test script (not loaded via the stdlib mechanism) is flat/fast at the same sizes. Isolated `conj`, `nth`, and function-call overhead individually to flat/fast; namespace substitution alone didn't reproduce it either. Root cause not yet found -- next step is bisecting the real `core.clr` file to find what triggers it.

**Why this blocks beta.** Collection transforms are central to Clojure-style application code. A beta cannot claim dependable command-line/service use if routine inputs cause stack overflows or severe performance cliffs.

**Fix.**

1. Replace recursive eager builders with `loop`/`recur` or runtime-backed iterative builders.
2. Ensure construction uses an efficient accumulation strategy; avoid repeatedly appending to immutable structures when it produces quadratic behavior.
3. Define collection ordering semantics where host hash-map/hash-set iteration is observable.
4. Add large-input tests for every builder and every collection kind it accepts.

**Acceptance criteria.**

- At least 100k-element tests for `map`, `filter`, `take`, `drop`, `reverse`, `range`, `repeat`, `partition`, and `concat` complete without stack overflow.
- Tests cover vectors, lists, maps, and sets where each operation claims seqability.
- A benchmark baseline is stored and checked manually on release candidates.
- Memory ownership tests verify that temporary sequence materialization does not leak or prematurely free values.

#### P0.2: Establish a reproducible release-quality test gate

**Problem.** The test runner is comprehensive in concept, but a release must provide a saved result proving that the full suite passed. The repository currently has no visible CI workflow, and there is no single release command that combines all required validation.

**Fix.**

1. Add CI on supported platforms (at minimum macOS and Linux if both are supported releases).
2. Run `cargo test --workspace`, the full language suite in JIT and legacy modes, a release build, and docs-status generation/checking.
3. Publish a concise machine-readable summary artifact: counts, failures, skipped tests, engine, OS, LLVM version, Rust version, and commit SHA.
4. Fail CI on unexpected skips, timeouts, or modified generated status files.
5. Add a `make verify-beta` (or equivalent script) that developers and CI both invoke.

**Acceptance criteria.**

- Two consecutive clean CI runs on each release platform.
- Full dual-engine runner produces zero failures and zero unexplained skips/timeouts.
- Build and run smoke tests execute a generated project through both JIT and AOT paths.
- No test depends on untracked local setup beyond documented LLVM/Rust prerequisites.

#### P0.3: Resolve the REPL performance/correctness architecture or limit it explicitly

**Problem.** The REPL recompiles session history on each evaluation, yielding O(n²) growth. The documented naive persistent-codegen fix is incorrect because LLVM global values live in the original execution engine.

**Beta decision.** Either fix this before beta or label the REPL as **short-session/development-only**. Do not market it as a robust long-running Clojure-style REPL until fixed.

**Correct fix direction.** Maintain one long-lived execution engine and incrementally add/link modules, while retaining the globals established by earlier evaluations. Add the module-linking design only after proving multi-evaluation variable/function references work.

**Acceptance criteria.**

- Regression: `(def x 10)` in one evaluation and `x` in a later evaluation returns `10`.
- Regression: define a function in one evaluation, redefine its dependency in another, and document the selected semantics.
- A 1,000-form session stays within a declared responsiveness and memory budget.
- REPL tests are automated rather than only manually exercised.

#### P0.4: Define the beta concurrency contract

**Problem (updated 2026-09-24).** Atoms, refs, agents, channels, `go`, and STM exist. Two real STM correctness bugs were found and **fixed** this session, both needed together (a concurrent stress test failed after fixing only the first):

1. `commit_transaction` computed a sorted `ref_ids` list for deadlock-safe locking but never actually used it -- validation/writes/commutes each locked and released one ref's mutex per iteration across three separate loops instead of holding every lock for the whole commit, so another thread's transaction could interleave mid-commit.
2. `Transaction::stage_write` unconditionally cleared the read record for any ref it wrote to. Since `alter` reads a ref then stages a write computed from that read, this silently deleted the only thing commit-time validation had left to check -- **conflict detection was completely disabled for the single most common STM pattern**, not just weakened.

Verified with a new stress test (8 threads × 300 transfers between two refs, asserting the total stays exact) that failed before the fix and passes reliably (6 runs, no flakes) after.

**Still open, not fixed:** `core.async` parking semantics are still missing (channels/`go`/`alts!!` work but aren't `core.async`-grade -- `go` blocks run on a fixed-size worker pool, not lightweight suspended state machines, so more concurrently-blocked `go` blocks than CPU cores can exhaust the pool). The `dosync` retry loop still has no backoff, so heavy contention means a CPU-spin rather than a crash -- a liveness/perf concern, not the correctness one that's now fixed.

**Fix.** Pick one of these beta contracts:

- **Recommended:** support atoms and basic channels; mark refs/STM, agents, `go`, and `alts!!` experimental.
- **Larger scope:** harden all exposed primitives and publish a precise behavior specification.

**Acceptance criteria for the recommended contract.**

- Docs and CLI help label experimental APIs consistently.
- Atom and basic channel operations have concurrent stress tests and defined shutdown/error behavior.
- `go` worker-pool exhaustion has a clear diagnostic or documented prohibition.
- No examples promise `core.async` parity.

### P1 — strongly recommended for beta quality

#### P1.1: Reduce unsafe-runtime and compiler warning noise

**Problem.** The inspected workspace produced numerous warnings, including many unnecessary `unsafe` blocks in the runtime and approximately 145 warnings from codegen. Warnings obscure new regressions in the code most likely to contain memory or compiler correctness issues.

**Fix.**

1. Set a ratcheting warning budget: no new warnings in changed crates; reduce baseline each sprint.
2. Run `cargo clippy --workspace --all-targets` in CI.
3. Add focused ownership tests for retain/release paths, collection conversion, exceptions, closures, and FFI strings.
4. Use sanitizers where available for native test runs; add Miri coverage for isolated safe/unsafe runtime units where feasible.

**Acceptance criteria.**

- No compiler warnings in release-critical crates (`clorus-runtime`, `clorus-codegen`, `clorus-cli`) or an approved, tracked exception list with an owner and removal milestone.
- ASan/UBSan test job is green on a supported platform.
- FFI string, pointer, and exception paths have leak/double-free regression tests.

#### P1.2: Stabilize the Rust bridge workflow

**Problem.** Interop is useful but narrow. Documentation should not imply arbitrary Rust functions, impl methods, or standard collections work automatically.

**Fix.**

1. Publish one supported type matrix with call/return ownership rules, error translation behavior, and examples.
2. Ship a `clorus new --rust-bridge` template or equivalent sample project.
3. Add end-to-end tests for all supported types in JIT and AOT modes.
4. Give unsupported shapes actionable diagnostics: rejected type, location, supported replacement, and `.clri`/bridge guidance.

**Acceptance criteria.**

- A newcomer can build and use a local bridge crate from documented steps without editing generated files.
- `String`, `&str`, numbers, booleans, unit, errors, and raw-pointer carriers have ownership/error tests in both engines.
- Beta docs state that `Vec`, `HashMap`, generics, traits, callbacks, and by-value structs are unsupported unless specifically implemented.

#### P1.3: Make AOT shipping a first-class tested workflow

**Problem.** Native AOT output is central to the value proposition, but needs more release confidence than simple compilation.

**Fix.**

1. Define supported output layout, dynamic library discovery rules, target triples, and install behavior.
2. Add AOT smoke projects: pure Clorus, stdlib heavy, Rust bridge, exception path, and namespace/multi-module project.
3. Test execution from outside the build directory and after installation/package creation.

**Acceptance criteria.**

- A generated app builds and runs with no developer workspace environment variables.
- AOT test projects pass in CI on each supported platform.
- Packaging/install/uninstall behavior is documented and tested.

#### P1.4: Improve diagnostics and documentation correctness

**Problem.** Diagnostics/source location fidelity remain a tooling gap, and several historical documents point to stale paths or superseded claims.

**Fix.**

1. Prioritize parser, macro expansion, namespace, type/arity, FFI, and linker errors with source spans and suggested recovery.
2. Identify authoritative documents at the top of every historical plan.
3. Add a documentation link checker and a compatibility matrix to release CI.
4. Keep generated parity/coverage data synchronized with tests.

**Acceptance criteria.**

- Representative errors include file, line, column, source excerpt, and useful next action.
- README links resolve and describe the actual beta contract.
- No historical document can be mistaken for current release status.

### P2 — post-beta or beta-expansion work

- Character and ratio literals; later numeric tower (`BigInt`, `BigDecimal`).
- Full lazy-sequence and realization semantics audit.
- `delay`, `future`, `promise`, `deliver`, and `volatile!`.
- Full `core.async` state-machine/parking implementation.
- Rich Rust type mapping: containers, structs, enums, generics, trait objects, callbacks, async-native integration.
- LSP/editor integration, formatter, linter, debugger/profiler support.
- Broader standard-library namespaces: `clorus.string` completion (partially started 2026-09-24 -- `blank?`, `capitalize`, `triml`, `trimr`, `trim-newline` added under a real `clorus.string` namespace, following the same `(require '[clorus.string :as str])` pattern `clorus.set` already used; lower-level primitives like `split`/`join`/`trim` remain bare `clorus.core` globals, not yet moved), walk/data/EDN/spec equivalents (still not started).

## 5. Workstream plan

| Workstream | Primary code areas | First deliverable | Beta exit condition |
|---|---|---|---|
| Collection scale | `stdlib/clorus/core.clr`, runtime collections | Large-input regression suite | No stack overflow/quadratic cliffs on supported operations |
| Runtime safety | `clorus-runtime` value/collection/FFI modules | Ownership test matrix + warning budget | Sanitized tests and controlled warning baseline |
| Compiler/codegen | `clorus-codegen` | Codegen decomposition map + regression cases | Stable JIT/AOT parity on test suite |
| REPL | `clorus-repl` | Long-session semantic test | Incremental engine fix or explicit short-session limit |
| Concurrency | runtime atom/ref/channel/agent/go modules | Published beta contract | Contracted primitives stress-tested |
| Rust FFI | `clorus-cli` FFI modules, `clorus-ffi-gen` | Type/ownership matrix + bridge template | All supported shapes E2E-tested in both engines |
| AOT/distribution | CLI, packaging, install scripts | Installed-app smoke project | Build/run outside workspace passes in CI |
| DX/docs | CLI diagnostics, `docs/`, build scripts | Error-snapshot tests and link checks | Accurate, navigable beta documentation |

## 6. Recommended execution order

### Milestone A — make the current core trustworthy

1. Land iterative/efficient sequence builders and large-input tests.
2. Add release test orchestration and CI artifacts.
3. Set warning ratchet and begin runtime ownership hardening.
4. Declare the concurrency and REPL beta limits in README and CLI documentation.

**Gate:** full JIT/legacy test report is green; large-collection regressions are green; public scope is accurate.

### Milestone B — make the supported use case easy

1. Publish Rust FFI type/ownership matrix.
2. Add and validate a bridge-crate project template.
3. Add AOT app install/run smoke tests.
4. Improve high-frequency compiler/FFI diagnostics.

**Gate:** a fresh developer can build a CLI/service that calls a Rust bridge crate, test it, package it, and run the installed AOT output by following only documented instructions.

### Milestone C — beta release candidate hardening

1. Run concurrency, FFI, and collection stress tests repeatedly.
2. Run sanitizer jobs and resolve release-critical findings.
3. Repair stale documentation and publish known limitations.
4. Perform an upgrade/install/uninstall and sample-project validation on every supported platform.

**Gate:** all beta-release checks below pass twice on clean environments.

## 7. Beta release checklist

### Correctness

- [ ] Full language suite passes in JIT and legacy modes with a saved report.
- [ ] `cargo test --workspace` passes.
- [ ] Large collection regressions pass without timeouts or stack overflow.
- [ ] AOT smoke suite passes outside the source checkout.
- [ ] Supported FFI shapes pass in JIT and AOT modes.
- [ ] Expected compile-error tests verify diagnostics, not merely nonzero exits.

### Runtime safety and performance

- [ ] Runtime ownership/retain-release matrix is tested.
- [ ] Sanitizer run is green on at least one supported native platform.
- [ ] No new warnings; baseline is documented and decreasing.
- [ ] Performance baselines exist for startup, compilation, collection transforms, and FFI calls.
- [ ] REPL limitation is resolved or conspicuously documented.

### Tooling and distribution

- [ ] `clorus new`, `run`, `build`, `check`, `repl`, `pack`, and `install` are smoke-tested.
- [ ] A user can run a built binary without repository-relative paths or hidden environment setup.
- [ ] Failure messages name the relevant source file and actionable next step.
- [ ] CI runs all release checks and stores results.

### Product contract

- [ ] README names the beta-supported application profile.
- [ ] Rust FFI support matrix is linked from README.
- [ ] Experimental concurrency APIs are clearly marked.
- [ ] Known limitations are concise, current, and release-specific.
- [ ] Every advertised feature has either an end-to-end test or an explicit experimental label.

## 8. Definition of done for beta

Beta is complete when Clorus can truthfully make this statement:

> Clorus beta supports building and distributing native command-line tools and single-process services with its documented core language and Rust bridge workflow. JIT and AOT execution are continuously validated. APIs outside the published compatibility matrix—especially advanced concurrency and rich Rust type interop—remain experimental.

That claim is useful to early adopters and leaves room for the larger goal: a production-grade application platform with broader Clojure compatibility, resilient tooling, mature concurrency, and substantially richer Rust interop.

## 9. What “production-grade apps” means after beta

The long-term objective is achievable, but it should be reached in layers:

1. **Beta:** dependable native applications within a small, explicit contract.
2. **Production core:** collection scale, AOT distribution, diagnostics, safety, CI, and stable bridge crates are hardened.
3. **Production platform:** mature async/concurrency semantics, robust tooling, richer Rust host integration, ecosystem/library depth, and operational observability.

Avoid measuring readiness primarily by number of language forms implemented. For a language implementation, predictable semantics, memory safety, repeatable builds, debuggable failures, and an enforceable compatibility contract are the real production milestones.

## 10. Changelog: fixes made same-day (2026-09-24), after this plan was first written

A follow-up session did a pass of direct execution-based verification against this plan and the codebase -- running real code and comparing actual output against expected, not trusting docs or code-review alone -- and found several real, previously-unknown bugs along the way. All are fixed, verified individually, and confirmed against both full test suites (129/129 language/runtime, 125/125 stdlib). None of these were anticipated by the original version of this plan.

- **Compiler**: fixed an infinite loop (not just a missing-feature error) when the lexer hits a character it can't tokenize (e.g. `\a`, `|`, `$`) -- `tokenize()` would spin forever instead of erroring.
- **STM**: fixed the two correctness bugs described in P0.4 above (commit atomicity, `alter` conflict detection silently disabled).
- **Build tooling**: `Makefile`'s `install`/`install-dev` referenced a nonexistent path and a nonexistent `--bin repl` target -- both failed outright. `scripts/build-compiler.sh`'s test summary printed every workspace crate's test count on its own line instead of one summed total. `scripts/test-stdlib.sh` crashed on stock macOS bash (uses `declare -A`, requires bash 4+), then (once fixed) silently discovered 0 test files (a quoted-glob pattern-matching bug), then (once fixed) silently miscounted 8 passing `EXPECT_COMPILE_ERROR` tests as failures, then (once fixed) died partway through from a `set -e` interaction with the fix for that. All fixed; the script now correctly runs and reports on all 125 stdlib/language/compiler/integration tests.
- **`reverse`**: see P0.1 above -- was building garbage, not reversing.
- **`starts-with?`/`ends-with?`/`includes?`**: were silently always-truthy on `false`, for as long as they've existed. Two stacked bugs: a Rust `bool`/LLVM `i32` ABI mismatch (the C ABI doesn't guarantee unused return-register bytes are zeroed), and separately boxing the boolean result as a `Double` instead of a real `Bool` (a boxed `0.0` is still truthy in Clorus, since only `nil`/`false` are falsy).
- **`char-at`/`index-of`**: had complete, correct Rust implementations and LLVM declarations sitting completely unreachable -- missing from the dispatch allowlist. Wired up; also added `last-index-of` (new).
- **`println`/`print`**: never showed the actual contents of maps, sets, or lists -- a second, incomplete, stale copy of "convert value to display string" existed alongside the correct one (used by `str`), with `HashMap` literally hardcoded as `"{...}"` and `List`/`HashSet` missing entirely. Deleted the duplicate, delegated to the correct implementation instead.
- **`conj` on maps**: always returned `nil` -- no `HashMap` case existed in the dispatch at all (not broken logic, genuinely missing). This silently broke `into` for maps too, since `into` is `(reduce conj to from)`.
- **`get`/`first`/`rest`/`last`/`nth`/`take`/`drop`/`concat`/`join` on maps and sets**: audited every `ValueTag` dispatch in the runtime crate (22 sites total) for the same "silent wildcard fallback instead of exhaustive handling" shape of bug that caused the `conj` issue above, and found it repeated across all of these -- all silently returned `nil`/empty, produced an empty string, or skipped elements entirely for `HashMap`/`HashSet`, even though both are genuinely seqable in real Clojure. Fixed with one shared helper (materializes a map's entries as `[k v]` pairs, or a set's elements, as a vector) that every affected operation delegates to, rather than one bespoke implementation per operation per collection type. The remaining audited sites were checked and left alone -- their fallbacks are correct, deliberate design (`clorus_hash` is already a fully exhaustive match with no wildcard; `flatten` correctly treats maps/sets as leaves rather than recursing into them; `get-in`/`assoc-in`/`update-in`'s path argument is correctly rejected for non-sequence types; `clorus_is_truthy`'s catch-all *is* the correct "everything but nil/false is truthy" semantic). String support for these remains an explicit, tracked gap pending the missing Char type, not silently papered over.
- **`clorus.string` namespace**: added (see P2 above).

**Process note for future maintainers:** every one of the bugs above passed the *existing* 129-test suite the whole time they existed -- "N/N tests passing" was never proof these specific paths worked, because nothing exercised them. They were all found by directly running small, targeted snippets and checking actual output against expected output, not by reading code or trusting prior status docs. This plan's own P1.1 (warning noise) and P0.2 (no CI) sections point at the structural fix: an exhaustive-match discipline for `ValueTag` dispatches (so a missing case is a compile error, not a silent no-op) and dedicated print/display regression tests would have caught these mechanically, before they shipped.
