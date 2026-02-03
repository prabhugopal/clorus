# 🔢 Clorus Number System - No Compromise

**Goal:** Full Clojure-grade number types with exact semantics

Version: 1.0
Status: Design Phase
Target: Production-quality numeric tower

---

## 🎯 Design Philosophy

**NO COMPROMISE = Full Clojure Compatibility:**

```
Clojure Number Tower (what we're building):
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Long (i64)          → 42, -100, 1000000
Double (f64)        → 3.14, 1e6, NaN, Infinity
Ratio               → 1/3, 22/7 (EXACT fractions)
BigInt              → 999999999999999999999N (arbitrary size)
BigDecimal          → 3.14159265358979323846M (arbitrary precision)

Auto-promotion:     Integer → Long → BigInt
                    Float → Double → BigDecimal
Exact arithmetic:   Never lose precision unless explicit
Interop-friendly:   Works with Rust numeric types seamlessly
```

**Why This Matters:**

```
Graphics/Robotics Requirements:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

✅ Long: Pixel coords, loop counters, array indices
✅ Double: Trig, physics calculations, transforms
✅ Ratio: Exact aspect ratios (16/9), timing (1/60fps)
✅ BigInt: Cryptography, large calculations
✅ BigDecimal: Financial, high-precision science

Without these: BROKEN hardware control, incorrect calculations!
```

---

## 📊 Complete Type System

### Type 1: Long (i64) - **PRIMARY INTEGER TYPE**

```rust
// Runtime representation
pub struct Long {
    value: i64,
}

// Properties:
Range:     -9,223,372,036,854,775,808 to 9,223,372,036,854,775,807
Size:      8 bytes
Alignment: 8 bytes
Inline:    Yes (fits in Value union)

// Operations:
Arithmetic:  +, -, *, /, mod, quot, rem
Bitwise:     bit-and, bit-or, bit-xor, bit-not,
             bit-shift-left, bit-shift-right
Comparison:  =, <, >, <=, >=
Conversion:  long, int (truncate), double (promote)

// Auto-promotion:
Long + Long → Long
Long * Long → Long (or BigInt on overflow!)
Long / Long → Long (quotient) or Ratio (exact division)
Long + Double → Double (promote Long to Double)

// Overflow behavior:
(+ Long.MAX_VALUE 1) → BigInt (auto-promote!)
(* 1000000 1000000) → BigInt (overflow detection)
```

**Use Cases:**
- Pixel coordinates: `(def x 320)` `(def y 240)`
- Loop counters: `(loop [i 0] (if (< i 1000000) ...))`
- Array indices: `(nth arr 42)`
- Hardware registers: `(bit-or 0x01 0x02)`
- Sensor readings: `(def adc-value 1023)`

---

### Type 2: Double (f64) - **FLOATING POINT**

```rust
// Runtime representation
pub struct Double {
    value: f64,
}

// Properties:
Precision: ~15-17 decimal digits
Range:     ±1.7e±308
Special:   NaN, +Infinity, -Infinity, -0.0
Size:      8 bytes
Inline:    Yes (fits in Value union)

// Operations:
Arithmetic:  +, -, *, /, mod
Math:        sin, cos, sqrt, exp, log, pow
Comparison:  =, <, >, <=, >= (NaN-aware!)
Conversion:  double, float, long (truncate)

// Auto-promotion:
Double + Double → Double
Long + Double → Double
Double + Ratio → Double (lose exactness warning?)
Double / Double → Double

// Special cases:
(/ 1.0 0.0) → +Infinity
(/ -1.0 0.0) → -Infinity
(/ 0.0 0.0) → NaN
(= NaN NaN) → false  (IEEE 754 semantics)
```

**Use Cases:**
- Physics: `(def velocity 9.8)` `(def angle (/ Math/PI 4))`
- Transforms: `(def scale 1.5)`
- Smooth animations: `(lerp 0.0 1.0 0.5)`
- Sensor fusion: `(kalman-filter reading)`

---

### Type 3: Ratio - **EXACT FRACTIONS** ⭐

```rust
// Runtime representation
pub struct Ratio {
    numerator: i64,      // or BigInt for large ratios
    denominator: i64,    // always positive, gcd-reduced
}

// Properties:
Form:      Always reduced to lowest terms
Example:   2/4 → 1/2 automatically
Size:      16 bytes (two i64s)
Inline:    No (pointer to heap)

// Operations:
Arithmetic:
  (+ 1/3 1/3) → 2/3
  (* 1/3 3)   → 1
  (/ 22 7)    → 22/7 (exact!)
  (+ 1/2 1/3) → 5/6 (find common denominator)

Reduction:
  2/4 → 1/2 (gcd reduction)
  6/9 → 2/3

Conversion:
  (double 1/3) → 0.333...
  (long 22/7)  → 3 (truncate)

Comparison:
  (< 1/3 1/2) → true
  (= 2/4 1/2) → true (compare reduced forms)
```

**Why Critical:**

```clojure
;; Graphics aspect ratio
(def aspect-ratio 16/9)          ; EXACT!
(def pixels (* width aspect-ratio))  ; No rounding error

;; Timing (60fps)
(def frame-time 1/60)            ; EXACT 16.666... ms
(def total-time (* frame-time 60)) ; Exactly 1 second!

;; Music tempo
(def beat-duration 1/120)        ; 120 BPM, exact

;; Without Ratio:
(def aspect-ratio 1.777777777)   ; ❌ Rounding error!
(* aspect-ratio 9)               ; ❌ 15.999... not 16!
```

**Implementation:**

```rust
impl Ratio {
    pub fn new(num: i64, denom: i64) -> Self {
        assert!(denom != 0, "Ratio denominator cannot be zero");
        let g = gcd(num.abs(), denom.abs());
        let (n, d) = (num / g, denom / g);

        // Normalize: denominator always positive
        if d < 0 {
            Ratio { numerator: -n, denominator: -d }
        } else {
            Ratio { numerator: n, denominator: d }
        }
    }

    pub fn add(&self, other: &Ratio) -> Ratio {
        // a/b + c/d = (ad + bc) / bd
        let num = self.numerator * other.denominator
                + other.numerator * self.denominator;
        let denom = self.denominator * other.denominator;
        Ratio::new(num, denom)  // Auto-reduces!
    }

    pub fn multiply(&self, other: &Ratio) -> Ratio {
        // a/b * c/d = ac / bd
        let num = self.numerator * other.numerator;
        let denom = self.denominator * other.denominator;
        Ratio::new(num, denom)
    }

    pub fn to_double(&self) -> f64 {
        self.numerator as f64 / self.denominator as f64
    }
}

// GCD using Euclid's algorithm
fn gcd(mut a: i64, mut b: i64) -> i64 {
    while b != 0 {
        let temp = b;
        b = a % b;
        a = temp;
    }
    a
}
```

---

### Type 4: BigInt - **ARBITRARY SIZE INTEGERS** ⭐

```rust
// Runtime representation (using num-bigint crate)
pub struct BigInt {
    inner: num_bigint::BigInt,  // Arbitrary precision
}

// Properties:
Size:      Dynamic (grows as needed)
Precision: Unlimited
Example:   123456789012345678901234567890N
Inline:    No (pointer to heap)

// Operations:
Arithmetic: All operations preserve exactness
  (+ 999999999999999999N 1N) → 1000000000000000000N
  (* 123456789N 987654321N) → huge exact result

Auto-promotion:
  Long overflow → BigInt automatically
  (+ Long.MAX_VALUE 1) → BigInt

Conversion:
  (long bigint-val)   → i64 (may lose precision)
  (double bigint-val) → f64 (may lose precision)
  (ratio bigint-val)  → Ratio(bigint, 1)

Comparison:
  Works across all types
  (< 999N 1000) → true
```

**Use Cases:**
```clojure
;; Cryptography
(def large-prime 170141183460469231731687303715884105727N)
(def encrypted (mod (* message large-prime) modulus))

;; Combinatorics (factorial of large numbers)
(defn factorial [n]
  (reduce * (range 1N (inc n))))
(factorial 100)  ; Exact! Not Infinity

;; Financial (pennies in large transactions)
(def transaction-cents 999999999999999N)  ; $9,999,999,999,999.99
```

**Implementation:**
```rust
// Use num-bigint crate
use num_bigint::BigInt as NumBigInt;

pub struct BigInt {
    inner: NumBigInt,
}

impl BigInt {
    pub fn from_long(n: i64) -> Self {
        BigInt { inner: NumBigInt::from(n) }
    }

    pub fn add(&self, other: &BigInt) -> BigInt {
        BigInt { inner: &self.inner + &other.inner }
    }

    // Detect Long overflow and promote
    pub fn long_add_checked(a: i64, b: i64) -> Value {
        match a.checked_add(b) {
            Some(result) => Value::long(result),
            None => {
                // Overflow! Promote to BigInt
                let big_a = BigInt::from_long(a);
                let big_b = BigInt::from_long(b);
                Value::bigint(big_a.add(&big_b))
            }
        }
    }
}
```

---

### Type 5: BigDecimal - **ARBITRARY PRECISION DECIMALS** ⭐

```rust
// Runtime representation (using rust_decimal crate)
pub struct BigDecimal {
    inner: rust_decimal::Decimal,  // Arbitrary precision
}

// Properties:
Precision: Configurable (default 28 digits)
Example:   3.14159265358979323846M
Inline:    No (pointer to heap)

// Operations:
Arithmetic: Exact up to precision limit
  (+ 0.1M 0.2M) → 0.3M  (EXACT! Unlike floats)
  (/ 1M 3M)     → 0.333... (to precision limit)

Conversion:
  (double bigdec) → f64
  (ratio bigdec)  → Ratio (if exact)

Comparison:
  (= 0.1M 0.1) → false (different types!)
  (== 0.1M 0.1) → true (value equality)
```

**Use Cases:**
```clojure
;; Financial calculations (NO rounding errors!)
(def price 19.99M)
(def tax-rate 0.0825M)
(def total (+ price (* price tax-rate)))  ; Exact!

;; Scientific calculations needing precision
(def avogadro 6.02214076M)
(def planck 6.62607015M)

;; Robotics position (sub-millimeter precision)
(def position 123.456789012345M)  ; Exact position
```

---

## 🏗️ Runtime Architecture

### Value Union (16 bytes)

```rust
#[repr(C)]
pub struct Value {
    header: ValueHeader,  // 8 bytes (refcount + tag)
    data: ValueData,      // 8 bytes (inline or pointer)
}

#[repr(u8)]
pub enum ValueTag {
    Long = 1,          // i64 inline
    Double = 2,        // f64 inline
    Ratio = 3,         // Pointer to Ratio
    BigInt = 4,        // Pointer to BigInt
    BigDecimal = 5,    // Pointer to BigDecimal
    // ... other types
}

#[repr(C)]
pub union ValueData {
    long: i64,         // Inline: Long
    double: f64,       // Inline: Double
    ptr: *mut u8,      // Pointer: Ratio, BigInt, BigDecimal
}
```

### Memory Layout

```
Long/Double (16 bytes total):
┌─────────────────────────────────┐
│ Header (8 bytes)                │
│  ┌──────────┬─────────┐         │
│  │ RefCount │ Tag     │         │
│  │ 56 bits  │ 8 bits  │         │
│  └──────────┴─────────┘         │
├─────────────────────────────────┤
│ Data (8 bytes)                  │
│  ┌─────────────────────────┐   │
│  │ i64 or f64 (inline!)    │   │
│  └─────────────────────────┘   │
└─────────────────────────────────┘

Ratio/BigInt/BigDecimal (16 + heap):
┌─────────────────────────────────┐
│ Header (8 bytes)                │
├─────────────────────────────────┤
│ Pointer (8 bytes)               │
│  └──→ Heap allocation           │
└─────────────────────────────────┘
         ↓
    ┌──────────────────────┐
    │ Ratio/BigInt/BigDec  │
    │ (variable size)      │
    └──────────────────────┘
```

---

## ⚙️ Arithmetic System

### Type Promotion Rules

```
Promotion Hierarchy (never lose precision):
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Long ────┬──────> BigInt
         │
         └──────> Ratio ──────> BigDecimal
                   │
Double ────────────┴──────> BigDecimal

Rules:
1. Long + Long → Long (or BigInt on overflow)
2. Long + Double → Double
3. Long + Ratio → Ratio
4. Long + BigInt → BigInt
5. Double + Ratio → Double (loses exactness)
6. Ratio + Ratio → Ratio (exact!)
7. BigInt + anything → BigInt (except Double)
8. BigDecimal is the "universal" type
```

### Division Behavior

```clojure
;; Integer division options:

;; Option 1: Clojure-style (exact division → Ratio)
(/ 22 7) → 22/7                  ; Ratio (exact!)
(quot 22 7) → 3                  ; Quotient only
(rem 22 7) → 1                   ; Remainder

;; Option 2: Truncating division
(/ 22 7) → 3                     ; Truncate to Long
(/ 22.0 7) → 3.142857...         ; Double if any arg is Double

;; RECOMMENDATION: Option 1 (Clojure-style)
;; Exactness by default, explicit conversion for performance
```

### Overflow Detection

```rust
// Long arithmetic with auto-promotion to BigInt
pub fn long_add(a: i64, b: i64) -> Value {
    match a.checked_add(b) {
        Some(result) => Value::long(result),
        None => {
            // Overflow! Promote to BigInt
            let big = BigInt::from_long(a).add(&BigInt::from_long(b));
            Value::bigint(big)
        }
    }
}

pub fn long_multiply(a: i64, b: i64) -> Value {
    match a.checked_mul(b) {
        Some(result) => Value::long(result),
        None => {
            // Overflow! Promote to BigInt
            let big = BigInt::from_long(a).multiply(&BigInt::from_long(b));
            Value::bigint(big)
        }
    }
}
```

---

## 📋 Implementation Phases

### Phase 1: Long Type (Week 1) - **FOUNDATION**

**Day 1-2: Runtime**
```rust
// Add to value.rs
pub enum ValueTag {
    Long = 1,
    // ... existing tags shifted
}

pub union ValueData {
    long: i64,
    double: f64,  // Rename from 'number'
    ptr: *mut u8,
}

impl Value {
    pub fn long(n: i64) -> Self { ... }
    pub fn double(f: f64) -> Self { ... }
    pub fn is_long(&self) -> bool { ... }
    pub fn as_long(&self) -> i64 { ... }
}

// Type-aware arithmetic
#[no_mangle]
pub extern "C" fn clorus_add(a: *mut Value, b: *mut Value) -> *mut Value {
    match ((*a).tag, (*b).tag) {
        (Long, Long) => long_add((*a).as_long(), (*b).as_long()),
        (Double, Double) => Value::double((*a).as_double() + (*b).as_double()),
        (Long, Double) => Value::double((*a).as_long() as f64 + (*b).as_double()),
        (Double, Long) => Value::double((*a).as_double() + (*b).as_long() as f64),
    }
}
```

**Day 3-4: Parser**
```rust
// Parse 42 → Long, 42.0 → Double
fn parse_number(s: &str) -> Expr {
    if s.contains('.') || s.contains('e') || s.contains('E') {
        Expr::Double(s.parse::<f64>().unwrap())
    } else {
        Expr::Long(s.parse::<i64>().unwrap())
    }
}
```

**Day 5-6: Bitwise Ops**
```rust
#[no_mangle]
pub extern "C" fn clorus_bit_and(a: *mut Value, b: *mut Value) -> *mut Value {
    assert!((*a).is_long() && (*b).is_long());
    Value::long((*a).as_long() & (*b).as_long())
}

// bit-or, bit-xor, bit-not, bit-shift-left, bit-shift-right...
```

**Day 7: Testing**
```clojure
(deftest long-arithmetic
  (assert (= (+ 1 2) 3))
  (assert (= (* 5 6) 30))
  (assert (integer? 42))
  (assert (not (integer? 42.0))))

(deftest bitwise-ops
  (assert (= (bit-and 0xFF 0x0F) 15))
  (assert (= (bit-shift-left 1 8) 256)))
```

---

### Phase 2: Ratio Type (Week 2) - **EXACTNESS**

**Day 1-3: Runtime**
```rust
// ratio.rs (NEW FILE)
pub struct Ratio {
    numerator: i64,
    denominator: i64,  // Always positive, gcd-reduced
}

impl Ratio {
    pub fn new(num: i64, denom: i64) -> Self { ... }
    pub fn add(&self, other: &Ratio) -> Ratio { ... }
    pub fn multiply(&self, other: &Ratio) -> Ratio { ... }
    pub fn to_double(&self) -> f64 { ... }
}

fn gcd(a: i64, b: i64) -> i64 { ... }
```

**Day 4-5: Division Operator**
```rust
// Division returns Ratio for exact division
pub fn divide_values(a: Value, b: Value) -> Value {
    match (a.tag, b.tag) {
        (Long, Long) => {
            let ratio = Ratio::new(a.as_long(), b.as_long());
            Value::ratio(ratio)  // Exact!
        }
        (Double, _) | (_, Double) => {
            // Any Double -> Double result
            Value::double(a.as_double() / b.as_double())
        }
        (Ratio, Ratio) => {
            // a/b ÷ c/d = (a/b) * (d/c) = ad/bc
            let r1 = a.as_ratio();
            let r2 = b.as_ratio();
            let result = Ratio::new(
                r1.numerator * r2.denominator,
                r1.denominator * r2.numerator
            );
            Value::ratio(result)
        }
    }
}
```

**Day 6-7: Testing**
```clojure
(deftest ratio-exactness
  (assert (= (/ 1 3) 1/3))
  (assert (= (+ 1/3 1/3) 2/3))
  (assert (= (* 1/3 3) 1))
  (assert (ratio? 1/3))
  (assert (= (double 1/3) 0.333...)))
```

---

### Phase 3: BigInt & BigDecimal (Week 3-4) - **UNLIMITED**

**Week 3: BigInt**
```rust
// Use num-bigint crate
[dependencies]
num-bigint = "0.4"

// bigint.rs (NEW FILE)
use num_bigint::BigInt as NumBigInt;

pub struct BigInt {
    inner: NumBigInt,
}

// Overflow detection & auto-promotion
pub fn long_add_checked(a: i64, b: i64) -> Value {
    a.checked_add(b)
        .map(|r| Value::long(r))
        .unwrap_or_else(|| {
            Value::bigint(BigInt::from_long(a).add(&BigInt::from_long(b)))
        })
}
```

**Week 4: BigDecimal**
```rust
// Use rust_decimal crate
[dependencies]
rust_decimal = "1.32"

// bigdecimal.rs (NEW FILE)
use rust_decimal::Decimal;

pub struct BigDecimal {
    inner: Decimal,
}
```

---

## 🧪 Testing Strategy

### Unit Tests

```clojure
;; tests/lang/core/numbers.clr

;; Long tests
(deftest test-long-arithmetic
  (assert (= (+ 1 2) 3))
  (assert (= (- 10 3) 7))
  (assert (= (* 5 6) 30))
  (assert (= (quot 10 3) 3))
  (assert (= (rem 10 3) 1)))

(deftest test-long-overflow
  ;; Should auto-promote to BigInt
  (def max Long.MAX_VALUE)
  (def result (+ max 1))
  (assert (bigint? result))
  (assert (> result max)))

;; Double tests
(deftest test-double-arithmetic
  (assert (< (abs (- (+ 1.0 2.0) 3.0)) 0.0001))
  (assert (double? 3.14))
  (assert (= (/ 1.0 0.0) +Infinity)))

;; Ratio tests
(deftest test-ratio-exactness
  (assert (= (/ 1 3) 1/3))
  (assert (= (+ 1/3 1/3) 2/3))
  (assert (= (+ 1/2 1/3) 5/6))
  (assert (= (* 22/7 7) 22))
  (assert (ratio? 1/3)))

;; BigInt tests
(deftest test-bigint-unlimited
  (def huge 999999999999999999999N)
  (def bigger (+ huge huge))
  (assert (bigint? bigger))
  (assert (> bigger huge)))

;; BigDecimal tests
(deftest test-bigdecimal-precision
  (assert (= (+ 0.1M 0.2M) 0.3M))  ; Exact!
  (assert (not (= (+ 0.1 0.2) 0.3)))  ; Doubles fail!
  (assert (bigdecimal? 0.1M)))

;; Mixed type promotion
(deftest test-type-promotion
  (assert (double? (+ 1 2.0)))
  (assert (ratio? (+ 1 1/3)))
  (assert (bigint? (+ 1 999N))))

;; Bitwise operations
(deftest test-bitwise
  (assert (= (bit-and 0xFF 0x0F) 15))
  (assert (= (bit-or 0x01 0x02) 3))
  (assert (= (bit-shift-left 1 8) 256))
  (assert (= (bit-shift-right 256 4) 16)))
```

### Integration Tests

```clojure
;; Graphics test
(deftest test-graphics-math
  ;; Exact aspect ratio
  (def aspect 16/9)
  (def width 1920)
  (def height (quot (* width 9) 16))
  (assert (= height 1080)))

;; Robotics test
(deftest test-robotics-hardware
  ;; PWM register (needs exact integers)
  (def duty-cycle 50)  ; 50% duty
  (def pwm-max 1023)
  (def pwm-value (quot (* duty-cycle pwm-max) 100))
  (assert (= pwm-value 511))

  ;; Control register (bitwise)
  (def enable-bit 0x01)
  (def speed-bits (bit-shift-left 5 4))
  (def control-reg (bit-or enable-bit speed-bits))
  (assert (= control-reg 0x51)))

;; Financial test
(deftest test-financial-precision
  (def price 19.99M)
  (def tax 0.0825M)
  (def total (+ price (* price tax)))
  (assert (= total 21.63M)))  ; Exact!
```

---

## ✅ Success Criteria

After full implementation:

```
✅ Long type works (i64)
✅ Double type works (f64)
✅ Ratio type works (exact fractions)
✅ BigInt works (unlimited size)
✅ BigDecimal works (arbitrary precision)
✅ Auto-promotion on overflow
✅ Bitwise operations on integers only
✅ Type predicates (long?, double?, ratio?, etc.)
✅ Full Clojure numeric compatibility
✅ No precision loss unless explicit
✅ Graphics math works perfectly
✅ Robotics hardware control works
✅ All tests pass (100+ test cases)
```

---

## 📊 Comparison with Current State

```
Before (f64 only):
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
❌ 42 → 42.0 (wrong type!)
❌ (bit-and 255 15) → BROKEN
❌ (/ 1 3) → 0.333... (not exact)
❌ Overflow → Infinity or NaN
❌ No exact fractions

After (Full Number Tower):
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
✅ 42 → Long (correct!)
✅ (bit-and 255 15) → 15 (works!)
✅ (/ 1 3) → 1/3 (exact Ratio!)
✅ Overflow → BigInt (auto-promote!)
✅ Exact arithmetic available

Impact:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Graphics:    ❌ Broken → ✅ Perfect
Robotics:    ❌ Broken → ✅ Perfect
Clojure:     ❌ 50% compatible → ✅ 100% compatible
Reliability: ❌ Precision errors → ✅ Exact by default
```

---

## 🎯 Timeline

```
Week 1:  Long type + bitwise ops         (7 days)
Week 2:  Ratio type + exact division     (7 days)
Week 3:  BigInt + overflow detection     (7 days)
Week 4:  BigDecimal + testing            (7 days)
─────────────────────────────────────────────────
Total:   1 month for SOLID number system

Then continue with stdlib enhancement...
```

---

## 🚀 Why This First?

**Dependencies:**

```
Number System
     ↓
├─→ Type predicates (integer?, ratio?, etc.)
├─→ Math library (needs exact types)
├─→ Bitwise ops (robotics)
├─→ Collections (exact indices)
├─→ Graphics (exact coords)
└─→ Everything else!

Without proper numbers: NOTHING ELSE WORKS RIGHT!
```

---

**Ready to implement? This is production-quality, no-compromise Clojure numerics! 🔢**
