# AOT Type Inference & Optimization for Clorus

## Question: Can the compiler decide f32 vs f64 based on usage?

**Short Answer**: Yes! This is called **type-based optimization** and **bit-width inference**.

## Current State (JIT Mode)

Right now, everything is `f64`:
```clojure
42      ; f64
3.14    ; f64
```

## AOT Optimization Strategies

### 1. **Static Type Inference** (Recommended)

The compiler analyzes usage and chooses the smallest safe type:

```clojure
; Example program
(def x 42)           ; Compiler infers: could be i32, i64, or f64
(def y 3.14)         ; Compiler infers: must be f64 (has decimal)
(def z 10000000000)  ; Compiler infers: needs i64 (too big for i32)

(+ x 1)              ; i32 math
(+ y 1.0)            ; f64 math
(* x y)              ; x promoted to f64, then f64 math
```

**Implementation**:
```rust
enum InferredType {
    I32,    // 32-bit integer
    I64,    // 64-bit integer
    F32,    // 32-bit float
    F64,    // 64-bit float
}

fn infer_type(expr: &Expr) -> InferredType {
    match expr {
        Expr::Number(n) if n.fract() == 0.0 && *n < i32::MAX as f64 => {
            InferredType::I32  // Fits in i32
        }
        Expr::Number(n) if n.fract() == 0.0 => {
            InferredType::I64  // Integer, but needs i64
        }
        Expr::Number(_) => InferredType::F64,  // Has decimal

        Expr::List(ops) => {
            // Infer from operations
            // If all operands are integers, result is integer
            // If any operand is float, result is float
        }
    }
}
```

### 2. **Range Analysis**

Determine safe ranges and pick smallest type:

```clojure
(def x 100)
; Range analysis: x ∈ [100, 100]
; Fits in: i8, i16, i32, i64, f32, f64
; Choose: i8 (smallest)

(def y (+ x 1))
; Range: y ∈ [101, 101]
; Choose: i8

(def z (* x 1000))
; Range: z ∈ [100000, 100000]
; Needs: i32 (too big for i16)
```

**LLVM has built-in range analysis!**

### 3. **Profile-Guided Optimization (PGO)**

Run program, collect statistics, recompile:

```bash
# Step 1: Compile with instrumentation
clorus build --pgo-generate

# Step 2: Run with typical workload
./target/debug/my-program < typical-input.txt

# Step 3: Recompile with profile data
clorus build --pgo-use
```

Compiler sees:
- "x is always < 1000" → use i16
- "y needs full f64 precision" → use f64
- "z is only used in comparisons" → use i32

### 4. **Gradual Typing** (Future)

Let programmer provide hints:

```clojure
; Explicit types
(def x ^i32 42)
(def y ^f64 3.14)

; Or inferred
(def x 42)      ; Compiler figures it out
```

## Real-World Example: Julia Language

Julia does this successfully:

```julia
# Julia infers types at compile time
x = 42          # typeof(x) = Int64
y = 3.14        # typeof(y) = Float64
z = x + y       # promotes x to Float64, result is Float64
```

**Performance**: Type-specialized code is 100-300x faster than untyped!

## For Clorus AOT

### Phase 1: Simple Inference (Week 1-2)
```rust
// In AOT codegen
fn compile_number(&mut self, n: f64) -> Value {
    if n.fract() == 0.0 && n >= i32::MIN as f64 && n <= i32::MAX as f64 {
        // Use i32
        self.context.i32_type().const_int(n as u64, false)
    } else {
        // Use f64
        self.context.f64_type().const_float(n)
    }
}
```

### Phase 2: Flow Analysis (Week 3-4)
```rust
struct TypeInference {
    variables: HashMap<String, InferredType>,
}

impl TypeInference {
    fn infer_expr(&mut self, expr: &Expr) -> InferredType {
        match expr {
            Expr::Number(n) => self.infer_number(n),
            Expr::Symbol(name) => self.variables.get(name).cloned().unwrap_or(InferredType::F64),
            Expr::List(ops) => self.infer_operation(ops),
            // ...
        }
    }

    fn infer_operation(&mut self, ops: &[Expr]) -> InferredType {
        let types: Vec<_> = ops.iter().map(|e| self.infer_expr(e)).collect();

        // If any operand is float, result is float
        if types.iter().any(|t| matches!(t, InferredType::F32 | InferredType::F64)) {
            InferredType::F64
        } else {
            // All integers - find widest
            types.iter().max().cloned().unwrap()
        }
    }
}
```

### Phase 3: LLVM Optimization Passes (Automatic!)

LLVM can optimize automatically:

```llvm
; Before optimization
define double @example(double %x) {
  %y = fadd double %x, 1.0
  %z = fptosi double %y to i32  ; Convert to int
  %result = sitofp i32 %z to double  ; Convert back
  ret double %result
}

; After LLVM optimization
define double @example(double %x) {
  %y = fadd double %x, 1.0
  ; LLVM notices the round-trip and optimizes
  ret double %y
}
```

## Memory Savings

Type specialization saves memory:

```
Current (all f64):
  [1, 2, 3, 4, 5]
  = 5 × 8 bytes = 40 bytes

With i32:
  [1, 2, 3, 4, 5]
  = 5 × 4 bytes = 20 bytes  (50% savings!)

With i8 (if range allows):
  [1, 2, 3, 4, 5]
  = 5 × 1 byte = 5 bytes  (87% savings!)
```

## Recommendations

### For JIT Mode (Current)
Keep `f64` everywhere - simple and works.

### For AOT Mode (Future)

**Level 1** (Easy - 1 week):
```rust
// Simple literal inference
42      → i32
3.14    → f64
true    → i1 (LLVM bool)
```

**Level 2** (Medium - 2-3 weeks):
```rust
// Flow-sensitive inference
(def x 42)        → infer i32
(def y (+ x 1))   → infer i32 (propagate)
(def z (+ x 1.0)) → infer f64 (promote x)
```

**Level 3** (Advanced - 1-2 months):
```rust
// Whole-program analysis
// Analyze all uses of variable
// Pick smallest safe type
// Insert conversions as needed
```

### Optional: Let User Choose

```bash
clorus build --int-size=32        # Use i32 by default
clorus build --int-size=64        # Use i64 by default
clorus build --float-size=32      # Use f32 where safe
clorus build --optimize-types     # Automatic inference
```

## Example Implementation

```rust
// Type inference pass (before codegen)
pub struct TypeInferencePass {
    types: HashMap<String, LLVMType>,
}

impl TypeInferencePass {
    pub fn run(&mut self, ast: &[Expr]) {
        // Pass 1: Collect constraints
        for expr in ast {
            self.collect_constraints(expr);
        }

        // Pass 2: Solve constraints
        self.solve_constraints();

        // Pass 3: Insert conversions
        self.insert_conversions();
    }

    fn collect_constraints(&mut self, expr: &Expr) {
        match expr {
            Expr::Number(n) => {
                if n.fract() == 0.0 {
                    // Constraint: can be integer
                    self.add_constraint(expr, Constraint::Integer);
                }
            }
            Expr::List(ops) if ops[0] == Expr::Symbol("+") => {
                // Constraint: operands must have same type
                self.add_constraint(ops[1], Constraint::SameAs(ops[2]));
            }
            // ...
        }
    }
}
```

## TL;DR

**Yes**, the compiler can and should infer types for AOT:

1. **JIT Mode**: Keep f64 (simple, working)
2. **AOT Mode**: Add type inference
3. **Start Simple**: Literal numbers → i32 or f64
4. **Later**: Flow analysis → propagate types
5. **Future**: Whole-program → optimal types

**Benefits**:
- 50-90% memory savings
- Faster arithmetic (i32 vs f64)
- Better cache performance
- SIMD opportunities

**Example**:
```clojure
; Source
(def nums [1 2 3 4 5])

; JIT: [f64, f64, f64, f64, f64] = 40 bytes
; AOT: [i8, i8, i8, i8, i8] = 5 bytes
```

Want me to implement basic type inference for AOT mode later?
