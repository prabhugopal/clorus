# Clorus FFI Troubleshooting Guide

## Common Issues and Solutions

### Issue 1: "Symbol not found" Error

**Symptom:**
```
Error: Symbol clorus_my_function not found
```

**Cause:** Library not compiled with correct crate types

**Solution:**
```toml
# In your Rust library's Cargo.toml
[lib]
crate-type = ["cdylib", "staticlib", "rlib"]
```

Then rebuild:
```bash
cd ~/my-rust-lib
cargo build --release
```

---

### Issue 2: String Functions Crash/Segfault

**Symptom:**
```
Exit code 139 (segmentation fault)
```

**Cause:** Runtime library not loaded (should be automatic now)

**Solution:**
```bash
# Check if runtime is loaded
clorus run --debug

# Should see:
# [DEBUG] Loaded clorus-runtime library

# If not, rebuild runtime:
cd ~/Learning/git/clorus
cargo build -p clorus-runtime --release
```

**Verification:**
```clojure
; Test string literal
"Hello, World!"
; Should output: => "Hello, World!"
```

---

### Issue 3: Async Functions Not Found

**Symptom:**
```
Error: Undefined function: async-lib/async-greet
```

**Cause:** Trying to call async function directly (FFI generator skips them)

**Solution:** Write blocking wrappers

**Wrong:**
```rust
// This won't be wrapped
pub async fn async_greet(name: String) -> String {
    format!("Hello, {}!", name)
}
```

**Correct:**
```rust
pub async fn async_greet(name: String) -> String {
    format!("Hello, {}!", name)
}

// This WILL be wrapped
pub fn greet_blocking(name: String) -> String {
    block_on(async_greet(name))
}
```

```clojure
; Call the blocking version
(async-lib/greet-blocking "World")
```

---

### Issue 4: "ffi.rs not found" During Compilation

**Symptom:**
```
error: file not found for module `ffi`
```

**Cause:** System generated ffi.rs but lib.rs doesn't include it

**Solution:** System should auto-add, but if not:
```rust
// Add to end of lib.rs
pub mod ffi;
```

---

### Issue 5: Network Blocks Dependency Downloads

**Symptom:**
```
error: failed to get `futures` as a dependency
Caused by: [56] Failure when receiving data
```

**Cause:** Network security blocks crates.io

**Solutions:**

**Option 1: Allow crates.io** (recommended)
```bash
# Go to http://localhost:3581
# Add crates.io to allowlist
```

**Option 2: Use cached dependencies**
```bash
# If already downloaded once:
cargo build --offline
```

**Option 3: Pre-download dependencies**
```bash
# Download in unrestricted environment
cd ~/my-rust-lib
cargo fetch
```

---

### Issue 6: Performance is Slower Than Expected

**Symptom:** FFI calls seem slow

**Diagnosis:**
```bash
clorus run --profile  # (when profiling is implemented)
```

**Common Causes:**

**1. String conversions in tight loop**
```clojure
; ❌ Bad - converts 1M times
(loop [i 0]
  (if (< i 1000000)
    (do
      (rust-lib/process "hello")
      (recur (+ i 1)))))

; ✅ Good - batch processing
(rust-lib/process-batch (repeat 1000000 "hello"))
```

**2. Small function calls**
```clojure
; ❌ Bad - FFI overhead dominates
(defn sum-range [n]
  (loop [i 0 total 0]
    (if (< i n)
      (recur (+ i 1) (+ total (rust-lib/add i 1)))
      total)))

; ✅ Good - do work in Rust
(rust-lib/sum-range n)
```

---

### Issue 7: Type Mismatch Errors

**Symptom:**
```
Error: Expected Number, got String
```

**Cause:** Type confusion between Rust and Clorus

**Type Mapping:**
```
Rust f64      → Clorus Number
Rust i32      → Clorus Number (converted)
Rust String   → Clorus String
Rust bool     → Clorus Bool
Rust ()       → Clorus nil
```

**Solution:** Check function signatures match

```rust
// Rust
pub fn add(x: f64, y: f64) -> f64 { ... }
```

```clojure
; Clorus - pass numbers
(my-lib/add 10.0 20.0)  ; ✅
(my-lib/add "10" "20")   ; ❌ Wrong!
```

---

### Issue 8: Multiple Projects Conflict

**Symptom:** FFI changes in one project affect another

**Cause:** Phase 1 generates ffi.rs in library source

**Workaround:** Use separate library copies per project

**Better Solution:** Wait for Phase 2 (generates in target/)

---

### Issue 9: Library Path Issues

**Symptom:**
```
clorus-runtime library not found
```

**Solution:** Library search order:
1. Current project's target/release
2. Current project's target/debug
3. Installed library directory
4. Workspace target directories

**Manual fix:**
```bash
# Copy from Clorus installation
cp ~/Learning/git/clorus/target/release/libclorus_runtime.dylib \
   ~/my-project/target/release/
```

**Permanent fix:** Install Clorus properly
```bash
cd ~/Learning/git/clorus
cargo install --path crates/clorus-cli
```

---

### Issue 10: Generated FFI is Wrong

**Symptom:** Compilation errors in auto-generated ffi.rs

**Diagnosis:**
```bash
cat ~/my-rust-lib/src/ffi.rs
```

**Common Issues:**

**1. Unsupported type**
```rust
// ❌ Not yet supported
pub fn process(data: Vec<String>) -> HashMap<String, i32> { ... }

// ✅ Supported
pub fn process(data: String) -> f64 { ... }
```

**2. Complex return type**
```rust
// ❌ Not yet supported
pub fn fetch() -> Result<String, Error> { ... }

// ✅ Workaround - unwrap in Rust
pub fn fetch() -> String {
    match fetch_impl() {
        Ok(s) => s,
        Err(e) => format!("Error: {}", e)
    }
}
```

---

## Getting Help

### Debug Checklist

Run through this before reporting issues:

```bash
# 1. Enable debug output
clorus run --debug

# 2. Check library types
grep "crate-type" ~/my-rust-lib/Cargo.toml
# Should show: ["cdylib", "staticlib", "rlib"]

# 3. Check functions are public
grep "pub fn" ~/my-rust-lib/src/lib.rs

# 4. Verify FFI was generated
ls ~/my-rust-lib/src/ffi.rs
cat ~/my-rust-lib/src/ffi.rs

# 5. Check runtime is loaded
clorus run --debug 2>&1 | grep runtime

# 6. Test simple case first
echo '42' > test.clrs
clorus run test.clrs
```

### Reporting Bugs

Include:
1. ✅ Full error message
2. ✅ Rust library code (lib.rs)
3. ✅ Cargo.toml
4. ✅ Clorus code that fails
5. ✅ Output of `clorus run --debug`
6. ✅ Generated ffi.rs (if exists)

### Performance Issues

Include:
1. ✅ Profiler output (when available)
2. ✅ Type of operations (numeric/string/I/O)
3. ✅ Frequency of calls
4. ✅ Expected vs actual performance

---

## Known Limitations (Phase 1)

**Not bugs, but design limitations:**

1. ⚠️ ffi.rs generated in library source
2. ⚠️ Can't use crates.io directly
3. ⚠️ Manual blocking wrappers for async
4. ⚠️ Limited type support (f64, i32, String, bool, ())
5. ⚠️ No Result/Option support yet
6. ⚠️ No Vec/HashMap support yet

**All will be addressed in Phase 2!**

---

## Success Indicators

Your FFI setup is correct if:

✅ `clorus run --debug` shows "Loaded clorus-runtime library"
✅ String literals display correctly: `"hello"` → `=> "hello"`
✅ Numbers work: `42` → `=> 42`
✅ Your Rust functions are callable
✅ No segfaults or crashes
✅ Generated ffi.rs compiles without errors

---

## Quick Fixes Summary

| Problem | Quick Fix |
|---------|-----------|
| Symbol not found | Add `crate-type = ["cdylib", "staticlib", "rlib"]` |
| Segfault | Check runtime loaded: `clorus run --debug` |
| Async not found | Write blocking wrapper |
| Network error | Download deps manually or allow crates.io |
| Slow performance | Batch operations, move work to Rust |
| Type mismatch | Check Rust-Clorus type mapping |
| Library not found | Check target/ or copy manually |

---

## Still Stuck?

1. Read `RUST_FFI_GUIDE.md` - Full documentation
2. Check examples in `~/Learning/clorus/async-hello/`
3. Run `clorus run --debug` and read output carefully
4. Try the minimal test case in this guide
5. Report issue with debug info

**Most issues are solved by ensuring:**
- Correct Cargo.toml crate types
- Public functions
- Runtime library loads
- Blocking wrappers for async
