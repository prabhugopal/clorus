# Code Audit Details (Code-Only)

This document expands the code-level audit with file references, risks, and concrete improvement points. It is based only on source code.

## High-Impact Correctness Risks

1. User macros likely don’t expand across forms  
   - `parse_and_expand` expands each form with a fresh `MacroRegistry`, so `(defmacro ...)` in one form won’t affect later forms.  
   - Files: `crates/clorus-syntax/src/lib.rs`, `crates/clorus-syntax/src/macros.rs`

2. Map/Set are hash-only and mutable  
   - `ClorusHashMap` uses `HashMap<u64, (*mut Value, *mut Value)>` keyed by hash only. Collisions overwrite entries.  
   - `ClorusHashSet` stores only hash values.  
   - Files: `crates/clorus-runtime/src/map.rs`, `crates/clorus-runtime/src/set.rs`

3. Structural equality missing for collections  
   - `clorus_equals` uses pointer identity for list/vector/map/set and other complex types.  
   - File: `crates/clorus-runtime/src/value.rs`

4. `swap!` is not linearizable under contention  
   - No CAS retry loop; comment notes this.  
   - File: `crates/clorus-runtime/src/atom.rs`

5. Quoted lists become vectors; unquote-splicing is not splicing  
   - `compile_quoted` and `compile_syntax_quoted_sequence` return vectors for lists and do not splice `~@`.  
   - File: `crates/clorus-codegen/src/codegen.rs`

6. Symbols are not implemented  
   - `clorus_symbol` returns keyword; symbol equality is pointer-based; quoted symbols become strings.  
   - Files: `crates/clorus-runtime/src/value.rs`, `crates/clorus-runtime/src/io.rs`, `crates/clorus-codegen/src/codegen.rs`

7. `update` is compiled but runtime is stubbed  
   - `clorus_map_update` is invoked by codegen but returns unchanged map.  
   - Files: `crates/clorus-codegen/src/codegen.rs`, `crates/clorus-runtime/src/collections.rs`

## Language/Reader Gaps

1. Metadata is ignored  
   - `^` is consumed and discarded, never reaches AST metadata.  
   - File: `crates/clorus-syntax/src/parser.rs`

2. `binding` and `var` exist in AST but aren’t codegen’d  
   - `Expr::Binding` and `Expr::Var` are defined but no compile path.  
   - Files: `crates/clorus-syntax/src/ast.rs`, `crates/clorus-codegen/src/codegen.rs`

3. Reader support is incomplete  
   - No chars, ratios, regex, reader conditionals.  
   - File: `crates/clorus-syntax/src/lexer.rs`

## Runtime Semantics Mismatches vs Clojure

1. `contains?` only supports sets  
   - Clojure supports maps and vectors.  
   - File: `crates/clorus-codegen/src/codegen.rs`

2. `conj` is incomplete  
   - Runtime supports vector/list/set only; map conj is missing.  
   - File: `crates/clorus-runtime/src/collections.rs`

3. `conj` in stdlib always returns vector  
   - `stdlib/core.clr` redefines `conj` via `concat` and `[]`.  
   - File: `stdlib/core.clr`

## Memory/Resource Issues

1. `clorus_println_variadic` and `clorus_pr` leak retains  
   - `clorus_nth` retains values; the print functions do not release.  
   - File: `crates/clorus-runtime/src/io.rs`

2. Collection functions mix retain/release inconsistently  
   - Particularly in `map.rs`, `set.rs`, `collections.rs`.  
   - Files: `crates/clorus-runtime/src/map.rs`, `crates/clorus-runtime/src/set.rs`, `crates/clorus-runtime/src/collections.rs`

## Compiler/Codegen Risks

1. Monolithic `codegen.rs`  
   - Large match blocks, many `unwrap()` calls. Risky for malformed IR and hard to extend.  
   - File: `crates/clorus-codegen/src/codegen.rs`

2. Free-variable collection ignores destructuring  
   - Only binds `Pattern::Symbol`, so destructured params may be treated as free.  
   - File: `crates/clorus-codegen/src/codegen.rs`

3. Protocol dispatch assumes `__type__` string  
   - Missing or wrong type yields runtime errors.  
   - File: `crates/clorus-codegen/src/codegen.rs`

## REPL/CLI Observations

1. REPL recompiles history every eval  
   - Simple but O(n) growth per expression.  
   - File: `crates/clorus-repl/src/repl_engine.rs`

2. Debug prints are unconditional  
   - `eprintln!` calls are not gated.  
   - File: `crates/clorus-repl/src/repl_engine.rs`

3. Stdlib expansion uses per-form macro expansion  
   - Macro registry reset issue applies to stdlib too.  
   - Files: `crates/clorus-repl/src/repl_engine.rs`, `crates/clorus-syntax/src/lib.rs`

## Suggested Refactor Targets (Low-Risk, High-Leverage)

1. Macro registry per file parse  
   - Fixes user macro usability across forms.  
   - Files: `crates/clorus-syntax/src/lib.rs`, `crates/clorus-syntax/src/macros.rs`

2. Replace map/set with structural hashing + equality  
   - Fixes correctness for `get`, `contains?`, equality, and `distinct/dedupe`.  
   - Files: `crates/clorus-runtime/src/map.rs`, `crates/clorus-runtime/src/set.rs`, `crates/clorus-runtime/src/value.rs`

3. Implement CAS retry in `swap!`  
   - Fixes concurrency correctness.  
   - File: `crates/clorus-runtime/src/atom.rs`

4. Fix quoted data structure semantics  
   - Preserve lists; implement true unquote-splicing.  
   - File: `crates/clorus-codegen/src/codegen.rs`

5. Align stdlib predicates to runtime  
   - Remove or rebind stubs; avoid semantic drift.  
   - File: `stdlib/core.clr`
