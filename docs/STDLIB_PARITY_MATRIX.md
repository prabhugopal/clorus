# Clorus Stdlib Parity Matrix (`clorus.core`)

Last updated: 2026-03-05  
Validation baseline: `CLORUS_BIN=./target/debug/clorus CLORUS_TEST_ENGINES="jit legacy" CLORUS_TEST_JOBS=2 tests/run_all_tests.sh` -> Passed 241, Failed 0, Skipped 1.

This is the working source-of-truth for **stdlib parity execution** (separate from core language/compiler parity).

## Legend
- `✅ Paired`: implemented and tested in both `jit` and `legacy`.
- `🟡 Partial`: implemented, but with semantic or arity gaps.
- `❌ Missing`: not available in `clorus.core` yet.

## Function & Collection Building Blocks
- ✅ `identity`, `constantly`, `complement`, `juxt`
- ✅ `partial`, `comp` (baseline behavior)
- ✅ `map`, `filter`, `reduce`, `keep`, `mapcat`, `apply` (fixed-prefix + coll-tail call shape)
- ✅ `every?`, `some?`, `not-any?`, `not-every?`
- ✅ `take`, `drop`, `take-while`, `drop-while` (ordering parity + bad-arg checks covered)
- ✅ `partition`, `partition-all`, `partition-by`, `split-at`, `split-with` (positive-`n` guard + bad-arg checks covered)
- ✅ transducer completion baseline (`completing`, `transduce` completion arity invocation)

## Numeric Helpers
- ✅ `inc`, `dec`, `int`
- ✅ `abs`, `min`, `max`, `sum`, `product`, `quot`, `rem`, `floor`, `ceil`, `round` (bad-arg checks covered)
- ✅ predicates: `zero?`, `pos?`, `neg?`, `even?`, `odd?`

## Sequence / Collection Predicates
- ✅ `nil?`, `empty?`, `not-empty`, `some?`
- ✅ type predicates: `string?`, `number?`, `char?`, `keyword?`, `symbol?`, `vector?`, `list?`, `map?`, `set?`, `seq?`, `coll?`, `fn?`, `boolean?`, `bool?`

## Map Utilities
- ✅ `keys`, `vals`, `select-keys`, `rename-keys`, `invert-map`, `dissoc-in`
- ✅ `merge-with` (0/1/2/variadic maps + bad-arg error paths)
- ✅ `update` (3/4/5/6/variadic `7+` arities + bad-arg error paths)

## Nested Map Accessors
- ✅ `get-in`, `assoc-in`, `update-in` semantics covered (including list/vector key paths)

## Sequence Construction Helpers
- ✅ `range`, `repeat`, `repeatedly`, `cycle`, `iterate`
- ✅ `sort`, `sort-by` stable baseline (numeric path + comparator arities + duplicate-key stability checks)
- ✅ `zipmap`, `frequencies`, `group-by`, `remove` (bad-arg checks covered)

## Exception Helpers
- ✅ `ex-info`, `exception?`, `ex-data`, `ex-message`, `ex-cause`

## Reader Support Helpers
- ✅ `re-pattern` baseline for regex reader literal lowering (`#\"...\"`)

## Residual Gaps (Narrow, Explicit)
1. `sort`/`sort-by` now support comparator arities with stable tie behavior.
   - Missing: full Clojure comparator contract parity (boolean + compare-style numeric-return comparators across mixed domains).
2. Reader regex support is baseline only (`#\"...\"` -> `re-pattern` string form).
   - Missing: full regex API parity (`re-find`, `re-matches`, replacement regex forms).
3. Stdlib long-tail is now mostly covered for core bad-arg/arity paths.
   - Remaining work is selective deep-behavior expansion, not broad missing primitives.

## Cleanup Completed
- Removed duplicate re-definitions in `stdlib/clorus/core.clr` for:
  - `odd?`
  - `even?`
  - `zero?`
  - `neg?`
- Fixed `invert-map` implementation to avoid destructuring-in-reducer bug path and crash.
- Gated runtime call-site diagnostics behind `CLORUS_LOG_CALL_ERRORS` to keep normal test/runtime output parity-clean.

## Execution Plan (Chunked)
1. Expand `sort`/`sort-by` semantics matrix beyond numeric baseline.
2. Add regex API parity slice on top of current reader-lowering baseline.
