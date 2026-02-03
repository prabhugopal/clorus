# Clorus Test Suite

## Organization

Tests are organized by component and feature for easy navigation and maintenance.

### Directory Structure

```
tests/
├── lang/                  # Core language features
│   ├── core/             # Functions, closures, higher-order functions
│   ├── macros/           # Macro system (defmacro, expansion)
│   ├── collections/      # Collection operations
│   └── special-forms/    # Special forms (def, if, let, do)
│
├── stdlib/               # Standard library tests
│   ├── core/            # Core utilities (predicates, sequences)
│   ├── lazy/            # Lazy sequences
│   ├── transducers/     # Transducer operations
│   ├── math/            # Math library
│   └── string/          # String operations
│
├── concurrency/          # Concurrency primitives
│   ├── atoms/           # Atomic references (swap!, reset!)
│   ├── refs/            # STM transactions (dosync, alter)
│   ├── agents/          # Asynchronous agents
│   └── channels/        # CSP channels (go blocks)
│
├── polymorphism/         # Protocols, records, multimethods
├── types/               # Type predicates and checking
├── exceptions/          # Error handling (try/catch/throw)
├── repl/                # REPL-specific features
└── integration/         # End-to-end integration tests
```

## Naming Conventions

- **Test files**: Use descriptive names: `feature-name.clr`
- **Consolidated tests**: Multiple small tests combined into logical groups
- **One test per feature**: Each file focuses on a specific feature area

## Running Tests

```bash
# Run all tests
cargo test

# Run specific test category
cargo test lang
cargo test stdlib
cargo test concurrency

# Run specific test file
cargo test atoms
```

## Adding New Tests

1. Determine the appropriate category
2. Create or add to existing test file in that category
3. Follow existing test patterns
4. Ensure tests are self-contained and repeatable

## Test Organization Goals

✅ Easy to find specific tests
✅ Clear separation by feature
✅ Logical grouping of related tests
✅ Clean repository structure
✅ Professional organization
