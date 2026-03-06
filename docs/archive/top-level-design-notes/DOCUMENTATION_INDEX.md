# Clorus Documentation Index

**Purpose:** Navigate the documentation maze - find what you need quickly

**Start here:** `STATUS.md` - Canonical docs map

---

## 🎯 Quick Links

| I Want To... | Go To |
|--------------|-------|
| **Find canonical docs quickly** | `STATUS.md` |
| **See what's implemented** | `reference/LANGUAGE_SPEC.md` |
| **Compare to Clojure** | `LANGUAGE_PARITY.md` |
| **See current parity checklist (paired vs gaps)** | `PARITY_CHECKLIST.md` |
| **See execution plan to parity** | `PARITY_EXECUTION_PLAN.md` |
| **See docs cleanup plan** | `DOCS_CONSOLIDATION_PLAN.md` |
| **See exception semantics plan** | `EXCEPTION_SEMANTICS_PLAN.md` |
| **Learn a specific feature** | See [Feature Guides](#feature-guides) below |
| **Understand the roadmap** | `ROADMAP_TO_100_PARITY.md` |
| **See recent changes** | `sessions/` folder |
| **Understand hardcoded values** | `HARDCODED_VALUES_ANALYSIS.md` |
| **Understand architecture** | `ARCHITECTURE_ANALYSIS.md` |

---

## 📚 Core Documentation

### Primary References (Read These First)
1. **`reference/LANGUAGE_SPEC.md`** ⭐ - Complete language specification
   - All implemented features
   - Quick reference
   - Examples
   - Current limitations

2. **`LANGUAGE_PARITY.md`** - Detailed Clojure comparison
   - Feature-by-feature breakdown
   - Coverage percentages
   - Progress timeline

3. **`PARITY_CHECKLIST.md`** - Current parity truth table
   - Paired vs partial vs missing
   - Dual-engine validation baseline
   - Next milestone checklist

4. **`ROADMAP_TO_100_PARITY.md`** - Path forward
   - What's missing
   - Implementation priorities
   - Timeline estimates

5. **`PARITY_EXECUTION_PLAN.md`** - Current execution plan
   - Dual-engine baseline and gating commands
   - Milestones for compiler/runtime/repl hardening
   - Priority backlog and definition of done

### Canonical Operations Docs
- `PARITY_CHECKLIST.md` — current feature status
- `PARITY_EXECUTION_PLAN.md` — active execution plan
- `DOCS_CONSOLIDATION_PLAN.md` — docs merge/archive policy
- `tests/FEATURE_COVERAGE.md` — test inventory map

---

## 🗃️ Historical Docs

Many legacy docs are now marked with:
- `Status: Archived (Historical)`
- `Canonical replacement: <path>`

Use them for implementation history only, not current status.

---

## 📅 Session Notes (What Changed When)

Recent sessions in reverse chronological order:

- **Jan 29, 2026**: `sessions/SESSION_COLLECTIONS_LAZY_SEQUENCES.md`
  - Type predicates (all 15)
  - Eager sequences (map, filter, reduce, doall)

- **Jan 28, 2026**: `sessions/SESSION_COMPLETE_2026_01_25.md`
  - Polymorphism system (records, protocols, multimethods)

- **Jan 27, 2026** (Evening): `LAZY_SEQUENCES_COMPLETE.md`
  - Full lazy sequence library

- **Jan 27, 2026**: `PHASE_C_COMPLETE.md`
  - Polymorphism foundation

- **Older**: See `sessions/` folder for complete history

---

## 🏗️ Implementation Phases

### Completed Phases
- ✅ `PHASE1_COMPLETE.md` - Core language
- ✅ `PHASE2A_COMPLETE.md` - Collections
- ✅ `PHASE2B_COMPLETE.md` - Extended collections
- ✅ `PHASE_B_COMPLETE.md` - Control flow
- ✅ `PHASE_C_COMPLETE.md` - Polymorphism

### Current Phase
- 🔄 **Phase D**: Standard library expansion

---

## 🎓 Learning Path

### Beginner
1. Read `reference/LANGUAGE_SPEC.md` - Overview
2. Try examples in REPL
3. Look at `tests/` folder for examples

### Intermediate
1. Read `LANGUAGE_PARITY.md` - Understand coverage
2. Read feature guides for areas you need
3. Check `examples/` folder

### Advanced
1. Read implementation docs in `implementation/`
2. Read session notes for context
3. Check codebase directly

---

## 🗂️ Documentation Structure

```
docs/
├── LANGUAGE_SPEC.md           ⭐ START HERE
├── LANGUAGE_PARITY.md         Detailed comparison
├── ROADMAP_TO_100_PARITY.md   What's next
│
├── features/                  Feature-specific docs
│   ├── REPL_FEATURES.md
│   ├── STRING_OPERATIONS_COMPLETE.md
│   ├── AGENTS_COMPLETE.md
│   ├── CHANNELS_PHASE1_COMPLETE.md
│   └── ...
│
├── implementation/            Implementation details
│   └── LOOP_AND_ATOMS_COMPLETE.md
│
├── design/                    Design documents
│   └── RUST_INTEROP_ROADMAP.md
│
├── sessions/                  Session notes (what changed when)
│   ├── SESSION_COLLECTIONS_LAZY_SEQUENCES.md (Jan 29)
│   ├── SESSION_COMPLETE_2026_01_25.md (Jan 28)
│   └── ...
│
└── guides/                    How-to guides
    └── AUTOCOMPLETE_TEST_GUIDE.md
```

---

## 📝 Notes on Documentation

### Duplicates and Consolidation
Many docs have overlapping content. This is intentional:
- **Detailed docs** (e.g., `AGENTS_COMPLETE.md`) - Deep dive on one feature
- **Session docs** (e.g., `SESSION_*`) - What changed, context, decisions
- **Spec docs** (e.g., `reference/LANGUAGE_SPEC.md`) - Consolidated reference

**Rule of thumb:**
- Want overview? → `reference/LANGUAGE_SPEC.md`
- Want feature details? → `features/` folder
- Want implementation context? → `sessions/` folder
- Want to know what changed? → `sessions/` folder (recent first)

### Maintenance
- `reference/LANGUAGE_SPEC.md` - Update when features added
- `LANGUAGE_PARITY.md` - Update percentage when major features complete
- `sessions/` - Add new session doc after major work
- Feature docs - Update when feature changes significantly

---

## 🔍 Finding What You Need

### By Topic

**Collections:**
- Operations: `COLLECTION_ACCESS_COMPLETE.md`
- Lazy: `LAZY_SEQUENCES_COMPLETE.md`
- HOF: `HOF_IMPLEMENTATION_COMPLETE.md`

**State:**
- Atoms: `LOOP_AND_ATOMS_COMPLETE.md`
- Refs: `REFS_STM_COMPLETE.md`
- Agents: `AGENTS_COMPLETE.md`
- Channels: `CHANNELS_PHASE1_COMPLETE.md`

**Polymorphism:**
- Everything: `PHASE_C_COMPLETE.md`

**Macros:**
- Quote: `QUOTE_IMPLEMENTATION_COMPLETE.md`
- Threading: `THREADING_MACROS_COMPLETE.md`
- Shorthand: `SHORTHAND_FN_COMPLETE.md`

**FFI:**
- Overview: `FFI_ROADMAP.md`
- Design: `design/RUST_INTEROP_ROADMAP.md`

### By Date

Check `sessions/` folder, files are named with dates or have "Last Updated" in content.

### By Implementation Status

- ✅ Complete: Look for `*_COMPLETE.md` files
- 🔄 In Progress: Look for `*_IN_PROGRESS.md` files
- 📋 Planned: Check `ROADMAP_TO_100_PARITY.md`

---

## 🚀 For Contributors

### Adding a New Feature
1. Implement feature
2. Test thoroughly
3. Create `features/FEATURE_NAME_COMPLETE.md`
4. Update `reference/LANGUAGE_SPEC.md`
5. Update `LANGUAGE_PARITY.md` percentages
6. Create `sessions/SESSION_DESCRIPTION.md` with context

### Writing Documentation
- **Be concise** - Code examples speak louder
- **Show examples** - Every feature needs an example
- **Mark status** - ✅ Complete, ⚠️ Partial, ❌ Missing
- **Link related docs** - Help people navigate

---

## 📊 Statistics

**Total Documentation Files:** ~47+
**Primary References:** 3
**Architecture Docs:** 2
**Feature Guides:** ~25
**Session Notes:** ~15
**Implementation Guides:** ~10

**Most Important:** 1 (LANGUAGE_SPEC.md)

---

**Last Updated:** February 3, 2026
**Maintainers:** Prabhu Gopal, Claude Code
