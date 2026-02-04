# Clorus REPL Integration Test Plan

## Purpose
Ensure REPL functionality remains intact during architectural refactoring from standalone binary to integrated subcommand.

## Test Categories

### 1. Smoke Tests (Must Pass Before/After)

#### Basic REPL Functionality
```bash
# Test 1: REPL starts and quits
echo ":quit" | clorus repl

# Test 2: Simple arithmetic
echo "(+ 1 2)\n:quit" | clorus repl | grep "3"

# Test 3: Function definition
echo "(defn inc [x] (+ x 1))\n(inc 5)\n:quit" | clorus repl | grep "6"

# Test 4: Stdlib loading
echo "(map inc [1 2 3])\n:quit" | clorus repl | grep "\[2 3 4\]"
```

#### Commands
```bash
# Test 5: Help command
echo ":help\n:quit" | clorus repl | grep -i "commands"

# Test 6: Examples command
echo ":examples\n:quit" | clorus repl | grep -i "example"
```

#### Error Handling
```bash
# Test 7: Syntax error
echo "(+ 1\n:quit" | clorus repl 2>&1 | grep -i "error"

# Test 8: Undefined function
echo "(undefined-fn)\n:quit" | clorus repl 2>&1 | grep -i "undefined"
```

### 2. Integration Tests

#### Project Context
```bash
# Test 9: REPL in project directory
cd /tmp/test-project
echo "(ns my.app)\n:quit" | clorus repl | grep "my.app"

# Test 10: Load project files
# (requires setting up test project first)
```

#### Stdlib Loading
```bash
# Test 11: All 76 stdlib functions load
echo ":quit" | clorus repl 2>&1 | grep "Loaded clorus.core (76 functions)"

# Test 12: Stdlib functions work
echo "(filter even? [1 2 3 4])\n:quit" | clorus repl | grep "\[2 4\]"
echo "(take 2 [1 2 3 4])\n:quit" | clorus repl | grep "\[1 2\]"
echo "(drop 2 [1 2 3 4])\n:quit" | clorus repl | grep "\[3 4\]"
```

#### FFI/Library Loading
```bash
# Test 13: rust.fs module available
echo "(require '[rust.fs :as fs])\n:quit" | clorus repl 2>&1 | grep -v "Error"

# Test 14: File operations work
echo "(require '[rust.fs :as fs])\n(fs/exists? \"/tmp\")\n:quit" | clorus repl | grep "true"
```

### 3. Regression Tests (Current Known Issues)

#### Comparison Operators
```bash
# Test 15: Booleans returned (not numbers)
echo "(< 5 10)\n:quit" | clorus repl | grep "true"
echo "(> 5 10)\n:quit" | clorus repl | grep "false"
echo "(= 5 5)\n:quit" | clorus repl | grep "true"
```

#### Collection Operations
```bash
# Test 16: conj works on all types
echo "(conj [1 2] 3)\n:quit" | clorus repl | grep "\[1 2 3\]"
echo "(conj #{1 2} 3)\n:quit" | clorus repl | grep "3"
```

#### Type Handling
```bash
# Test 17: Long and Double both work
echo "(+ 5 3)\n:quit" | clorus repl | grep "8"
echo "(+ 5.5 3.2)\n:quit" | clorus repl | grep "8.7"
echo "(+ 5 3.5)\n:quit" | clorus repl | grep "8.5"
```

### 4. Performance Tests

```bash
# Test 18: Startup time
time (echo ":quit" | clorus repl) < 2s

# Test 19: Large collection
echo "(count (range 1000))\n:quit" | clorus repl | grep "1000"
```

### 5. User Experience Tests

#### Autocomplete
```bash
# Test 20: Tab completion available
# (manual test - rustyline integration)
```

#### History
```bash
# Test 21: History file created
echo "(+ 1 2)\n:quit" | clorus repl
test -f ~/.clorus_history
```

#### Error Messages
```bash
# Test 22: Helpful error messages
echo "(defn broken [x] (y))\n:quit" | clorus repl 2>&1 | grep -i "undefined"
```

## Test Execution Plan

### Phase 1: Baseline (Before Refactoring)
1. Run all smoke tests with current `repl` binary
2. Document all passing/failing tests
3. Create baseline metrics file

### Phase 2: During Refactoring
1. Run smoke tests after each major change
2. Fix any regressions immediately
3. Document any behavior changes

### Phase 3: Post-Refactoring
1. Run full test suite
2. Compare against baseline
3. Document any differences
4. Update tests if behavior intentionally changed

## Automated Test Script

```bash
#!/bin/bash
# test-repl.sh - Automated REPL test suite

REPL_CMD="${1:-repl}"  # Allow testing either 'repl' or 'clorus repl'
PASS=0
FAIL=0
TOTAL=0

run_test() {
    local name="$1"
    local command="$2"
    local expected="$3"

    TOTAL=$((TOTAL + 1))
    echo -n "Test $TOTAL: $name ... "

    if eval "$command" | grep -q "$expected"; then
        echo "✓ PASS"
        PASS=$((PASS + 1))
    else
        echo "✗ FAIL"
        FAIL=$((FAIL + 1))
        echo "  Command: $command"
        echo "  Expected: $expected"
    fi
}

echo "================================"
echo "Clorus REPL Test Suite"
echo "Testing: $REPL_CMD"
echo "================================"
echo

# Smoke tests
run_test "REPL starts" "echo ':quit' | $REPL_CMD" "Goodbye"
run_test "Simple arithmetic" "echo '(+ 1 2)' | $REPL_CMD" "3"
run_test "Stdlib loading" "echo ':quit' | $REPL_CMD 2>&1" "Loaded clorus.core"
run_test "Map function" "echo '(map inc [1 2 3])' | $REPL_CMD" "\[2 3 4\]"
run_test "Filter function" "echo '(filter even? [1 2 3 4])' | $REPL_CMD" "\[2 4\]"
run_test "Comparison operators" "echo '(< 5 10)' | $REPL_CMD" "true"

echo
echo "================================"
echo "Results: $PASS/$TOTAL passed, $FAIL failed"
echo "================================"

exit $FAIL
```

## Success Criteria

✅ All baseline tests must pass after refactoring
✅ No performance regression (startup time)
✅ All 76 stdlib functions still load
✅ Error messages remain helpful
✅ Autocomplete still works
✅ History file still created

## Rollback Plan

If more than 3 tests fail after refactoring:
1. Revert changes
2. Investigate failures
3. Fix issues in development
4. Re-test before merging
