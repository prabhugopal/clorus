# Clorus Bug Fix & Feature Implementation Plan

## Priority Order (Based on CORAL Team Reports)

### P0 - CRITICAL (Fix Immediately)
1. ✅ **Loop/Recur Bug** - ALREADY FIXED
2. ❌ **String Use-After-Free** - BLOCKS PRODUCTION
   - Location: `crates/clorus-runtime/src/string.rs:87-92`
   - Impact: Crashes on any string operations
   - Fix: Box string before creating Value

### P1 - HIGH PRIORITY
3. ⚠️ **Missing Stdlib Functions** - PARTIALLY DONE
   - ✅ Numeric: floor, ceil, round, inc, dec, max, min
   - ✅ Boolean: not
   - ✅ Collections: conj
   - ❌ String: substring, char-at, index-of, split, join, trim
   - ❌ Collections: take, drop (exist but may need improvement)

4. ❌ **Compiler Hang Detection**
   - Add 30-60s timeout
   - Add progress indicators
   - Detect infinite compilation loops

5. ❌ **Stdlib Transducers Issue**
   - wrap_in_function variable scoping bug
   - Nested closures fail in stdlib loading
   - Works in regular files, fails in stdlib

### P2 - NICE TO HAVE
6. Better error messages with line numbers
7. String interpolation syntax
8. REPL improvements (history, tab completion)

### NEW FEATURES
9. ❌ **Library Packaging System**
   - Create .clip (Clorus Library Package) format
   - Similar to .jar (Java), .crate (Rust), .gem (Ruby)
   - Allow distribution and linking of libraries
   - Package format: compiled .o files + metadata

## Implementation Plan

### Phase 1: Fix P0 Bug (30 min)
- Fix string use-after-free
- Add test case
- Verify CORAL Gallery works without workarounds

### Phase 2: Create Library Packaging (2 hours)
- Design .clip format
- Implement packaging tool
- Implement linking system
- Documentation

### Phase 3: Fix P1 Issues (2 hours)
- Add compiler timeout
- Fix stdlib transducers
- Add missing string functions

### Phase 4: Polish (1 hour)
- Better error messages
- Progress indicators
- Documentation updates

## Success Criteria

✅ All P0 bugs fixed
✅ Library packaging system working
✅ CORAL Gallery builds without workarounds
✅ Transducers functional in stdlib
✅ No compiler hangs
