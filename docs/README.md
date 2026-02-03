# Clorus Documentation

**Complete documentation for the Clorus language**

---

## 📚 Quick Links

### For Users

- **[Getting Started](guide/GETTING_STARTED.md)** - Installation and first steps *(coming soon)*
- **[Language Guide](guide/LANGUAGE_GUIDE.md)** - Complete language tutorial *(coming soon)*
- **[Standard Library Guide](guide/STDLIB_GUIDE.md)** - Stdlib reference *(coming soon)*
- **[Rust FFI Guide](guide/RUST_FFI_GUIDE.md)** - Calling Rust from Clorus

### For Developers

- **[Language Specification](reference/LANGUAGE_SPEC.md)** - Formal language spec
- **[Standard Library API](reference/STDLIB_API.md)** - Complete API reference *(coming soon)*
- **[FFI Reference](reference/FFI_REFERENCE.md)** - FFI details *(coming soon)*

### Implementation & Roadmap

- **[Stdlib Enhancement Plan](implementation/STDLIB_ENHANCEMENT_PLAN.md)** - Roadmap for stdlib completion
- **[Feature Matrix](implementation/FEATURE_MATRIX.md)** - Feature coverage vs Clojure
- **[Coverage Assessment](implementation/COVERAGE_ASSESSMENT.md)** - Gap analysis
- **[Repository Cleanup Plan](implementation/REPOSITORY_CLEANUP_PLAN.md)** - Organization guide

---

## 📖 Documentation Structure

```
docs/
├── guide/              # User guides (tutorials, how-tos)
├── reference/          # Reference documentation (specs, APIs)
├── implementation/     # Implementation plans and roadmaps
├── design/            # Architecture and design decisions
├── features/          # Feature-specific documentation
│   ├── concurrency/
│   ├── collections/
│   ├── macros/
│   ├── repl/
│   └── exceptions/
└── sessions/          # Development session notes
    └── 2025-01/
```

---

## 🎯 Current Focus

**Priority:** Standard library enhancement and number type system

See [implementation/STDLIB_ENHANCEMENT_PLAN.md](implementation/STDLIB_ENHANCEMENT_PLAN.md) for details.

---

## 📊 Status

- **Core Language:** ~80% complete
- **Standard Library:** ~25% complete (actively improving)
- **Concurrency:** 100% complete
- **Collections:** 75% complete

See [implementation/FEATURE_MATRIX.md](implementation/FEATURE_MATRIX.md) for full breakdown.
