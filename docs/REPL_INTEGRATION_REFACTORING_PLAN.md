# REPL Integration Refactoring Plan

## Executive Summary

**Current Architecture:** Standalone `repl` binary (separate from `clorus` binary)
**Target Architecture:** Integrated `clorus repl` subcommand (single binary)
**Estimated Effort:** 4-6 hours
**Risk Level:** Medium (requires careful testing)
**Benefits:** Professional architecture, easier distribution, better UX

---

## Current State Analysis

### What Works
✅ `repl` binary functions correctly
✅ Loads 76 stdlib functions
✅ All core features working
✅ Comparison operators return booleans
✅ Collection operations work correctly
✅ Error handling functional

### Architecture Problem
```
Current (Non-Professional):
~/.clorus/bin/
├── clorus      # Main CLI tool
└── repl        # Separate REPL binary  <-- Problem!

# Usage:
$ repl                    # Launches standalone REPL
$ clorus repl            # Shells out to 'repl' binary

What Happens:
1. User runs 'clorus repl'
2. clorus-cli/commands.rs::repl() function executes
3. It shells out using std::process::Command to launch 'repl' binary
4. Separate process with separate memory space
```

### Why This Is Bad
1. ❌ **Two binaries to maintain** - PATH, installation, distribution
2. ❌ **Process overhead** - Spawning new process, separate memory
3. ❌ **Not industry standard** - No modern language does this
4. ❌ **Duplication** - Both binaries link same libraries, force-link same FFI symbols
5. ❌ **User confusion** - Which binary do they use?
6. ❌ **Packaging complexity** - Need to ship both binaries

---

## Target Architecture

```
Professional:
~/.clorus/bin/
└── clorus      # Single unified binary

# Usage:
$ clorus repl             # Direct invocation, no spawning
$ clorus build
$ clorus run
$ clorus new my-app

How It Works:
1. User runs 'clorus repl'
2. clorus-cli/main.rs routes to REPL command
3. Calls clorus_repl::run() library function directly
4. Everything in same process, same memory space
```

### Industry Standard Examples
```bash
# Rust
cargo build / cargo run / cargo test    # Single 'cargo' binary

# Go
go build / go run / go test             # Single 'go' binary

# Node
node script.js / node --eval "..."      # Single 'node' binary

# Deno
deno run / deno repl / deno test        # Single 'deno' binary

# Python
python script.py / python -i            # Single 'python' binary
```

---

## Implementation Plan

### Phase 1: Restructure clorus-repl Crate (1 hour)

**Current:**
```toml
# crates/clorus-repl/Cargo.toml
[[bin]]
name = "repl"
path = "src/main.rs"
```

**Target:**
```toml
# crates/clorus-repl/Cargo.toml
[lib]
name = "clorus_repl"
path = "src/lib.rs"

# Optional: Keep binary for development/testing
[[bin]]
name = "repl-dev"
path = "src/bin/main.rs"
```

**Changes:**
1. Create `src/lib.rs`:
```rust
// crates/clorus-repl/src/lib.rs
mod repl_engine;
mod project;

pub use repl_engine::ReplEngine;
pub use project::ProjectConfig;

// Main entry point for REPL
pub fn run() -> Result<(), String> {
    // Move all main() logic here
    run_repl_with_config(None)
}

pub fn run_repl_with_config(config: Option<ReplConfig>) -> Result<(), String> {
    // Full REPL implementation
    // All the logic currently in main.rs
}

pub struct ReplConfig {
    pub history_file: Option<String>,
    pub stdlib_path: Option<String>,
    pub no_banner: bool,
    // ... other config options
}
```

2. Move `src/main.rs` → `src/bin/main.rs` (optional dev binary):
```rust
// crates/clorus-repl/src/bin/main.rs
fn main() {
    if let Err(e) = clorus_repl::run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
```

### Phase 2: Update clorus-cli Integration (1 hour)

**Update Cargo.toml:**
```toml
# crates/clorus-cli/Cargo.toml
[dependencies]
clorus-repl = { path = "../clorus-repl" }
# ... other deps
```

**Update commands.rs:**
```rust
// crates/clorus-cli/src/commands.rs
pub fn repl() -> Result<(), String> {
    // OLD (shells out):
    // use std::process::Command;
    // Command::new("repl").spawn()...

    // NEW (direct call):
    clorus_repl::run()
}
```

### Phase 3: Handle FFI Symbol Duplication (30 min)

**Problem:** Both binaries force-link the same FFI symbols

**Solution:** Move all FFI force-linking to ONE place

```rust
// Create: crates/clorus-runtime/src/force_link.rs
#[used]
static FORCE_LINK_ATOM: unsafe extern "C" fn(*mut Value) -> *mut Value = clorus_runtime::atom::clorus_atom;
// ... all other force-linked symbols ...

// In clorus-cli/src/main.rs:
mod force_link {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../clorus-runtime/src/force_link.rs"));
}
```

### Phase 4: Update install.sh (15 min)

```bash
# install.sh
echo "📦 Copying binaries..."
cp target/release/clorus "$BIN_DIR/"
# REMOVE: cp target/release/repl "$BIN_DIR/"
chmod +x "$BIN_DIR/clorus"
```

### Phase 5: Update Documentation (30 min)

1. Update README.md
2. Update REPL documentation
3. Update installation instructions
4. Migration guide for existing users

### Phase 6: Testing (2 hours)

**Test Matrix:**
```
✓ clorus repl works (direct invocation)
✓ All 76 stdlib functions load
✓ REPL commands work (:quit, :help, :examples)
✓ History file created correctly
✓ Autocomplete works
✓ Error messages correct
✓ Project context detected
✓ FFI libraries load correctly
✓ No performance regression
```

**Automated Test:**
```bash
# Run test suite
./test-repl.sh "clorus repl"

# Compare against baseline
diff baseline-results.txt new-results.txt
```

---

## Risk Mitigation

### Risk 1: FFI Symbol Issues
**Probability:** High
**Impact:** Critical (REPL won't work)

**Mitigation:**
- Test FFI loading immediately
- Have rollback plan ready
- Test all concurrency primitives (atoms, refs, agents, channels)

### Risk 2: Library Loading Path Issues
**Probability:** Medium
**Impact:** High (libraries won't load)

**Mitigation:**
- Test library loading in different contexts
- Verify CLORUS_HOME, ~/.clorus paths
- Test in project directory vs standalone

### Risk 3: Breaking Existing Users
**Probability:** Low
**Impact:** Medium (users have scripts using `repl`)

**Mitigation:**
- Keep `repl-dev` binary temporarily
- Provide migration guide
- Create symlink `repl` → `clorus repl` during install

---

## Migration Path for Users

### Option 1: Symlink (Backward Compatible)
```bash
# In install.sh
ln -sf "$BIN_DIR/clorus" "$BIN_DIR/repl-wrapper"
# Create wrapper script
cat > "$BIN_DIR/repl" << 'EOF'
#!/bin/bash
exec clorus repl "$@"
EOF
chmod +x "$BIN_DIR/repl"
```

### Option 2: Deprecation Warning
```bash
# If old 'repl' binary exists, warn user
if [ -f "$BIN_DIR/repl" ]; then
    echo "⚠️  WARNING: Standalone 'repl' binary is deprecated"
    echo "    Use 'clorus repl' instead"
    echo "    The 'repl' binary will be removed in v0.3.0"
fi
```

---

## Success Criteria

✅ **Architecture**
- Single `clorus` binary
- `clorus repl` launches REPL in same process
- No process spawning overhead

✅ **Functionality**
- All tests pass (30/30)
- No regressions
- Same user experience

✅ **Performance**
- No startup time regression
- Memory usage comparable or better

✅ **Distribution**
- Single binary to install
- Simpler PATH management
- Professional appearance

---

## Rollback Plan

If integration fails:

1. **Immediate:**
   ```bash
   git stash  # Save changes
   cargo build --release --bin repl  # Rebuild standalone
   ```

2. **Investigation:**
   - Review FFI symbol loading
   - Check library paths
   - Verify configuration

3. **Fix Forward:**
   - Address issues in separate branch
   - Re-test thoroughly
   - Merge when stable

---

## Timeline

**Total Estimated Time: 4-6 hours**

- Phase 1: Restructure clorus-repl (1 hour)
- Phase 2: Update clorus-cli (1 hour)
- Phase 3: FFI symbols (30 min)
- Phase 4: Install script (15 min)
- Phase 5: Documentation (30 min)
- Phase 6: Testing (2 hours)

**Suggested Schedule:**
- **Session 1 (2 hours):** Phases 1-3
- **Session 2 (1 hour):** Phase 4-5
- **Session 3 (2 hours):** Phase 6 + fixes

---

## Recommendation

**Proceed: YES**

**Why:**
1. Current architecture is non-professional
2. Refactoring is straightforward (well-understood pattern)
3. Benefits outweigh risks
4. Good time to do it (before 1.0 release)
5. Test suite provides safety net

**When:**
- After we complete configuration system refactoring
- Before adding major new features
- When we have 4-6 hours for focused work

**Priority:** High (architectural improvement)

---

## Next Steps

If you approve this plan:

1. **I will create:**
   - Updated test script (fixed to handle banner)
   - Baseline test results file
   - Step-by-step implementation checklist

2. **You will review:**
   - Confirm approach
   - Identify any concerns
   - Set timeline

3. **We will execute:**
   - Phase-by-phase implementation
   - Test after each phase
   - Document any issues

**Ready to proceed?**
