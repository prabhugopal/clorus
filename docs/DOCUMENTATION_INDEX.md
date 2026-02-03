# Clorus Documentation Index

**Purpose:** Navigate the documentation maze - find what you need quickly

**Start here:** `LANGUAGE_SPEC.md` - Single source of truth

---

## 🎯 Quick Links

| I Want To... | Go To |
|--------------|-------|
| **See what's implemented** | `LANGUAGE_SPEC.md` |
| **Compare to Clojure** | `LANGUAGE_PARITY.md` |
| **Learn a specific feature** | See [Feature Guides](#feature-guides) below |
| **Understand the roadmap** | `ROADMAP_TO_100_PARITY.md` |
| **See recent changes** | `sessions/` folder |

---

## 📚 Core Documentation

### Primary References (Read These First)
1. **`LANGUAGE_SPEC.md`** ⭐ - Complete language specification
   - All implemented features
   - Quick reference
   - Examples
   - Current limitations

2. **`LANGUAGE_PARITY.md`** - Detailed Clojure comparison
   - Feature-by-feature breakdown
   - Coverage percentages
   - Progress timeline

3. **`ROADMAP_TO_100_PARITY.md`** - Path forward
   - What's missing
   - Implementation priorities
   - Timeline estimates

---

## 🔧 Feature Guides

### Collections
- `COLLECTION_ACCESS_COMPLETE.md` - Collection operations (get, nth, assoc, etc.)
- `HASHSET_IMPLEMENTATION_COMPLETE.md` - Set implementation
- `HOF_IMPLEMENTATION_COMPLETE.md` - Higher-order functions
- `LAZY_SEQUENCES_COMPLETE.md` - Lazy sequence library

### Control Flow & Macros
- `THREADING_MACROS_COMPLETE.md` - ->>, ->, some->>, doto
- `QUOTE_IMPLEMENTATION_COMPLETE.md` - Quote, syntax-quote, unquote
- `SHORTHAND_FN_COMPLETE.md` - #() syntax

### State Management
- `implementation/LOOP_AND_ATOMS_COMPLETE.md` - Loop/recur and atoms
- `features/REFS_STM_COMPLETE.md` - Software Transactional Memory
- `features/AGENTS_COMPLETE.md` - Agent system
  - `AGENTS_PHASE1_COMPLETE.md` - Basic agents
  - `AGENTS_PHASE2_COMPLETE.md` - Error handling
  - `AGENTS_PHASE3_COMPLETE.md` - Validation
  - `AGENTS_PHASE4_COMPLETE.md` - Thread pools
- `features/CHANNELS_PHASE1_COMPLETE.md` - CSP channels

### Polymorphism
- `PHASE_C_COMPLETE.md` - Records, protocols, multimethods

### Strings & I/O
- `features/STRING_OPERATIONS_COMPLETE.md` - String manipulation

### Namespaces
- `NAMESPACE_VALIDATION_COMPLETE.md` - Namespace system

### FFI
- `FFI_ROADMAP.md` - Rust FFI integration
- `design/RUST_INTEROP_ROADMAP.md` - Rust interop design

### REPL
- `features/REPL_FEATURES.md` - REPL capabilities
- `features/AUTOCOMPLETE_COMPLETE.md` - Tab completion
- `guides/AUTOCOMPLETE_TEST_GUIDE.md` - Testing autocomplete

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
1. Read `LANGUAGE_SPEC.md` - Overview
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
- **Spec docs** (e.g., `LANGUAGE_SPEC.md`) - Consolidated reference

**Rule of thumb:**
- Want overview? → `LANGUAGE_SPEC.md`
- Want feature details? → `features/` folder
- Want implementation context? → `sessions/` folder
- Want to know what changed? → `sessions/` folder (recent first)

### Maintenance
- `LANGUAGE_SPEC.md` - Update when features added
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
4. Update `LANGUAGE_SPEC.md`
5. Update `LANGUAGE_PARITY.md` percentages
6. Create `sessions/SESSION_DESCRIPTION.md` with context

### Writing Documentation
- **Be concise** - Code examples speak louder
- **Show examples** - Every feature needs an example
- **Mark status** - ✅ Complete, ⚠️ Partial, ❌ Missing
- **Link related docs** - Help people navigate

---

## 📊 Statistics

**Total Documentation Files:** ~45+
**Primary References:** 3
**Feature Guides:** ~25
**Session Notes:** ~15
**Implementation Guides:** ~10

**Most Important:** 1 (LANGUAGE_SPEC.md)

---

**Last Updated:** January 29, 2026
**Maintainers:** Prabhu Gopal, Claude Code
