# Namespace Validation: Clojure vs Clorus

## Critical Difference: ❌ Clorus Does NOT Validate Namespace Matching!

### Clojure Behavior (Strict) ✅

```clojure
;; File: src/com/example/core.clj
(ns com.example.wrong)  ; ❌ ERROR!
;; => Error: Namespace 'com.example.wrong' does not match file path 'com/example/core'

;; File: src/com/example/core.clj
(ns com.example.core)   ; ✅ CORRECT - must match file path
```

**Clojure validates at load time:**
1. File must be at `src/com/example/core.clj`
2. Must contain `(ns com.example.core)`
3. Any mismatch = immediate error with clear message

### Clorus Behavior (Permissive) ⚠️

```clojure
;; File: src/helper/utils.clrs
(ns helper.utils)        ; ✅ Works - correct
(defn double [x] (* x 2))

;; File: src/helper/utils.clrs
(ns wrong.namespace)     ; ⚠️ NO ERROR! But causes confusing failures later
(defn double [x] (* x 2))
```

**Clorus behavior:**
1. ❌ **No validation** that namespace matches file path
2. ⚠️ File can declare ANY namespace
3. 💥 Causes confusing compile errors later

## The Problem Demonstrated

### Test Case 1: Entry File (No Validation)

```clojure
;; File: src/main.clrs
(ns this.makes.no.sense)  ; ✅ Compiles fine!

(defn -main [args]
  42)
```

**Result:** ✅ Works perfectly! Entry files have no validation.

```bash
$ clorus run
=> 42
```

### Test Case 2: Required Module (Partial Validation)

**Setup:**
```
src/
├── main.clrs
└── helper/
    └── utils.clrs
```

**Correct usage:**
```clojure
;; src/helper/utils.clrs
(ns helper.utils)  ; ✅ Matches file path
(defn double [x] (* x 2))

;; src/main.clrs
(ns main
  (:require [helper.utils :as util]))  ; ✅ File found at src/helper/utils.clrs

(util/double 5)  ; ✅ Works! Returns 10
```

**Incorrect usage (namespace mismatch):**
```clojure
;; src/helper/utils.clrs
(ns wrong.namespace)  ; ⚠️ WRONG! Doesn't match file path
(defn double [x] (* x 2))

;; src/main.clrs
(ns main
  (:require [helper.utils :as util]))  ; File loads from src/helper/utils.clrs

(util/double 5)  ; ❌ Compile error: Unknown function: util/double
```

**Error message:**
```
Error: Compile error: Unknown function: util/double.
Did you (use rust.util)?
```

**Why it fails:**
1. ✅ Clorus finds file at `src/helper/utils.clrs` (based on require path)
2. ✅ Clorus loads and compiles the file
3. ⚠️ Functions are registered in `wrong.namespace` (from ns declaration)
4. ❌ Looking for functions in `helper.utils` (from require)
5. 💥 Functions not found → confusing error

**Can you access via the actual namespace?**
```clojure
;; Try to require by the actual namespace declared in file
(ns main
  (:require [wrong.namespace :as wrong]))

(wrong/double 5)  ; ❌ ERROR!
```

**Error:**
```
Error: Module file not found: /src/wrong/namespace.clrs
```

**Why:** Clorus looks for file at `src/wrong/namespace.clrs`, but the file is actually at `src/helper/utils.clrs`!

## Summary of Clorus Behavior

| Aspect | Clojure | Clorus | Impact |
|--------|---------|--------|--------|
| Entry file ns validation | ✅ Yes | ❌ No | Minor - entry file can have any ns |
| Module file ns validation | ✅ Yes | ❌ No | **Critical** - causes confusing errors |
| File path lookup | ✅ Based on ns name | ✅ Based on require path | Works the same |
| Function registration | ✅ In declared ns | ✅ In declared ns | Works the same |
| Error messages | ✅ Clear validation error | ❌ Confusing "function not found" | **Poor UX** |

## The Root Cause

**Location:** `commands.rs:410-436`

```rust
// Convert module name to file path: math -> src/math.clrs
let module_path = module_name.replace('.', "/");
let module_file = project_root.join("src").join(format!("{}.clrs", module_path));

if !module_file.exists() {
    return Err(format!("Module file not found: {}", module_file.display()));
}

// Load the file - NO validation of namespace!
let source = fs::read_to_string(&module_file)?;
let exprs = clorus::parse_and_expand(&source)?;

// Set namespace context from the file's (ns ...) declaration
for expr in &exprs {
    if let Expr::Ns { name, requires, rust_imports } = expr {
        let mut ns_ctx = NamespaceContext::default_namespace();
        ns_ctx.current = name.clone();  // ⚠️ Uses whatever name is declared!
        // ...
        codegen.set_namespace(ns_ctx);
    }
}
```

**Missing validation:**
```rust
// ❌ No check that:
//    name == module_name
// or
//    name == "helper.utils" when file is at src/helper/utils.clrs
```

## The Fix Needed

### Add Validation in `load_and_compile_modules()`

```rust
// After line 434 in commands.rs
if let Expr::Ns { name, requires, rust_imports } = expr {
    // ✅ ADD THIS VALIDATION
    if name != module_name {
        return Err(format!(
            "Namespace mismatch in {}:\n  File path expects: (ns {})\n  Found: (ns {})\n  \
             In Clorus, namespace must match file path.\n  \
             Move file to: src/{}.clrs\n  \
             Or change namespace to: (ns {})",
            module_file.display(),
            module_name,
            name,
            name.replace('.', "/"),
            module_name
        ));
    }

    let mut ns_ctx = NamespaceContext::default_namespace();
    ns_ctx.current = name.clone();
    // ...
}
```

### What This Fixes

**Before (current):**
```clojure
;; src/helper/utils.clrs
(ns wrong.namespace)  ; ⚠️ Silently accepted
```

**After (with validation):**
```
Error: Namespace mismatch in src/helper/utils.clrs:
  File path expects: (ns helper.utils)
  Found: (ns wrong.namespace)

  In Clorus, namespace must match file path.

  Either:
  - Change namespace to: (ns helper.utils)
  - Or move file to: src/wrong/namespace.clrs
```

## Impact Analysis

### Without Validation (Current State)

**Pros:**
- More flexible
- Entry files can have any namespace
- Less restrictive

**Cons:**
- ❌ Confusing errors when namespace mismatches
- ❌ Poor developer experience
- ❌ Doesn't follow Clojure convention
- ❌ Hard to debug
- ❌ Encourages bad practices

### With Validation (Recommended)

**Pros:**
- ✅ Clear error messages upfront
- ✅ Matches Clojure behavior (Clojure developer expectation)
- ✅ Enforces best practices
- ✅ Easier to debug
- ✅ Better IDE tooling support

**Cons:**
- Slightly less flexible for entry files
- Need to ensure examples follow convention

## Recommendation

### Implement Strict Validation (Like Clojure)

**For all files (including entry file):**
```rust
fn validate_namespace(file_path: &Path, declared_ns: &str, project_root: &Path) -> Result<(), String> {
    // Calculate expected namespace from file path
    let relative = file_path.strip_prefix(project_root.join("src"))
        .map_err(|_| "File must be in src/ directory")?;

    let expected_ns = relative
        .with_extension("")
        .to_string_lossy()
        .replace('/', ".")
        .replace('\\', ".");

    if declared_ns != expected_ns {
        return Err(format!(
            "Namespace mismatch in {}:\n  \
             File path expects: (ns {})\n  \
             Found: (ns {})\n\n  \
             Fix by either:\n  \
             1. Change namespace: (ns {})\n  \
             2. Move file to: src/{}.clrs",
            file_path.display(),
            expected_ns,
            declared_ns,
            expected_ns,
            declared_ns.replace('.', "/")
        ));
    }

    Ok(())
}
```

**Apply to:**
1. ✅ Entry file (src/main.clrs → must have `(ns main)`)
2. ✅ All required modules
3. ✅ All compiled files

## Migration Path

### Phase 1: Add Warning (Non-Breaking)

```
Warning: Namespace 'wrong.namespace' doesn't match file path 'src/helper/utils.clrs'
  Expected: (ns helper.utils)
  This will become an error in Clorus v0.2.0
```

### Phase 2: Make it an Error (Breaking Change)

- Update all examples to follow convention
- Document the requirement
- Enable strict validation

## Comparison Table

| Feature | Clojure | Clorus (Current) | Clorus (Proposed) |
|---------|---------|------------------|-------------------|
| Entry file validation | ✅ Strict | ❌ None | ✅ Strict |
| Module validation | ✅ Strict | ❌ None | ✅ Strict |
| Error on mismatch | ✅ Clear error | ❌ Confusing error | ✅ Clear error |
| Follows convention | ✅ Yes | ⚠️ Optional | ✅ Required |
| Developer experience | ✅ Excellent | ❌ Poor | ✅ Excellent |

## Conclusion

**Current State:**
- ❌ Clorus does NOT validate namespace matches file path
- ❌ Causes confusing errors at compile time
- ❌ Poor developer experience
- ❌ Doesn't match Clojure behavior

**Recommendation:**
- ✅ Add strict validation like Clojure
- ✅ Clear error messages
- ✅ Better developer experience
- ✅ Matches Clojure convention

**Next Steps:**
1. Implement validation in `load_and_compile_modules()`
2. Add validation for entry file in `run()` function
3. Update all examples to follow convention
4. Document the requirement clearly
