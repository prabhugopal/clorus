# Clorus Stdlib Parity Matrix (`clorus.core`)

Last updated: 2026-03-04  
Validation baseline: `CLORUS_BIN=./target/debug/clorus CLORUS_TEST_ENGINES="jit legacy" tests/run_all_tests.sh` -> Passed 197, Failed 0, Skipped 1.

This is the working source-of-truth for **stdlib parity execution** (separate from core language/compiler parity).

## Legend
- `✅ Paired`: implemented and tested in both `jit` and `legacy`.
- `🟡 Partial`: implemented, but with semantic or arity gaps.
- `❌ Missing`: not available in `clorus.core` yet.

## Function & Collection Building Blocks
- ✅ `identity`, `constantly`, `complement`, `juxt`
- ✅ `partial`, `comp` (baseline behavior)
- ✅ `map`, `filter`, `reduce`, `keep`, `mapcat`
- ✅ `every?`, `some?`, `not-any?`, `not-every?`
- ✅ `take`, `drop`, `take-while`, `drop-while`
- ✅ `partition`, `partition-all`, `split-at`, `split-with`
- ❌ `partition-by`

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
- ✅ `update` (3/4/5/6 arities)
- 🟡 `update` variadic (`7+` args path still blocked by `apply` call-shape parity)

## Nested Map Accessors
- ✅ `get-in`, `assoc-in`, `update-in` semantics covered (including list/vector key paths)

## Sequence Construction Helpers
- ✅ `range`, `repeat`, `repeatedly`, `cycle`, `iterate`
- ✅ `zipmap`, `frequencies`, `group-by`, `remove`

## Exception Helpers
- ✅ `ex-info`, `exception?`, `ex-data`, `ex-message`, `ex-cause`

## Highest-Priority Remaining Gaps
1. `update` full variadic parity (`7+`) after `apply` multi-arg call-shape completion.
2. Add `partition-by`.
3. Expand transducer-facing stdlib surface (`transduce`-adjacent helpers where missing).
4. Add explicit error-message parity tests for stdlib arity and bad-arg paths.

## Execution Plan (Chunked)
1. `update` full variadic parity + regression tests.
2. `partition-by` implementation + edge matrix.
3. transducer stdlib helper parity pass.
4. strict stdlib error-path parity sweep.
