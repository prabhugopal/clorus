# try/catch/finally Implementation Status

## ✅ COMPLETED (Partial Implementation)

### 1. AST Support ✅
- Added `Try` variant with body, catch clauses, and optional finally block
- Added `Throw` variant for throwing exceptions
- Added `CatchClause` struct for exception type, binding, and handler

### 2. Parser Support ✅
- Full parsing of `(try body (catch Type e handler) (finally cleanup))`
- Support for multiple catch clauses
- Support for optional finally block
- Support for `(throw expr)`
- **52 tests passing** (up from 48)

### 3. Macro Expansion ✅
- try/catch/finally expressions properly expanded through macro system
- All nested expressions in body, handlers, and finally blocks are expanded

### 4. Codegen (Placeholder) ⚠️
- Basic code generation that:
  - Executes the body
  - Executes finally block (if present)
  - Returns result
- **Does NOT actually catch exceptions** (see below)
- **Does NOT actually throw exceptions** (see below)

## ❌ NOT IMPLEMENTED (Complex)

### Why Exception Handling is Hard in LLVM

LLVM exception handling requires:

1. **invoke instruction** - Instead of regular `call`, must use `invoke` which has two destinations:
   - Normal destination (if call succeeds)
   - Unwind destination (if exception is thrown)

2. **landingpad instruction** - Landing pad blocks that catch exceptions:
   ```llvm
   lpad:
     %1 = landingpad { i8*, i32 }
           catch i8* @exception_type_info
           cleanup
   ```

3. **Personality function** - Required for stack unwinding:
   - Must declare `__gxx_personality_v0` or similar
   - Links to C++ runtime for exception handling

4. **Exception type info** - RTTI structures for exception types:
   - `@exception_type_info = external global i8`
   - Must match C++ typeinfo structures

5. **Throwing exceptions** - Requires C++ runtime calls:
   - `__cxa_allocate_exception(size)`
   - `__cxa_throw(exception, type_info, destructor)`

6. **Stack unwinding** - Automatic cleanup of stack frames during throw

### Example of what's needed:

```llvm
define i32 @func_with_exception() personality i8* bitcast (i32 (...)* @__gxx_personality_v0 to i8*) {
entry:
  ; Use invoke instead of call
  invoke void @risky_function()
          to label %normal unwind label %lpad

normal:
  ret i32 0

lpad:
  %1 = landingpad { i8*, i32 }
          catch i8* @exception_type_info
  ; Extract exception pointer
  %2 = extractvalue { i8*, i32 } %1, 0
  ; Check if it matches our type
  %3 = call i8* @__cxa_begin_catch(i8* %2)
  ; Handle exception
  call void @__cxa_end_catch()
  ret i32 1
}
```

## Current Status Summary

| Feature | Status | Notes |
|---------|--------|-------|
| AST | ✅ Complete | Try/Throw variants added |
| Parser | ✅ Complete | Full syntax support |
| Macro expansion | ✅ Complete | Properly handles nested expressions |
| Codegen - finally blocks | ✅ Works | Finally blocks execute |
| Codegen - catch blocks | ❌ Placeholder | Doesn't catch anything |
| Codegen - throw | ❌ Placeholder | Doesn't throw anything |
| LLVM invoke/landingpad | ❌ Not implemented | Complex, needs 4-6 hours |
| Exception types | ❌ Not implemented | Need RTTI structures |
| Stack unwinding | ❌ Not implemented | Need personality function |

## What Works Now

```clojure
;; Syntax parses correctly
(try
  (risky-operation)
  (catch Error e (handle-error e))
  (finally (cleanup)))

;; Finally blocks DO execute
(try
  (do-something)
  (finally
    (println "This runs")))  ;; ✅ This will actually run

;; But exceptions DON'T actually get caught
(try
  (/ 10 0)  ;; Would crash, not caught
  (catch Error e
    (println "This won't run")))  ;; ❌ Handler won't be called
```

## To Complete Implementation

**Estimated time: 4-6 hours**

### Phase 1: LLVM Setup (1-2 hours)
1. Add personality function declaration
2. Mark functions that can throw with personality
3. Declare C++ runtime functions (__cxa_*)

### Phase 2: Invoke/Landingpad (2-3 hours)
1. Replace regular calls with invoke
2. Create landing pad blocks for each try
3. Wire up unwind destinations
4. Implement selector logic for multiple catch clauses

### Phase 3: Exception Throwing (1 hour)
1. Implement __cxa_allocate_exception calls
2. Implement __cxa_throw calls
3. Handle exception value wrapping

### Phase 4: Testing (1 hour)
1. Test simple throw/catch
2. Test multiple catch clauses
3. Test finally with exceptions
4. Test re-throwing
5. Test uncaught exceptions

## Alternative: Simpler Error Handling

If full C++-style exceptions are too complex, we could implement a simpler error handling model:

1. **Result types** - Functions return (value, error) tuples
2. **Error propagation** - Manual checking and propagation
3. **Panics** - Simple abort on fatal errors

This would be easier to implement (~2 hours) but less powerful than true exceptions.

## Recommendation

Given the complexity, I recommend:

1. **Keep current placeholder implementation** - It allows code to compile
2. **Move on to other features** - Destructuring, gensym, etc. are more critical
3. **Come back to exceptions later** - When we need production-grade error handling

The syntax and AST are ready, so when we do implement full exceptions, we can just replace the codegen without changing the language.
