# Number Type Implementation in Clorus

## Current State: f64-Only Numbers

**Status:** ✅ Working but Limited
**Type:** Single precision floating-point (64-bit IEEE 754)
**Similar to:** JavaScript numbers, Lua numbers, early Clojure

### Overview

Clorus currently represents **all numbers** as 64-bit floating point values (`f64`). This includes:
- Integers like `42`, `0`, `-17`
- Decimals like `3.14`, `-0.5`, `2.718`
- Special values: `NaN`, `Infinity`, `-Infinity`

### Implementation Details

#### Value Representation

Location: `crates/clorus-runtime/src/value.rs`

```rust
pub enum ValueTag {
    Number = 0,      // f64 stored inline in Value
    List = 1,
    Vector = 2,
    // ... other types
}

pub struct Value {
    // Numbers are stored directly in the value (not heap-allocated)
    // Uses NaN-boxing technique for efficient representation
}
```

**Key characteristics:**
- Numbers are **immediate values** (not pointers)
- Stored inline in the 64-bit `Value` struct
- Uses NaN-boxing for type tagging
- Zero heap allocation for numeric values

#### FFI Interface

Location: `crates/clorus-runtime/src/value.rs:447-456`

```rust
/// Create a number value from f64
#[no_mangle]
pub extern "C" fn clorus_value_number(n: f64) -> *mut Value {
    Value::number(n)
}

/// Extract f64 from number value
#[no_mangle]
pub extern "C" fn clorus_value_as_number(val: *mut Value) -> f64 {
    unsafe { (*val).as_number() }
}
```

#### Codegen

Location: `crates/clorus-codegen/src/codegen.rs`

Numbers are compiled directly to LLVM `f64` constants:

```rust
Expr::Number(n) => {
    let num_val = self.context.f64_type().const_float(*n);
    let create_fn = self.module.get_function("clorus_value_number")?;
    let result = self.builder.build_call(
        create_fn,
        &[num_val.into()],
        "num"
    )?;
    Ok(result.try_as_basic_value().left()?.into_pointer_value())
}
```

**Arithmetic operations** (`+`, `-`, `*`, `/`, etc.) work directly on f64:

```rust
"+" => {
    let left_num = self.builder.build_call(
        as_number_fn,
        &[left.into()],
        "left_num"
    )?.try_as_basic_value().left()?.into_float_value();

    let right_num = self.builder.build_call(
        as_number_fn,
        &[right.into()],
        "right_num"
    )?.try_as_basic_value().left()?.into_float_value();

    let sum = self.builder.build_float_add(left_num, right_num, "sum")?;
    // ... convert back to Value
}
```

---

## Limitations of f64-Only

### 1. Integer Precision Loss

**Problem:** f64 can only precisely represent integers up to 2^53 (9,007,199,254,740,992)

```clojure
;; Current behavior (f64)
(= (+ 9007199254740992 1) 9007199254740992)  ; => true (WRONG!)

;; Expected with proper integers
(= (+ 9007199254740992 1) 9007199254740993)  ; => true (CORRECT)
```

### 2. No Integer Division Semantics

**Problem:** Division always returns float, even for exact divisions

```clojure
;; Current behavior
(/ 10 2)    ; => 5.0 (not 5)
(/ 10 3)    ; => 3.3333333333333335

;; Expected with proper integers
(/ 10 2)    ; => 5 (integer)
(quot 10 3) ; => 3 (integer quotient)
(/ 10 3)    ; => 10/3 (rational) or 3.3333... (float)
```

### 3. No Rationals

**Problem:** Cannot represent exact fractions

```clojure
;; Current behavior
(/ 1 3)     ; => 0.3333333333333333 (loses precision)

;; Expected with rationals
(/ 1 3)     ; => 1/3 (exact rational)
(* 3 (/ 1 3))  ; => 1 (exact)
```

### 4. Type Predicates Missing

```clojure
;; Not available (no distinction)
(integer? 5)      ; N/A
(float? 3.14)     ; N/A
(rational? 1/3)   ; N/A

;; Only have
(number? 5)       ; => true (everything is a number)
```

### 5. Bit Operations Unsafe

```clojure
;; Unsafe for large numbers
(bit-and 9007199254740993 1)  ; May give wrong results
```

---

## Clojure's Number Tower (Reference)

Clojure implements a sophisticated number hierarchy:

```
Number (interface)
  ├─ Integer
  │   ├─ Long (primary integer type, 64-bit)
  │   ├─ BigInt (arbitrary precision)
  │   ├─ Short, Byte, Int (Java interop)
  │   └─ BigInteger (Java arbitrary precision)
  │
  ├─ Floating Point
  │   ├─ Double (64-bit IEEE 754, primary float type)
  │   └─ Float (32-bit, Java interop)
  │
  ├─ Ratio (exact fractions, e.g., 1/3)
  │
  └─ BigDecimal (arbitrary precision decimal)
```

**Key features:**
- Automatic promotion (long → bigint when overflow)
- Contagion (integer + float = float)
- Exact arithmetic by default
- Performance: primitives for common cases

---

## Proposed Implementation: Full Number Tower

### Phase 1: Integer/Float Distinction (Priority: HIGH)

**Goal:** Support both `i64` integers and `f64` floats

#### 1.1 Update ValueTag

```rust
pub enum ValueTag {
    Integer = 0,     // NEW: i64 immediate value
    Float = 1,       // Renamed from Number
    List = 2,
    Vector = 3,
    // ...
}
```

#### 1.2 Value Storage Strategy

**Option A: NaN-Boxing (Recommended)**

Use IEEE 754 NaN space to encode integers:

```rust
// Quiet NaN range: 0x7FF8_0000_0000_0000 to 0x7FFF_FFFF_FFFF_FFFF
// Use upper bits for type tags, lower 48 bits for data

// Integer: 0x7FF8_0000_0000_0000 | (i48 value)
// Float:   Just store f64 normally
// Pointer: 0x7FF9_0000_0000_0000 | (48-bit pointer)
```

**Advantages:**
- Zero overhead for common cases
- Integers fit in 48 bits (±140 trillion)
- Fast type checking
- Cache-friendly

**Option B: Tagged Union (Simpler)**

```rust
pub struct Value {
    tag: ValueTag,
    data: u64,  // Reinterpreted based on tag
}
```

**Advantages:**
- Clearer code
- Easier to debug
- Supports full 64-bit integers

#### 1.3 FFI Functions

```rust
// New integer functions
#[no_mangle]
pub extern "C" fn clorus_value_integer(n: i64) -> *mut Value;

#[no_mangle]
pub extern "C" fn clorus_value_as_integer(val: *mut Value) -> i64;

#[no_mangle]
pub extern "C" fn clorus_is_integer(val: *mut Value) -> bool;

#[no_mangle]
pub extern "C" fn clorus_is_float(val: *mut Value) -> bool;

// Updated arithmetic (handles both types)
#[no_mangle]
pub extern "C" fn clorus_add(a: *mut Value, b: *mut Value) -> *mut Value {
    // If both integers -> integer result
    // If either is float -> float result
    // If overflow -> promote to bigint (Phase 2)
}
```

#### 1.4 Codegen Updates

```rust
// Parse literals correctly
"42"    -> Integer(42)
"42.0"  -> Float(42.0)
"42e3"  -> Float(42000.0)

// Arithmetic type rules
fn compile_add(&mut self, left: Expr, right: Expr) {
    // Generate code that:
    // 1. Checks if both are integers
    // 2. If yes: integer add with overflow check
    // 3. If no: promote to float and float add
}
```

#### 1.5 Type Predicates

```clojure
;; stdlib/core.clr additions
(defn integer? [x]
  (clorus.runtime/is-integer x))

(defn float? [x]
  (clorus.runtime/is-float x))

(defn number? [x]
  (or (integer? x) (float? x)))
```

### Phase 2: Arbitrary Precision (BigInt)

**Goal:** Handle integer overflow gracefully

#### 2.1 Add BigInt Type

```rust
pub enum ValueTag {
    Integer = 0,     // i64 immediate
    Float = 1,       // f64 immediate
    BigInt = 2,      // Heap-allocated arbitrary precision
    // ...
}

pub struct ClorusBigInt {
    // Use GMP, num-bigint, or custom implementation
    data: Vec<u64>,  // Limbs
    sign: bool,
}
```

#### 2.2 Automatic Promotion

```rust
pub extern "C" fn clorus_add(a: *mut Value, b: *mut Value) -> *mut Value {
    if both_are_i64(a, b) {
        let result = a_val.checked_add(b_val);
        match result {
            Some(sum) => Value::integer(sum),
            None => {
                // Overflow! Promote to BigInt
                Value::bigint_from_overflow(a_val, b_val)
            }
        }
    }
    // ... handle other cases
}
```

### Phase 3: Rationals

**Goal:** Exact fractions

#### 3.1 Add Ratio Type

```rust
pub enum ValueTag {
    Integer = 0,
    Float = 1,
    BigInt = 2,
    Ratio = 3,       // NEW: numerator/denominator
    // ...
}

pub struct ClorusRatio {
    numerator: *mut Value,    // Integer or BigInt
    denominator: *mut Value,  // Integer or BigInt (never zero)
    // Always stored in lowest terms
}
```

#### 3.2 Division Semantics

```clojure
(/ 10 3)      ; => 10/3 (ratio)
(/ 10 2)      ; => 5 (integer, reduces to whole number)
(/ 10 3.0)    ; => 3.333... (float, because 3.0 is float)

;; Force types
(quot 10 3)   ; => 3 (integer quotient)
(rem 10 3)    ; => 1 (remainder)
(mod 10 3)    ; => 1 (modulo)
(float (/ 10 3))  ; => 3.333... (explicit conversion)
```

### Phase 4: BigDecimal (Optional)

For financial calculations requiring exact decimal arithmetic.

---

## Implementation Roadmap

### Milestone 1: Integer/Float Split (2-3 weeks)

**Priority:** HIGH
**Impact:** Fixes precision issues, enables proper integer semantics

1. **Week 1: Runtime**
   - [ ] Implement NaN-boxing or tagged union in `value.rs`
   - [ ] Add integer FFI functions
   - [ ] Update arithmetic operations
   - [ ] Write unit tests

2. **Week 2: Codegen**
   - [ ] Update number literal parsing
   - [ ] Update arithmetic codegen
   - [ ] Add type checking/promotion
   - [ ] Integration tests

3. **Week 3: Stdlib & Polish**
   - [ ] Add type predicates (`integer?`, `float?`)
   - [ ] Update all numeric functions
   - [ ] Performance benchmarks
   - [ ] Documentation

**Success Criteria:**
```clojure
✅ (integer? 42)        ; => true
✅ (float? 42.0)        ; => true
✅ (= 42 42.0)          ; => true (numeric equality)
✅ (identical? 42 42.0) ; => false (type-aware)
✅ (+ 1 2)              ; => 3 (integer)
✅ (+ 1 2.0)            ; => 3.0 (float)
✅ (/ 10 2)             ; => 5 (integer division when exact)
```

### Milestone 2: BigInt Support (2-3 weeks)

**Priority:** MEDIUM
**Impact:** Enables arbitrary precision

1. **Dependencies:** Add `num-bigint` crate
2. **Runtime:** Implement `ClorusBigInt` type
3. **Arithmetic:** Add overflow detection & promotion
4. **Testing:** Verify large number operations

**Success Criteria:**
```clojure
✅ (+ 9007199254740992 1)  ; => 9007199254740993 (correct!)
✅ (* 12345678901234567890 2)  ; Works correctly
✅ Transparent promotion (no user intervention needed)
```

### Milestone 3: Rationals (3-4 weeks)

**Priority:** MEDIUM
**Impact:** Exact fractions, proper division semantics

**Success Criteria:**
```clojure
✅ (/ 1 3)          ; => 1/3
✅ (* 3 (/ 1 3))    ; => 1 (exact)
✅ (rational? 1/3)  ; => true
✅ (float 1/3)      ; => 0.333...
```

### Milestone 4: Performance Optimization (1-2 weeks)

**Priority:** MEDIUM
**Impact:** Fast numeric operations

- [ ] Inline integer operations where possible
- [ ] Fast path for common cases
- [ ] SIMD for vector operations
- [ ] Benchmark against Clojure

---

## Technical Challenges & Solutions

### Challenge 1: Type Contagion Rules

**Problem:** When mixing types, which type wins?

**Solution:** Follow Clojure's rules:
1. Integer + Integer = Integer
2. Integer + Float = Float
3. Integer + BigInt = BigInt
4. Integer + Ratio = Ratio
5. Float + Ratio = Float
6. Ratio + Ratio = Ratio (simplified)

**Implementation:**
```rust
fn numeric_promote(a: ValueTag, b: ValueTag) -> ValueTag {
    match (a, b) {
        (Integer, Integer) => Integer,
        (Integer, Float) | (Float, Integer) => Float,
        (_, BigInt) | (BigInt, _) => BigInt,
        // ... etc
    }
}
```

### Challenge 2: Parser Ambiguity

**Problem:** Is `42` an integer or float?

**Solution:** Parse based on literal syntax:
```rust
fn parse_number(s: &str) -> Expr {
    if s.contains('.') || s.contains('e') || s.contains('E') {
        Expr::Float(s.parse::<f64>()?)
    } else if s.contains('/') {
        // Parse ratio: "22/7"
        Expr::Ratio(parse_ratio(s)?)
    } else {
        Expr::Integer(s.parse::<i64>()?)
    }
}
```

### Challenge 3: Equality Semantics

**Problem:** Should `42 == 42.0`?

**Solution:** Two types of equality:
```clojure
(= 42 42.0)         ; => true (numeric equality, Clojure style)
(identical? 42 42.0) ; => false (type-aware equality)
```

**Implementation:**
```rust
fn numeric_equal(a: Value, b: Value) -> bool {
    // Convert both to common type and compare
    let (a_promoted, b_promoted) = promote_for_comparison(a, b);
    compare_same_type(a_promoted, b_promoted)
}
```

### Challenge 4: Performance

**Problem:** Type checking overhead on every operation

**Solution:**
1. **NaN-boxing** for zero-overhead type tags
2. **Inline fast paths** for integer+integer
3. **JIT optimizations** in LLVM
4. **Specialize common cases** at compile time

---

## Compatibility Considerations

### Breaking Changes

**Before (current):**
```clojure
(type 42)   ; => "Number"
(/ 10 3)    ; => 3.3333...
```

**After (Phase 1):**
```clojure
(type 42)   ; => "Integer"
(type 42.0) ; => "Float"
(/ 10 3)    ; => 3.3333... (still float until Phase 3)
```

### Migration Path

1. **Deprecation warnings** for code relying on float semantics
2. **Compatibility mode** flag for old behavior
3. **Gradual rollout** (integer/float first, rationals later)

---

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_integer_creation() {
    let v = Value::integer(42);
    assert_eq!(v.tag(), ValueTag::Integer);
    assert_eq!(v.as_integer(), 42);
}

#[test]
fn test_integer_overflow() {
    let a = Value::integer(i64::MAX);
    let b = Value::integer(1);
    let result = clorus_add(a, b);
    assert_eq!(result.tag(), ValueTag::BigInt);
}

#[test]
fn test_type_contagion() {
    let i = Value::integer(5);
    let f = Value::float(2.0);
    let result = clorus_add(i, f);
    assert_eq!(result.tag(), ValueTag::Float);
    assert_eq!(result.as_float(), 7.0);
}
```

### Integration Tests

```clojure
;; tests/test-numbers.clr

(deftest test-integer-arithmetic
  (testing "Integer addition"
    (is (= (+ 2 3) 5))
    (is (integer? (+ 2 3)))
    (is (not (float? (+ 2 3)))))

  (testing "Mixed arithmetic"
    (is (= (+ 2 3.0) 5.0))
    (is (float? (+ 2 3.0))))

  (testing "Large integers"
    (is (= (+ 9007199254740992 1) 9007199254740993))
    (is (= (* 12345678901234567890 2) 24691357802469135780)))

  (testing "Division"
    (is (= (/ 10 2) 5))
    (is (integer? (/ 10 2)))
    (is (= (/ 10 3) 10/3))  ; Phase 3
    (is (rational? (/ 10 3)))))
```

---

## Performance Benchmarks

Target performance (vs current f64-only):

| Operation | Current | Phase 1 | Phase 2 | Phase 3 |
|-----------|---------|---------|---------|---------|
| Small int add | 1.0x | 0.9x | 0.9x | 0.9x |
| Small int mul | 1.0x | 0.9x | 0.9x | 0.9x |
| Float add | 1.0x | 1.0x | 1.0x | 1.0x |
| Large int add | N/A | N/A | 0.1x† | 0.1x† |
| Ratio ops | N/A | N/A | N/A | 0.2x† |

† Slower than f64 but **correct** (precision worth the cost)

---

## References

1. **Clojure Documentation**
   - [Numbers](https://clojure.org/reference/data_structures#Numbers)
   - [Ratios](https://clojure.org/reference/data_structures#Ratios)

2. **Technical Papers**
   - "NaN-Boxing: Efficient Representation of Dynamic Types"
   - "The Design of Numeric Towers" (Scheme)

3. **Implementation Examples**
   - [V8 Number Representation](https://v8.dev/blog/react-cliff)
   - [LuaJIT NaN-tagging](http://lua-users.org/lists/lua-l/2009-11/msg00089.html)
   - [Clojure Source](https://github.com/clojure/clojure/blob/master/src/jvm/clojure/lang/Numbers.java)

---

## Decision Log

### Decision 1: NaN-Boxing vs Tagged Union

**Date:** 2025-01-30
**Decision:** Use NaN-boxing for Phase 1
**Rationale:**
- Zero memory overhead
- Fast type checking (single bitwise AND)
- Industry proven (V8, JavaScriptCore, LuaJIT)
- 48-bit integers sufficient for 99.9% of cases

**Trade-offs:**
- More complex implementation
- Harder to debug
- Requires careful handling of special float values

### Decision 2: Automatic Promotion vs Explicit

**Date:** 2025-01-30
**Decision:** Automatic promotion (like Clojure)
**Rationale:**
- Better UX (no explicit bigint constructor needed)
- Mathematically sound (no silent overflow)
- Matches Clojure semantics

**Trade-offs:**
- Slight performance cost for overflow checking
- Less predictable performance

---

## Current Status

**✅ Implemented:**
- f64 numbers (all operations working)
- Basic arithmetic (+, -, *, /, mod)
- Comparisons (=, <, >, <=, >=)
- Type predicate: `number?`

**❌ Not Implemented:**
- Integer/Float distinction
- BigInt (arbitrary precision)
- Rationals (exact fractions)
- Type predicates: `integer?`, `float?`, `rational?`
- Integer division semantics
- Bit operations

**📋 Next Steps:**
1. Design review of NaN-boxing implementation
2. Prototype integer/float split in separate branch
3. Performance testing
4. Full implementation of Phase 1

---

## Questions & Future Considerations

### Q: Should we support other numeric types?

**Complex numbers?** Possibly, for scientific computing
**Decimals?** Yes, for financial applications (Phase 4)
**Fixed-point?** Maybe, for embedded/low-level work

### Q: How to handle numeric literals in macros?

**Challenge:** Macro expansion needs to preserve numeric type
**Solution:** Treat number literals as AST nodes with type info

### Q: Performance vs Correctness trade-off?

**Philosophy:** Correctness first, then optimize
**Approach:** Correct by default, unsafe operations available when needed

```clojure
;; Safe by default (checks overflow)
(+ 9007199254740992 1)  ; => BigInt automatically

;; Unsafe for performance-critical code
(unchecked-add 9007199254740992 1)  ; => May overflow, fast
```

---

**Document Version:** 1.0
**Last Updated:** 2025-01-30
**Status:** Living Document
**Owner:** Clorus Core Team
