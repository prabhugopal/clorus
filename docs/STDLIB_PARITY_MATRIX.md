# Clorus Stdlib Parity Matrix (`clorus.core`)

Last updated: 2026-03-04  
Validation baseline: `CLORUS_BIN=./target/debug/clorus CLORUS_TEST_ENGINES="jit legacy" tests/run_all_tests.sh` -> Passed 205, Failed 0, Skipped 1.

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
- ✅ `take`, `drop`, `take-while`, `drop-while`
- ✅ `partition`, `partition-all`, `partition-by`, `split-at`, `split-with`
- ✅ transducer completion baseline (`completing`, `transduce` completion arity invocation)

## Numeric Helpers
- ✅ `inc`, `dec`, `int`
- ✅ `abs`, `min`, `max`, `sum`, `product`, `quot`, `rem`, `floor`, `ceil`, `round`
- ✅ predicates: `zero?`, `pos?`, `neg?`, `even?`, `odd?`

## Sequence / Collection Predicates
- ✅ `nil?`, `empty?`, `not-empty`, `some?`
- ✅ type predicates: `string?`, `number?`, `char?`, `keyword?`, `symbol?`, `vector?`, `list?`, `map?`, `set?`, `seq?`, `coll?`, `fn?`, `boolean?`, `bool?`

## Map Utilities
- ✅ `keys`, `vals`, `select-keys`, `rename-keys`, `invert-map`, `dissoc-in`
- ✅ `merge-with` (0/1/2/variadic maps)
- ✅ `update` (3/4/5/6/variadic `7+` arities)

## Nested Map Accessors
- ✅ `get-in`, `assoc-in`, `update-in` semantics covered (including list/vector key paths)

## Sequence Construction Helpers
- ✅ `range`, `repeat`, `repeatedly`, `cycle`, `iterate`
- ✅ `zipmap`, `frequencies`, `group-by`, `remove`

## Exception Helpers
- ✅ `ex-info`, `exception?`, `ex-data`, `ex-message`, `ex-cause`

## Highest-Priority Remaining Gaps
1. Add broader explicit error-path parity tests for stdlib arity and bad-arg paths (baseline compile-arity coverage added for `update`).
2. Continue stdlib long-tail parity (`core` helpers still marked partial in checklist).

## Execution Plan (Chunked)
1. strict stdlib error-path parity sweep.
2. next stdlib long-tail parity chunk from checklist gaps.
