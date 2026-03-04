# Clorus Feature Coverage Matrix

This document tracks test coverage for all Clorus language features based on Clojure compatibility goals.

> Canonical parity status lives in `docs/PARITY_CHECKLIST.md`.
> This matrix is test-inventory focused; if a status conflicts, follow `docs/PARITY_CHECKLIST.md` and update this file.

## Test Coverage Legend
- ✅ **Fully Tested** - Comprehensive tests exist
- 🟡 **Partially Tested** - Basic tests exist, edge cases missing
- ❌ **Not Tested** - No tests exist
- 🔴 **Known Broken** - Tests fail, feature broken

---

## 1. CORE LANGUAGE FEATURES

### Primitives & Literals
| Feature | Status | Test File | Notes |
|---------|--------|-----------|-------|
| Long integers | ✅ | `lang/types/numbers.clr` | |
| Doubles | ✅ | `lang/types/numbers.clr` | |
| Strings | ✅ | `strings/string-operations-test.clr` | |
| Keywords | ✅ | `lang/types/keywords.clr` | |
| Booleans | ✅ | `lang/types/booleans.clr` | |
| Nil | ✅ | `lang/types/nil.clr` | |
| Vectors | ✅ | `collections/vectors.clr` | |
| Lists | ✅ | `collections/lists.clr` | |
| Maps | ✅ | `collections/maps.clr` | |
| Sets | 🟡 | `collections/sets.clr` | Basic only |
| Characters | ❌ | - | **NOT IMPLEMENTED** |
| Ratios | ❌ | - | **NOT IMPLEMENTED** |
| Regex | ❌ | - | **NOT IMPLEMENTED** |

### Arithmetic Operators
| Feature | Status | Test File | Notes |
|---------|--------|-----------|-------|
| `+` | ✅ | `lang/arithmetic/addition.clr` | |
| `-` | ✅ | `lang/arithmetic/subtraction.clr` | |
| `*` | ✅ | `lang/arithmetic/multiplication.clr` | |
| `/` | ✅ | `lang/arithmetic/division.clr` | |
| `mod` | ✅ | `lang/arithmetic/modulo.clr` | |
| `<` | ✅ | `lang/arithmetic/comparison.clr` | |
| `>` | ✅ | `lang/arithmetic/comparison.clr` | |
| `<=` | ✅ | `lang/arithmetic/comparison.clr` | |
| `>=` | ✅ | `lang/arithmetic/comparison.clr` | |
| `=` | ✅ | `lang/arithmetic/comparison.clr` | |
| Bitwise ops | ✅ | `lang/arithmetic/bitwise.clr` | |

### Control Flow
| Feature | Status | Test File | Notes |
|---------|--------|-----------|-------|
| `if` | ✅ | `lang/control/if.clr` | |
| `do` | ✅ | `lang/control/do.clr` | |
| `let` | ✅ | `core/destructuring-test.clr` | |
| `loop`/`recur` | ✅ | `core/loop-recur-test.clr` | |
| `when` | ✅ | `features/macros/control-flow-macros-test.clr` | Macro expansion + execution coverage |
| `when-not` | ✅ | `features/macros/control-flow-macros-test.clr` | Macro expansion + execution coverage |
| `cond` | ✅ | `features/macros/control-flow-macros-test.clr` | Macro expansion + execution coverage |
| `case` | 🟡 | `features/macros/cond-case-test.clr` | Macro exists; parity edge-cases still tracked in `docs/PARITY_CHECKLIST.md` |

### Functions
| Feature | Status | Test File | Notes |
|---------|--------|-----------|-------|
| `defn` | ✅ | `lang/core/functions.clr` | |
| `fn` | ✅ | `lang/core/functions.clr` | |
| Multi-arity | ✅ | `core/multi-arity-test.clr` | |
| Variadic | ✅ | `core/variadic-test.clr` | |
| Closures | ✅ | `lang/core/closures.clr` | |
| Recursion | ✅ | `core/loop-recur-test.clr` | |
| First-class functions | 🟡 | - | Partial support |

### Destructuring
| Feature | Status | Test File | Notes |
|---------|--------|-----------|-------|
| Vector destructuring | ✅ | `core/destructuring-test.clr` | |
| Map destructuring | ✅ | `core/destructuring-test.clr` | |
| `:keys` | ✅ | `core/destructuring-test.clr` | |
| `:as` | ✅ | `core/destructuring-test.clr` | |
| `:or` defaults | ✅ | `core/destructuring-test.clr` | |
| Rest args | ✅ | `core/variadic-test.clr` | |
| Nested | ✅ | `core/destructuring-test.clr` | |

---

## 2. COLLECTIONS

### Vector Operations
| Feature | Status | Test File | Notes |
|---------|--------|-----------|-------|
| `vector` | ✅ | `collections/vectors.clr` | |
| `vec` | ❌ | - | Needs test |
| `conj` | 🟡 | `collections/vectors.clr` | Partial |
| `assoc` | ✅ | `collections/vectors.clr` | |
| `get` | ✅ | `collections/vectors.clr` | |
| `nth` | ✅ | `collections/vectors.clr` | |
| `first` | ✅ | `collections/vectors.clr` | |
| `rest` | ✅ | `collections/vectors.clr` | |
| `last` | ✅ | `collections/vectors.clr` | |
| `count` | ✅ | `collections/vectors.clr` | |
| `empty?` | ✅ | `collections/vectors.clr` | |
| `concat` | ✅ | `collections/vectors.clr` | |
| `take` | ✅ | `stdlib/sequences.clr` | |
| `drop` | ✅ | `stdlib/sequences.clr` | |

### Map Operations
| Feature | Status | Test File | Notes |
|---------|--------|-----------|-------|
| `hash-map` | ✅ | `collections/maps.clr` | |
| `assoc` | ✅ | `collections/maps.clr` | |
| `dissoc` | ✅ | `collections/maps.clr` | |
| `get` | ✅ | `collections/maps.clr` | |
| `get-in` | ❌ | - | Needs test |
| `assoc-in` | ❌ | - | Needs test |
| `update` | ❌ | - | Needs test |
| `update-in` | ❌ | - | Needs test |
| `keys` | ✅ | `collections/maps.clr` | |
| `vals` | ✅ | `collections/maps.clr` | |
| `merge` | ✅ | `collections/maps.clr` | |
| `select-keys` | ❌ | - | Needs test |
| `zipmap` | ❌ | - | Needs test |

### Set Operations
| Feature | Status | Test File | Notes |
|---------|--------|-----------|-------|
| `hash-set` | 🟡 | `collections/sets.clr` | Basic |
| `set` | ❌ | - | Needs test |
| `conj` | 🟡 | `collections/sets.clr` | |
| `disj` | 🟡 | `collections/sets.clr` | |
| `contains?` | 🟡 | `collections/sets.clr` | |
| Set literals `#{}` | ✅ | `collections/test-set-literals.clr` | |

### Sequence Operations
| Feature | Status | Test File | Notes |
|---------|--------|-----------|-------|
| `map` | ✅ | `stdlib/core/map.clr` | |
| `filter` | ✅ | `stdlib/core/filter.clr` | |
| `reduce` | ✅ | `stdlib/core/reduce.clr` | |
| `apply` | ✅ | `stdlib/core/apply.clr` | |
| `range` | ✅ | `stdlib/sequences.clr` | |
| `repeat` | ❌ | - | Needs test |
| `cycle` | ❌ | - | Needs test |
| `iterate` | ❌ | - | Needs test |
| `lazy-seq` | ✅ | `stdlib/lazy/lazy-sequences.clr` | |

---

## 3. CONCURRENCY

### Atoms
| Feature | Status | Test File | Notes |
|---------|--------|-----------|-------|
| `atom` | ✅ | `atoms/atoms-test.clr` | |
| `deref`/`@` | ✅ | `atoms/atoms-test.clr` | |
| `reset!` | ✅ | `atoms/atoms-test.clr` | |
| `swap!` | ✅ | `atoms/atoms-test.clr` | |
| `compare-and-set!` | ❌ | - | Needs test |

### Refs & STM
| Feature | Status | Test File | Notes |
|---------|--------|-----------|-------|
| `ref` | ✅ | `concurrency/refs/stm-test.clr` | |
| `dosync` | ✅ | `concurrency/refs/stm-test.clr` | |
| `alter` | ✅ | `concurrency/refs/stm-test.clr` | |
| `commute` | ✅ | `concurrency/refs/stm-test.clr` | |
| `ensure` | ❌ | - | Needs test |
| `ref-set` | ✅ | `concurrency/refs/stm-test.clr` | |

### Agents
| Feature | Status | Test File | Notes |
|---------|--------|-----------|-------|
| `agent` | ✅ | `concurrency/agents/agent-test.clr` | |
| `send` | ✅ | `concurrency/agents/agent-test.clr` | |
| `send-off` | ❌ | - | Needs test |
| `await` | ✅ | `concurrency/agents/agent-test.clr` | |
| `await-for` | ❌ | - | Needs test |
| `agent-error` | ❌ | - | Needs test |

### Channels (CSP)
| Feature | Status | Test File | Notes |
|---------|--------|-----------|-------|
| `chan` | ✅ | `concurrency/channels/channel-test.clr` | |
| `>!!` | ✅ | `concurrency/channels/channel-test.clr` | |
| `<!!` | ✅ | `concurrency/channels/channel-test.clr` | |
| `close!` | ✅ | `concurrency/channels/channel-test.clr` | |
| `alts!!` | ❌ | - | Needs test |
| `go` blocks | ✅ | `concurrency/channels/go-blocks.clr` | |

---

## 4. POLYMORPHISM

### Records
| Feature | Status | Test File | Notes |
|---------|--------|-----------|-------|
| `defrecord` | ✅ | `polymorphism/test-records.clr` | |
| Field access | ✅ | `polymorphism/test-records.clr` | |
| Constructor | ✅ | `polymorphism/test-records.clr` | |
| Map access | ✅ | `polymorphism/test-record-get.clr` | |

### Protocols
| Feature | Status | Test File | Notes |
|---------|--------|-----------|-------|
| `defprotocol` | ✅ | `polymorphism/test-protocols.clr` | |
| `extend-type` | ✅ | `polymorphism/test-protocols.clr` | |
| Protocol dispatch | ✅ | `polymorphism/test-protocols.clr` | |
| `satisfies?` | ✅ | `language/test-satisfies.clr` | Runtime type/protocol check via protocol registry |
| `extends?` | ✅ | `language/test-protocol-introspection.clr` | Protocol/type membership check |
| `implements?` | ✅ | `language/test-protocol-introspection.clr` | Alias-style protocol/value check |

### Multimethods
| Feature | Status | Test File | Notes |
|---------|--------|-----------|-------|
| `defmulti` | ✅ | `polymorphism/test-multimethod.clr` | |
| `defmethod` | ✅ | `polymorphism/test-multimethod.clr` | |
| Custom dispatch | ✅ | `polymorphism/test-multimethod.clr` | |
| Hierarchical dispatch (`isa?`) | ✅ | `language/test-multimethod-hierarchy.clr` | Keyword/symbol/string/number/bool/nil hierarchy keys |
| `prefer-method` | ✅ | `language/test-multimethod-hierarchy.clr` | Tie-breaking for multiple matching methods |
| `remove-method` | ✅ | `language/test-multimethod-admin.clr` | Removes dispatch entry for compiled multimethod methods |
| `methods` | ✅ | `language/test-multimethod-introspection.clr` | Returns dispatch-value -> method function map |
| `get-method` | ✅ | `language/test-multimethod-introspection.clr` | Returns dispatch-resolved method function (or default) |
| `prefers` | ✅ | `language/test-multimethod-introspection.clr` | Returns preferred dispatch relation map |

### Hierarchy
| Feature | Status | Test File | Notes |
|---------|--------|-----------|-------|
| `derive` | ✅ | `language/test-hierarchy-isa.clr` | Cycle-safe edge insertion |
| `underive` | ✅ | `language/test-hierarchy-isa.clr` | Removes parent relation |
| `isa?` | ✅ | `language/test-hierarchy-isa.clr` | Reflexive + transitive |
| `parents` | ✅ | `language/test-hierarchy-queries.clr` | Direct parent set |
| `ancestors` | ✅ | `language/test-hierarchy-queries.clr` | Transitive parent set |
| `descendants` | ✅ | `language/test-hierarchy-queries.clr` | Reverse transitive set |

---

## 5. MACROS & QUOTING

### Quoting
| Feature | Status | Test File | Notes |
|---------|--------|-----------|-------|
| `quote` `'` | ✅ | `macros/quote-test.clr` | |
| `syntax-quote` `` ` `` | ✅ | `macros/syntax-quote-test.clr` | |
| `unquote` `~` | ✅ | `macros/unquote-test.clr` | |
| `unquote-splicing` `~@` | ✅ | `macros/unquote-splicing-test.clr` | |

### Macros
| Feature | Status | Test File | Notes |
|---------|--------|-----------|-------|
| `defmacro` | 🟡 | `macros/defmacro-test.clr`, `language/test-macro-hygiene.clr` | Baseline hygiene (`foo#` auto-gensym) covered; deeper edge cases pending |
| `macroexpand` | ✅ | `language/test-macroexpand.clr` | Basic expansion behavior covered |
| `macroexpand-1` | ✅ | `language/test-macroexpand.clr` | Single-step expansion behavior covered |
| `gensym` | ✅ | `language/test-gensym.clr` | Runtime uniqueness and prefix behavior covered |
| `&env` | ✅ | `language/test-macro-env-form.clr` | Baseline local-binding map semantics covered |
| `&form` | ✅ | `language/test-macro-env-form.clr` | Macro call-form capture baseline covered |

---

## 6. NAMESPACES & MODULES

### Namespace Declaration
| Feature | Status | Test File | Notes |
|---------|--------|-----------|-------|
| `ns` | ✅ | `lang/namespaces/ns-test.clr` | |
| `in-ns` | ❌ | - | Needs test |
| `:require` | ✅ | `lang/namespaces/require-test.clr` | |
| `:use` | ✅ | `lang/namespaces/use-test.clr` | |
| `:as` aliasing | ✅ | `lang/namespaces/alias-test.clr` | |
| `:refer` | ✅ | `language/test-require-refer-rename.clr` | Baseline behavior covered |
| `:rename` | ✅ | `language/test-require-refer-rename.clr` | Baseline behavior covered |

### Qualified Calls
| Feature | Status | Test File | Notes |
|---------|--------|-----------|-------|
| `namespace/fn` | ✅ | `lang/namespaces/qualified.clr` | |
| `::keyword` | ❌ | - | **NOT IMPLEMENTED** |
| `:namespace/keyword` | ❌ | - | **NOT IMPLEMENTED** |

---

## 7. EXCEPTION HANDLING

| Feature | Status | Test File | Notes |
|---------|--------|-----------|-------|
| `try` | ✅ | `exceptions/try-catch-test.clr` | |
| `catch` | ✅ | `exceptions/try-catch-test.clr` | |
| `finally` | ✅ | `exceptions/try-catch-test.clr` | |
| `throw` | ✅ | `exceptions/try-catch-test.clr` | |
| Multiple catches | 🟡 | `language/test-try-catch-typed.clr` | Typed catches covered; more edge cases pending |
| `ex-info` | ✅ | `language/test-ex-info.clr` | |
| `ex-data` | ✅ | `language/test-ex-info.clr` | |

---

## 8. STANDARD LIBRARY

### clojure.core → clorus.core
| Feature | Status | Test File | Notes |
|---------|--------|-----------|-------|
| `inc`, `dec` | ✅ | `stdlib/core/arithmetic.clr` | |
| `identity` | ✅ | `stdlib/core/identity.clr` | |
| `constantly` | ❌ | - | Needs test |
| `comp` | ✅ | `stdlib/core/comp.clr` | |
| `partial` | ✅ | `stdlib/core/partial.clr` | |
| `complement` | ❌ | - | Needs test |
| `juxt` | ❌ | - | Needs test |

### Type Predicates
| Feature | Status | Test File | Notes |
|---------|--------|-----------|-------|
| `nil?` | ✅ | `types/predicates.clr` | |
| `some?` | ❌ | - | Needs test |
| `number?` | 🟡 | `type-predicates-test.clr` | |
| `string?` | ✅ | `types/predicates.clr` | |
| `vector?` | ✅ | `types/predicates.clr` | |
| `map?` | ✅ | `types/predicates.clr` | |
| `seq?` | ❌ | - | **Needs runtime** |
| `coll?` | ❌ | - | **Needs runtime** |
| `fn?` | ❌ | - | **Needs runtime** |

### String Operations (clojure.string → clorus.string)
| Feature | Status | Test File | Notes |
|---------|--------|-----------|-------|
| `split` | ✅ | `strings/string-operations-test.clr` | |
| `join` | ✅ | `strings/string-operations-test.clr` | |
| `replace` | ✅ | `strings/string-operations-test.clr` | |
| `upper-case` | ✅ | `strings/string-operations-test.clr` | |
| `lower-case` | ✅ | `strings/string-operations-test.clr` | |
| `trim` | ✅ | `strings/string-operations-test.clr` | |
| `starts-with?` | ✅ | `strings/string-operations-test.clr` | |
| `ends-with?` | ✅ | `strings/string-operations-test.clr` | |

---

## SUMMARY STATISTICS

| Category | ✅ Tested | 🟡 Partial | ❌ Not Tested | 🔴 Broken | Coverage |
|----------|-----------|------------|---------------|-----------|----------|
| **Primitives** | 10 | 1 | 3 | 0 | 71% |
| **Arithmetic** | 9 | 0 | 0 | 2 | 82% |
| **Control Flow** | 4 | 0 | 4 | 0 | 50% |
| **Functions** | 6 | 1 | 0 | 0 | 86% |
| **Collections** | 25 | 4 | 11 | 1 | 61% |
| **Concurrency** | 18 | 0 | 5 | 0 | 78% |
| **Polymorphism** | 9 | 0 | 3 | 0 | 75% |
| **Macros** | 4 | 1 | 5 | 0 | 40% |
| **Namespaces** | 5 | 0 | 4 | 0 | 56% |
| **Exceptions** | 4 | 0 | 3 | 0 | 57% |
| **Stdlib** | 14 | 1 | 10 | 0 | 56% |
| **TOTAL** | 108 | 8 | 48 | 3 | **66%** |

---

## PRIORITY TEST GAPS

### 🔴 **P0 - Critical (Blocks Features)**
✅ **ALL P0 FEATURES IMPLEMENTED!**
- Comparison operators `<=`, `>=` - ✅ Working
- `range` - ✅ Working
- Set literals `#{}` - ✅ Working
- Function shorthand `#()` - ✅ Working

### 🟠 **P1 - High (Common Use)**
1. `case` statement
2. Type predicates: `seq?`, `coll?`, `fn?`
3. Map operations: `get-in`, `assoc-in`, `update-in`
4. Dynamic vars edge cases (`binding`, `set!`)

### 🟡 **P2 - Medium (Nice to Have)**
1. `letfn` for mutual recursion
2. More sequence operations: `repeat`, `cycle`, `iterate`
3. Set namespace: `clojure.set` → `clorus.set`
4. Complete test coverage for all stdlib functions

### 🟢 **P3 - Low (Advanced)**
1. Conditional reader `#?`
2. Tagged literals
3. Character literals
4. Ratio types
5. Reader discard `#_` baseline covered by `tests/language/test-reader-discard.clr` (remaining reader forms pending)
