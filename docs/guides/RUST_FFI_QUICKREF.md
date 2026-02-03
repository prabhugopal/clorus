# Clorus Rust FFI - Quick Reference

## 30-Second Quick Start

```bash
# 1. Create Rust library
cargo new --lib my-rust-lib

# 2. Add to Clorus.toml
[rust-dependencies]
my-rust-lib = { path = "../my-rust-lib" }

# 3. Use in Clorus
(use rust.my-rust-lib)
(my-rust-lib/some-function args)
```

## Essential Commands

```bash
clorus run              # Run with JIT (fast)
clorus build            # Build native binary (AOT)
clorus run --debug      # Show debug info
```

## Type Quick Reference

| Rust → Clorus | Example |
|---------------|---------|
| `f64` → Number | `42.0` |
| `String` → String | `"hello"` |
| `bool` → Bool | `true`/`false` |
| `()` → nil | `nil` |

## Async Pattern

```rust
// Rust library
pub async fn async_work(x: T) -> R { ... }

pub fn work_blocking(x: T) -> R {
    block_on(async_work(x))
}
```

```clojure
; Clorus code
(rust-lib/work-blocking arg)  ; Just works!
```

## Performance Rules of Thumb

- **Numeric ops:** ~0ns overhead → Use freely
- **String ops:** ~20-50ns → Batch when possible
- **I/O ops:** ~0% overhead → FFI cost negligible
- **Heavy compute:** ~0% overhead → Work dominates

## Common Gotchas

❌ **Don't:** Write FFI manually → System auto-generates
❌ **Don't:** Call async functions directly → Use blocking wrappers
❌ **Don't:** Convert strings in tight loops → Batch operations

✅ **Do:** Write pure Rust
✅ **Do:** Let system handle FFI
✅ **Do:** Profile before optimizing

## Cargo.toml Template

```toml
[lib]
crate-type = ["cdylib", "staticlib", "rlib"]

[dependencies]
# For async support:
futures = "0.3"
# OR smol = "2.0"
# OR tokio = { version = "1", features = ["full"] }
```

## Debug Checklist

1. ✅ Library has `crate-type = ["cdylib", "staticlib", "rlib"]`
2. ✅ Functions are `pub`
3. ✅ Async functions have blocking wrappers
4. ✅ Run `clorus run --debug` to see what's loaded

## Full Guide

See `RUST_FFI_GUIDE.md` for complete documentation.
