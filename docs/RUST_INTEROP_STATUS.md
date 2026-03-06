# Rust Interop Status (Source of Truth)

This document is the canonical status for Clorus Rust interop.

## Scope

Covers:
- `Clorus.toml` rust dependency forms
- What works today in `clorus build/run/repl`
- Current limits vs "seamless" crate interop
- Next milestones

## Manifest Support

`[rust-dependencies]` currently supports:

```toml
[rust-dependencies]
# local bridge crate
my_lib = { path = "../my-lib" }

# local bridge crate + explicit interface
my_lib2 = { path = "../my-lib2", interface = "interfaces/my-lib2.clri" }

# registry crate by version
libm = "0.2"

# registry crate + explicit interface
libm2 = { version = "0.2", interface = "interfaces/libm.clri" }
```

Notes:
- `version` dependencies are resolved from Cargo registry source.
- When multiple registry versions of the same crate appear in metadata, resolver prefers the highest discovered registry version.
- If no FFI-compatible functions can be discovered, provide `interface = "..."`.
- `interface = false` is currently treated as "no interface specified" (same as omitting `interface`).

## What Works Today

- Rust deps are processed during `clorus build`.
- Wrapper crate generation in `target/rust-ffi/*`.
- Registry source discovery via `cargo metadata` for `version` deps.
- Basic function-level interop for FFI-compatible signatures.
- REPL/run/build can use imported Rust functions when wrappers are generated successfully.
- Current auto-supported signature primitives:
  - floats: `f32`, `f64`
  - signed ints: `i8`, `i16`, `i32`, `i64`, `isize`
  - unsigned ints: `u8`, `u16`, `u32`, `u64`, `usize`
  - others: `bool`, `String`, `()`, `*mut u8`
- Unsupported signatures now report concrete examples in build errors.
- Interface parse errors now include file path + line/column token context.

## Current Limits

Not yet seamless for arbitrary crates:

- No broad automatic support for complex Rust types.
- Struct/impl/trait-heavy APIs are not generally auto-mapped.
- Generic APIs are not broadly auto-exposed.
- Enum/lifetime-rich APIs often require bridge/interface design.
- "Any crates.io crate just works" is not true yet.

## Practical Rule

Today, reliable production path is:

1. Use a local bridge crate with C-compatible surface, or
2. Use registry crate + explicit `.clri` interface for supported callable surface.

## Known Good Workflow

1. Add rust dependency in `Clorus.toml`.
2. Add interface file when auto-discovery is insufficient.
3. Run `clorus build` and verify wrapper generation logs.
4. Use imports from Clorus namespace (`(:rust [...])` / `(use rust.<lib>)` depending on project pattern).
5. Add an integration test for the specific interop API.

## Import Style Guidance

- Source files/modules: use `ns` with `:rust` imports (canonical style).
- REPL interactive sessions: `use rust.<lib>` is still supported for compatibility.
- Avoid mixing both styles for the same library in the same source file.

## Milestones To Reach "Seamless"

- M1: Stable registry + interface flow (done/mostly done)
- M2: Deterministic type mapping matrix (primitives, strings, buffers, booleans)
- M3: Struct/impl method exposure model
- M4: Better diagnostics with exact unsupported signature/type reasons
- M5: Crates.io starter template in `clorus new`
- M6: Conformance suite across `jit` and `legacy`

## Status Summary

- Foundation: **implemented**
- Production for selected APIs: **usable**
- Full seamless Rust interop: **in progress**

## Current E2E Coverage

The CLI/rust-ffi integration tests now cover:

- local path dependency + `.clri` auto interface + extended numeric signatures
- local path dependency + `.clri` auto interface + narrow integer signatures (`i8/u8/i16/u16`)
- local path dependency + `.clri` auto interface + bool/string signatures
- local path dependency + `.clri` `:rust` override for impl/associated methods
- local path auto-parse with unsupported-signature filtering
- local path auto-parse with supported bool/string/pointer signatures (`bool`, `String`, `*mut u8`)
- local path `interface = false` is covered and behaves like omitted interface (auto-parse path flow)
- local path auto-parse pointer-mismatch rejection includes explicit hint (`*mut u8` is the only auto-supported raw pointer carrier)
- local path dependency + explicit legacy interface path (`.clorus-ffi`)
- local path dependency + explicit legacy interface path (`.clorus-ffi`) with `:rust` symbol override for impl/associated methods
- local path dependency + explicit `.clri` interface path
- explicit interface-path values with leading/trailing whitespace are trimmed before resolution
- local path dependency + explicit `.clri` interface path with `:rust` symbol override for impl/associated methods
- empty `:rust` symbol override rejection with explicit function+interface-path diagnostics
- duplicate normalized interface function-name rejection with explicit interface-path diagnostics
- invalid normalized interface export-symbol rejection (clear function + interface diagnostics)
- empty interface-definition rejection (must define at least one `(fn ...)`)
- invalid `:rust` path-symbol rejection (e.g. malformed `Type::method` paths)
- malformed interface parse diagnostics with file-path + line/column context surfaced through `process_dependencies`
- malformed interface tokenize diagnostics include interface-file path context in `process_dependencies`
- wrapper compile failures from interface-driven generation include interface-file path context
- parse/tokenize/wrapper-compile interface failures assert dependency-name context end-to-end
- interface semantic rejection paths (duplicate names, invalid export symbols, invalid `:rust` paths, unsupported types) assert dependency-name context
- missing local dependency-path diagnostics include dependency name + path
- dependency processing errors now include dependency-name context prefixes
- missing dependency path / shape errors now flow through shared dependency-context formatting
- empty dependency path/version values are rejected early with dependency-name diagnostics
- empty dependency path/version rejection is covered across plain and interface-enabled manifest forms
- missing dependency path diagnostics are covered for both plain and interface-enabled manifest forms
- missing dependency path diagnostics include both explicit interface-path and `interface = true` auto-interface forms
- missing dependency path diagnostics also cover `interface = false` form (treated as no interface)
- version+`interface = true` auto-discovery failures include dependency-name context and searched interface paths
- unsupported explicit interface-extension failures include dependency-name context
- explicit interface directory-path failures include dependency-name context
- explicit empty interface-path failures include dependency-name context
- auto/explicit interface discovery failures are covered with dependency-name context assertions in E2E tests
- auto-interface directory-path misuse (`interfaces/<dep>.clri` as a directory) is E2E-covered with clear diagnostics
- interface semantic validation failures (e.g. invalid `:rust`) include dependency-name context
- dependency-context prefixing is idempotent (no duplicated `Rust dependency '<name>':` prefixes)
- dependency-context prefixing preserves existing dependency-labeled diagnostics (no double-labeling)
- dependency-context prefixing still wraps diagnostics labeled for a different dependency name
- dependency-context wrapping normalizes leading error whitespace and avoids trailing filler for blank errors
- explicit interface-path validation for empty/unsupported extensions (must be `.clri` or `.clorus-ffi`)
- explicit interface-path acceptance for both supported extensions (`.clri`, `.clorus-ffi`) is unit-covered
- explicit interface-path trim semantics are unit-covered for extension and directory validation paths
- explicit interface path must be a file (directory-path misuse rejected with clear diagnostics)
- explicit interface type-matrix validation (unsupported param and return types rejected before wrapper compile)
- explicit interface pointer-type validation (non-`*mut u8` pointer signatures rejected with clear diagnostics)
- local path dependency + `interface = true` auto interface fallback to legacy when `.clri` is absent
- local path dependency + `interface = true` auto interface preference for `.clri` when both `.clri` and `.clorus-ffi` exist
- `interface = true` missing-interface diagnostics with explicit searched paths
- auto-interface missing diagnostics are unit-covered for dependency name + both searched paths (`.clri`, `.clorus-ffi`)
- explicit interface-path missing diagnostics with concrete missing file path
- auto-interface fallback to legacy (`.clorus-ffi`) and explicit directory-path rejection are unit-covered
- auto-interface preference order (prefer `.clri` when both `.clri` and `.clorus-ffi` exist) is unit-covered
