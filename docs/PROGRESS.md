# Clorus Feature Implementation Progress

**Last Updated:** January 27, 2026
**Reference:** Based on Jank Programming Language checklist
**Current Completion:** 40% (18/45 features)

---

## 🎯 Current Sprint: Foundation (Week 1-4)

### Layer 0: Absolute Foundation

| # | Feature | Lex | Parse | Analyze | Eval | Status | Assignee |
|---|---------|-----|-------|---------|------|--------|----------|
| 1 | **fn*/base** | ✅ | ✅ | ✅ | ✅ | ✅ **DONE** (Jan 26) | - |
| 2 | **reader/shorthand fns** | ✅ | ✅ | ✅ | ✅ | ✅ **DONE** (Jan 27) | - |
| 3 | **do** | ✅ | ✅ | ✅ | ✅ | ✅ **DONE** (Jan 26) | - |
| 4 | **quote** | ❌ | ❌ | ❌ | ❌ | 🔵 Next up | - |

**Sprint Goal:** By end of Week 4, can write functional code with lambdas

---

## 📊 Implementation Status by Category

### Data Types (58% complete - 7/12)

| Feature | Status | Priority | Notes |
|---------|--------|----------|-------|
| nil | ✅ Done | - | Complete |
| integers | 🟨 Partial | 🟡 Medium | Only f64, need i64 |
| reals | ✅ Done | - | f64 complete |
| bools | ✅ Done | - | true/false |
| chars | ❌ Missing | 🟡 Low | `\a` `\newline` |
| strings | ✅ Done | - | String literals |
| keywords/unqualified | ✅ Done | - | `:name` with interning |
| keywords/qualified | ❌ Missing | 🔥 High | `:user/name` |
| keywords/auto-resolved | ❌ Missing | 🔥 High | `::name` |
| maps | ✅ Done | - | `{:a 1}` literals |
| vectors | ✅ Done | - | `[1 2 3]` literals |
| sets | ❌ Missing | 🔥 High | `#{1 2 3}` |
| lists | ✅ Done | - | `'(1 2 3)` |
| symbols | ✅ Done | - | Variable names |
| ratios | ❌ Missing | 🟡 Low | `22/7` |

---

### Special Forms (21% complete - 4/17)

| Feature | Status | Priority | Blocks |
|---------|--------|----------|--------|
| **def** | ✅ Done | - | - |
| **if** | ✅ Done | - | - |
| **do** | ✅ Done | - | ✅ Sequential eval |
| **let*** | ✅ Done | - | - |
| **quote** | ❌ Missing | 🔥🔥 | Macros |
| **var** | ❌ Missing | 🟡 | - |
| **fn*/base** | ✅ Done | - | ✅ UNLOCKS HOFs! |
| **fn*/arities** | ❌ Missing | 🔥🔥 | Multi-arity fns |
| **fn*/variadic** | ❌ Missing | 🔥🔥 | `& args` |
| **fn*/recur** | ❌ Missing | 🔥 | TCO |
| **loop*** | ❌ Missing | 🔥 | Iteration |
| **loop*/recur** | ❌ Missing | 🔥 | TCO |
| **throw** | ❌ Missing | 🟡 | Error handling |
| **try** | ❌ Missing | 🟡 | Error handling |
| **set!** | ❌ Missing | 🟡 | Mutation |
| **case*** | ❌ Missing | 🟡 | Pattern match |
| **letfn*** | ❌ Missing | 🟡 | Mutual recursion |

---

### Language Features (43% complete - 3/7)

| Feature | Status | Priority | Notes |
|---------|--------|----------|-------|
| bindings/thread-local | ✅ Done | - | Via let |
| bindings/conveyance | ✅ Done | - | Via closures |
| calls | ✅ Done | - | Function calls |
| destructuring | ❌ Missing | 🔥 | Vector & map |
| macros | 🟨 Partial | 🔥🔥 | Only -> and ->> |
| macros/&env | ❌ Missing | 🟡 | - |
| syntax-quoting | ❌ Missing | 🔥🔥 | `` ` `` |
| unquote | ❌ Missing | 🔥🔥 | `~` |

---

### Reader Macros (11% complete - 1/9)

| Feature | Status | Priority | Example |
|---------|--------|----------|---------|
| comment | ✅ Done | - | `; comment` |
| set | ❌ Missing | 🔥 | `#{1 2 3}` |
| shorthand fns | ✅ Done | - | `#(* % 2)` ✅ |
| regex | ❌ Missing | 🟡 | `#"[a-z]+"` |
| deref | ❌ Missing | 🔥 | `@atom` |
| quote | ❌ Missing | 🔥 | `'(1 2 3)` |
| var quote | ❌ Missing | 🟡 | `#'var` |
| conditional | ❌ Missing | 🟡 | `#?(:clj ...)` |
| tagged literal | ❌ Missing | 🟡 | `#inst "..."` |

---

## 🏆 Milestones

### Milestone 1: Foundation Complete (Week 4)
**Target Date:** Feb 23, 2026
**Status:** ✅ COMPLETE (100% complete - 4/4)

**Criteria:**
- [x] Threading macros (-> and ->>) ✅ DONE
- [x] fn*/base (lambdas) ✅ DONE (Jan 26, 2026)
- [x] reader/shorthand fns (#()) ✅ DONE (Jan 27, 2026)
- [x] do (multiple expressions) ✅ DONE (Jan 26, 2026)

**Unlock:** Can write functional code!

---

### Milestone 2: Collections Functional (Week 8)
**Target Date:** Mar 23, 2026
**Status:** 🔴 Not Started

**Criteria:**
- [ ] sets (#{1 2 3})
- [ ] loop*/recur
- [ ] destructuring
- [ ] qualified keywords

**Unlock:** Collections are fully usable!

---

### Milestone 3: Macro System (Week 16)
**Target Date:** May 18, 2026
**Status:** 🔴 Not Started

**Criteria:**
- [ ] syntax-quoting
- [ ] defmacro
- [ ] unquote/unquote-splicing
- [ ] fn*/arities
- [ ] fn*/variadic

**Unlock:** Users can define macros!

---

### Milestone 4: Production Ready (Week 24)
**Target Date:** Jul 13, 2026
**Status:** 🔴 Not Started

**Criteria:**
- [ ] try/catch/finally
- [ ] case*
- [ ] All critical features complete
- [ ] Comprehensive stdlib
- [ ] Documentation

**Unlock:** Production-ready language!

---

## 📅 Week-by-Week Plan

### Week 1 (Jan 27 - Feb 2, 2026)
**Focus:** fn*/base

- [ ] Day 1-2: Update AST with Fn variant
- [ ] Day 3-4: Parser for (fn [params] body)
- [ ] Day 5: Basic codegen (no closures)
- [ ] Day 6-7: Closure support

**Deliverable:** `(fn [x] (* x 2))` works!

---

### Week 2 (Feb 3 - 9, 2026)
**Focus:** Shorthand fns + do

- [ ] Day 1-2: Reader macro #() → fn
- [ ] Day 3: Parse do form
- [ ] Day 4: Codegen for do
- [ ] Day 5-7: Testing & integration

**Deliverable:** `#(* % 2)` and `(do ...)` work!

---

### Week 3 (Feb 10 - 16, 2026)
**Focus:** quote + sets

- [ ] Day 1-2: Parse quote / '
- [ ] Day 3-4: Lexer/parser for sets
- [ ] Day 5-6: Runtime set type
- [ ] Day 7: Testing

**Deliverable:** `'(1 2 3)` and `#{1 2 3}` work!

---

### Week 4 (Feb 17 - 23, 2026)
**Focus:** Integration

- [ ] Day 1-3: Comprehensive testing
- [ ] Day 4-5: REPL integration
- [ ] Day 6-7: Documentation

**Deliverable:** Milestone 1 complete! 🎉

---

## 🚧 Blockers & Dependencies

### Critical Path

```
fn*/base
  ├─→ BLOCKS: map, filter, reduce
  ├─→ BLOCKS: shorthand fns
  ├─→ BLOCKS: fn*/arities
  └─→ BLOCKS: fn*/variadic

quote
  └─→ BLOCKS: syntax-quoting
      └─→ BLOCKS: defmacro

do
  └─→ BLOCKS: multiple expressions in try/catch
```

### Current Blockers

| Feature | Blocked By | ETA |
|---------|------------|-----|
| HOFs (map/filter/reduce) | fn*/base | Week 1 |
| Shorthand fns (#()) | fn*/base | Week 1 |
| syntax-quoting | quote | Week 3 |
| defmacro | syntax-quoting | Week 16 |

---

## 📈 Progress Metrics

### Features by Status

```
✅ Done:        18 (40%)
🟨 Partial:     1  (2%)
❌ Missing:     26 (58%)
─────────────────────────
Total:          45 (100%)
```

### By Priority

```
🔥🔥🔥 Critical:  3 features
🔥🔥 High:       8 features
🔥 Medium-High:  7 features
🟡 Medium:       6 features
🟡 Low:          7 features
```

### Velocity

- **Current:** 2-3 features/week (with docs)
- **Target:** 3-4 features/week
- **Estimated completion:** 24-32 weeks

---

## 🎯 This Week's Tasks

### Primary (Must Complete)

1. [x] **fn*/base implementation** ✅ COMPLETED (Jan 26, 2026)
   - [x] AST update
   - [x] Parser
   - [x] Codegen (basic)
   - [x] Tests

2. [x] **reader/shorthand fns (#())** ✅ COMPLETED (Jan 27, 2026)
   - [x] Lexer support for `#(` token
   - [x] Parser transforms `#(* % 2)` → `(fn [%] (* % 2))`
   - [x] Handle `%1`, `%2` arguments (nested fns too!)
   - [x] Tests (4 test cases, all passing)
   - [x] Fixed infinite loop bug in lexer (% and & recognition)
   - [x] Fixed stack overflow bug in parser (token consumption)

### Secondary (Nice to Have)

3. [ ] Plan `do` implementation
4. [ ] Review closure capture design

### Documentation

- [ ] Update FEATURE_MATRIX.md
- [ ] Create fn implementation guide
- [ ] Add examples to docs

---

## 📝 Notes

### Decision Log

**2026-01-26:** Decided to prioritize fn*/base over everything else
- Rationale: Blocks 90% of useful features
- Alternative considered: Start with stdlib functions
- Decision: fn first, then stdlib can use it

**2026-01-26:** Using Jank checklist as reference
- Rationale: Well-structured, battle-tested
- Source: https://jank-lang.org/progress/

### Open Questions

1. **Integer types:** Add i64/i32 or stay with f64?
   - Leaning towards: Add i64 for Rust interop

2. **Ratios:** Implement 22/7 support?
   - Leaning towards: Skip (rarely used)

3. **Chars:** Full char type or just single-char strings?
   - Leaning towards: Single-char strings (simpler)

---

## 🔗 References

- **Feature Matrix:** `docs/FEATURE_MATRIX.md`
- **Roadmap:** `docs/DEPENDENCY_ORDERED_ROADMAP.md`
- **Jank Progress:** https://jank-lang.org/progress/
- **Clojure Spec:** https://clojure.org/reference

---

**Legend:**
- ✅ Done
- 🟨 Partial
- ❌ Missing
- 🔵 In Progress
- ⏸️ Blocked
- 🔥🔥🔥 Critical Priority
- 🔥🔥 High Priority
- 🔥 Medium-High Priority
- 🟡 Medium/Low Priority
