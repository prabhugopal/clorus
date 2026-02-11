# Implementation Plan: deftype/defrecord + Protocol Dispatch

## Overview
Add Clojure-style types and protocols to Clorus for proper polymorphic dispatch.

## Phase 1: Parser Support (clorus-syntax)

### New AST Nodes
```rust
// In clorus-syntax/src/expr.rs

pub enum Expr {
    // ... existing variants ...

    Deftype {
        name: String,
        fields: Vec<String>,
        protocols: Vec<ProtocolImpl>,
        span: Span,
    },

    Defrecord {
        name: String,
        fields: Vec<String>,
        protocols: Vec<ProtocolImpl>,
        span: Span,
    },

    ExtendProtocol {
        protocol: String,
        type_name: String,
        methods: Vec<(String, Vec<String>, Box<Expr>)>,  // (method, params, body)
        span: Span,
    },
}

pub struct ProtocolImpl {
    pub protocol_name: String,
    pub methods: Vec<ProtocolMethod>,
}

pub struct ProtocolMethod {
    pub name: String,
    pub params: Vec<String>,
    pub body: Box<Expr>,
}
```

### Parser Changes
```clojure
;; Syntax to support:
(deftype Button [props state]
  IComponent
  (render [this ctx]
    (gfx/draw-rounded-rect ...)))

(defrecord Point [x y])

(extend-protocol IComponent
  CustomWidget
  (render [this ctx]
    ...))
```

**Files to modify:**
- `crates/clorus-syntax/src/parser.rs` - Add parsing for deftype/defrecord
- `crates/clorus-syntax/src/expr.rs` - Add AST nodes

---

## Phase 2: Runtime Type System (clorus-runtime)

### Type Registry
```rust
// crates/clorus-runtime/src/types.rs (NEW FILE)

use std::collections::HashMap;
use std::sync::RwLock;
use once_cell::sync::Lazy;

// Global type registry
static TYPE_REGISTRY: Lazy<RwLock<TypeRegistry>> = Lazy::new(|| {
    RwLock::new(TypeRegistry::new())
});

pub struct TypeRegistry {
    types: HashMap<String, TypeInfo>,
}

pub struct TypeInfo {
    pub name: String,
    pub fields: Vec<String>,
    pub constructor: unsafe extern "C" fn(*mut *mut Value, usize) -> *mut Value,
    pub protocol_impls: HashMap<String, ProtocolImpl>,
}

pub struct ProtocolImpl {
    pub methods: HashMap<String, unsafe extern "C" fn(*mut Value, ...) -> *mut Value>,
}

impl TypeRegistry {
    pub fn register_type(&mut self, info: TypeInfo) {
        self.types.insert(info.name.clone(), info);
    }

    pub fn get_method(&self, type_name: &str, protocol: &str, method: &str)
        -> Option<unsafe extern "C" fn(*mut Value, ...) -> *mut Value> {
        self.types.get(type_name)?
            .protocol_impls.get(protocol)?
            .methods.get(method)
            .copied()
    }
}

// FFI functions
#[no_mangle]
pub extern "C" fn clorus_register_type(
    name: *const c_char,
    fields: *mut *const c_char,
    field_count: usize,
    constructor: unsafe extern "C" fn(*mut *mut Value, usize) -> *mut Value
) {
    // ... implementation
}

#[no_mangle]
pub extern "C" fn clorus_get_type_name(val: *mut Value) -> *const c_char {
    // Return the type name for a value
}
```

### Value Type Metadata
```rust
// Extend Value struct to include type information
pub struct Value {
    header: Header,
    type_name: Option<String>,  // NEW: Track type name
    data: ValueData,
}

// New ValueTag for custom types
pub enum ValueTag {
    // ... existing tags ...
    CustomType,  // NEW: For deftype/defrecord instances
}
```

**Files to create/modify:**
- `crates/clorus-runtime/src/types.rs` - NEW: Type registry
- `crates/clorus-runtime/src/value.rs` - Add type metadata
- `crates/clorus-runtime/src/lib.rs` - Export type functions

---

## Phase 3: Codegen Support (clorus-codegen)

### Type Construction
```rust
// In clorus-codegen/src/codegen.rs

impl<'ctx> CodeGen<'ctx> {
    pub fn compile_deftype(&mut self, name: &str, fields: &[String],
                          protocols: &[ProtocolImpl]) -> Result<(), String> {
        // 1. Generate constructor function
        let constructor_name = format!("{}_new", name);

        // 2. Generate protocol method implementations
        for protocol_impl in protocols {
            for method in &protocol_impl.methods {
                self.compile_protocol_method(name, &protocol_impl.protocol_name, method)?;
            }
        }

        // 3. Register type at runtime
        self.emit_type_registration(name, fields, protocols)?;

        Ok(())
    }

    pub fn compile_protocol_method(&mut self, type_name: &str,
                                   protocol: &str, method: &ProtocolMethod) -> Result<(), String> {
        // Generate LLVM function for this method
        let fn_name = format!("{}_{}_{}",type_name, protocol, method.name);

        // Compile method body with 'this' bound to first parameter
        // ...
    }

    pub fn compile_protocol_call(&mut self, protocol: &str, method: &str,
                                 receiver: &Expr, args: &[Expr]) -> Result<(), String> {
        // 1. Compile receiver expression
        let receiver_val = self.compile_expr(receiver)?;

        // 2. Get type name at runtime
        let type_name = self.emit_call("clorus_get_type_name", &[receiver_val])?;

        // 3. Lookup method in registry
        let method_ptr = self.emit_call("clorus_get_protocol_method",
                                       &[type_name, protocol_str, method_str])?;

        // 4. Call the method
        let result = self.emit_indirect_call(method_ptr, &compiled_args)?;

        Ok(())
    }
}
```

**Files to modify:**
- `crates/clorus-codegen/src/codegen.rs` - Add type compilation
- `crates/clorus-codegen/src/codegen.rs` - Add protocol dispatch

---

## Phase 4: Protocol Dispatch Runtime (clorus-runtime)

### Protocol Lookup
```rust
// crates/clorus-runtime/src/protocols.rs (NEW FILE)

#[no_mangle]
pub extern "C" fn clorus_get_protocol_method(
    type_name: *const c_char,
    protocol: *const c_char,
    method: *const c_char
) -> Option<unsafe extern "C" fn(*mut Value, ...) -> *mut Value> {
    let type_str = unsafe { CStr::from_ptr(type_name).to_str().ok()? };
    let protocol_str = unsafe { CStr::from_ptr(protocol).to_str().ok()? };
    let method_str = unsafe { CStr::from_ptr(method).to_str().ok()? };

    TYPE_REGISTRY.read().ok()?
        .get_method(type_str, protocol_str, method_str)
}

#[no_mangle]
pub extern "C" fn clorus_satisfies_protocol(
    val: *mut Value,
    protocol: *const c_char
) -> bool {
    // Check if value's type implements protocol
}
```

**Files to create:**
- `crates/clorus-runtime/src/protocols.rs` - NEW: Protocol dispatch

---

## Phase 5: CORAL Integration

### Update protocols.clrs
```clojure
;; coral-ui/src/core/protocols.clrs

(defprotocol IComponent
  (measure [this constraints])
  (layout [this bounds])
  (render [this ctx]))

;; No more manual dispatch needed!
;; Just call (render component ctx) and it dispatches automatically
```

### Update components to use deftype
```clojure
;; coral-ui/src/components/button.clrs

(deftype Button [props state bounds]
  IComponent
  (measure [this constraints]
    {:width (get-in this [:props :width] 100.0)
     :height (get-in this [:props :height] 30.0)})

  (layout [this new-bounds]
    (assoc this :bounds new-bounds))

  (render [this ctx]
    (let [bounds (:bounds this)
          props (:props this)
          label (get props :label "Button")]
      (gfx/draw-rounded-rect (:window ctx)
                             (:x bounds) (:y bounds)
                             (:w bounds) (:h bounds)
                             5.0 0x4472C4)
      (gfx/draw-text (:window ctx)
                     (+ (:x bounds) 10.0)
                     (+ (:y bounds) 15.0)
                     label 14.0 0xFFFFFF))))

;; Constructor function automatically generated
(defn button [props]
  (Button. props (atom {}) nil))
```

---

## Implementation Timeline

### Week 1: Parser & AST
- [ ] Add deftype/defrecord AST nodes
- [ ] Implement parsing for deftype/defrecord
- [ ] Add tests for parsing
- [ ] Update macro expander if needed

### Week 2: Runtime Type System
- [ ] Create type registry infrastructure
- [ ] Add type metadata to Value struct
- [ ] Implement type registration FFI
- [ ] Add protocol lookup functions

### Week 3: Codegen
- [ ] Implement deftype compilation
- [ ] Generate constructor functions
- [ ] Compile protocol method implementations
- [ ] Emit type registration calls

### Week 4: Protocol Dispatch
- [ ] Implement runtime protocol dispatch
- [ ] Add protocol method lookup
- [ ] Handle method not found errors
- [ ] Performance optimization (caching)

### Week 5: Testing & Integration
- [ ] Write comprehensive tests
- [ ] Update CORAL components to use deftype
- [ ] Remove manual dispatch from protocols.clrs
- [ ] Documentation

---

## Critical Files

### To Create
1. `crates/clorus-runtime/src/types.rs`
2. `crates/clorus-runtime/src/protocols.rs`

### To Modify
1. `crates/clorus-syntax/src/expr.rs`
2. `crates/clorus-syntax/src/parser.rs`
3. `crates/clorus-codegen/src/codegen.rs`
4. `crates/clorus-runtime/src/value.rs`
5. `crates/clorus-runtime/src/lib.rs`

---

## Testing Strategy

### Unit Tests
```clojure
;; Test deftype
(deftype Point [x y])
(def p (Point. 10 20))
(assert (= (:x p) 10))

;; Test protocol
(defprotocol IShow
  (show [this]))

(deftype Person [name age]
  IShow
  (show [this] (:name this)))

(def person (Person. "Alice" 30))
(assert (= (show person) "Alice"))

;; Test extend-protocol
(extend-protocol IShow
  Point
  (show [this]
    (str "(" (:x this) ", " (:y this) ")")))

(assert (= (show p) "(10, 20)"))
```

---

## Success Criteria

1. ✅ deftype creates new types with fields
2. ✅ Protocol methods dispatch correctly based on type
3. ✅ extend-protocol adds protocols to existing types
4. ✅ CORAL components work without manual dispatch
5. ✅ Performance is acceptable (no major slowdown)
6. ✅ Error messages are clear
7. ✅ Documentation is complete

---

## Notes

- This is a **major language feature** affecting parser, compiler, and runtime
- Estimated effort: **3-4 weeks** full-time
- Requires careful testing to avoid regressions
- Should be done incrementally with tests at each phase
- Consider performance implications of runtime dispatch
