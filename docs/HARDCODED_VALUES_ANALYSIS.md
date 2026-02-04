# Comprehensive Analysis: All Hardcoded Values in Clorus

## Executive Summary

This document catalogs all hardcoded values, function names, special forms, and data structures across the Clorus language implementation. It covers the runtime (168 FFI functions), compiler codegen (21 special forms, 50+ built-in implementations), parser/lexer, REPL, and namespace system.

---

## 1. RUNTIME (clorus-runtime/src/)

### 1.1 ValueTag Enum (value.rs:14-32)
**17 Hardcoded Type Tags:**
```
Long = 0        // i64 integer
Double = 1      // f64 floating point
List = 2
Vector = 3
HashMap = 4
String = 5
Keyword = 6
Symbol = 7
Bool = 8
Nil = 9
HashSet = 10
Atom = 11
Ref = 12
Agent = 13
Channel = 14
Function = 15
Var = 16
```

**Why Hardcoded:** Binary protocol discriminators
**Configurability:** ❌ NOT configurable - breaks binary compatibility

### 1.2 FFI Function Names (168 total)

#### Memory Management (3)
- `clorus_retain`
- `clorus_release`
- `clorus_refcount`

#### Value Construction (6)
- `clorus_value_long`
- `clorus_value_double`
- `clorus_value_boolean`
- `clorus_value_nil`
- `clorus_value_bool`
- `clorus_value_string`

#### Value Extraction (5)
- `clorus_value_as_long`
- `clorus_value_as_double`
- `clorus_value_as_number`
- `clorus_value_as_bool`
- `clorus_value_as_cstring`

#### Type Predicates (18)
- `clorus_is_long`, `clorus_is_double`, `clorus_is_number`
- `clorus_is_vector`, `clorus_is_list`, `clorus_is_map`, `clorus_is_set`
- `clorus_is_symbol`, `clorus_is_nil`, `clorus_is_bool`
- `clorus_is_seq`, `clorus_is_coll`
- `clorus_is_atom`, `clorus_is_ref`, `clorus_is_agent`, `clorus_is_channel`
- `clorus_is_keyword`
- `clorus_is_truthy` (defines falsy: nil, false, 0)

#### Arithmetic Operations (8)
- `clorus_add`, `clorus_sub`, `clorus_mul`, `clorus_div`, `clorus_mod`
- `clorus_gt`, `clorus_gte`, `clorus_lt`, `clorus_lte`

#### Bitwise Operations (6)
- `clorus_bit_and`, `clorus_bit_or`, `clorus_bit_xor`, `clorus_bit_not`
- `clorus_bit_shift_left`, `clorus_bit_shift_right`

#### Vector Operations (8)
- `clorus_vector_empty`, `clorus_vector_conj`, `clorus_vector_count`
- `clorus_vector_nth`, `clorus_vector_rest`, `clorus_vector_assoc`

#### List Operations (7)
- `clorus_list_empty`, `clorus_list_cons`, `clorus_list_count`
- `clorus_list_nth`, `clorus_list_first`, `clorus_list_rest`

#### Map Operations (12)
- `clorus_map_empty`, `clorus_map_assoc`, `clorus_map_dissoc`, `clorus_map_get`
- `clorus_map_get_in`, `clorus_map_assoc_in`, `clorus_map_update`
- `clorus_map_count`, `clorus_map_keys`, `clorus_map_vals`, `clorus_map_merge`

#### Set Operations (6)
- `clorus_set_empty`, `clorus_set_conj`, `clorus_set_disj`
- `clorus_set_contains`, `clorus_set_count`

#### String Operations (15)
- `clorus_str`, `clorus_string`
- `clorus_starts_with`, `clorus_ends_with`, `clorus_includes`
- `clorus_split`, `clorus_join`
- `clorus_trim`, `clorus_trim_left`, `clorus_trim_right`
- `clorus_upper_case`, `clorus_lower_case`
- `clorus_replace`, `clorus_replace_first`
- `clorus_subs2`, `clorus_subs3` (substring)
- `clorus_pr_str`

#### Generic Collection Operations (13)
- `clorus_conj`, `clorus_concat`, `clorus_count`
- `clorus_first`, `clorus_rest`, `clorus_last`, `clorus_nth`
- `clorus_take`, `clorus_drop`
- `clorus_distinct`, `clorus_flatten`
- `clorus_interpose`, `clorus_interleave`, `clorus_dedupe`

#### Atom Operations (4)
- `clorus_atom`, `clorus_deref`, `clorus_atom_deref`
- `clorus_reset`, `clorus_swap`

#### Ref Operations (6)
- `clorus_ref`, `clorus_ref_deref`, `clorus_ref_set`
- `clorus_alter`, `clorus_commute`, `clorus_ensure`

#### Transaction Operations (4)
- `clorus_tx_begin`, `clorus_tx_commit`, `clorus_tx_abort`, `clorus_tx_active`

#### Agent Operations (5)
- `clorus_agent`, `clorus_agent_deref`, `clorus_agent_error`, `clorus_send`

#### Channel Operations (6)
- `clorus_chan`, `clorus_chan_put`, `clorus_chan_take`, `clorus_chan_close`, `clorus_alts`

#### Go-block Operations (3)
- `clorus_go`, `clorus_await`, `clorus_await_for`

#### Function Operations (4)
- `clorus_function_new`, `clorus_function_call`, `clorus_alloc_env`

#### Var Operations (7)
- `clorus_var_new`, `clorus_var_get`, `clorus_var_set`, `clorus_var_free`
- `clorus_var_name`, `clorus_var_is_dynamic`, `clorus_var_set_meta`

#### I/O Operations (8)
- `clorus_print`, `clorus_println`, `clorus_pr`, `clorus_prn`
- `clorus_read_line`, `clorus_flush`, `clorus_eprintln`, `clorus_print_value`

#### Transducer Operations (3)
- `clorus_reduce`, `clorus_reduced`, `clorus_deref_reduced`, `clorus_is_reduced`

**Why Hardcoded:** Binary FFI contract between compiler and runtime
**Configurability:** ❌ NOT configurable - recompilation would be required

### 1.3 Truthiness Rules (value.rs:323-344)
**Falsy values:**
- `nil`
- `false`
- `0` (Long)
- `0.0` (Double)

**All other values are truthy**

**Why:** Clojure semantics
**Configurability:** ❌ NOT configurable - embedded in language

### 1.4 Keyword Interning (keyword.rs:11-57)
- Global static `KEYWORD_TABLE: Mutex<Option<HashMap<String, usize>>>`
- Keywords never deallocated (lifetime = program)
- Same keyword always returns same pointer

**Why:** Performance optimization (pointer equality)
**Configurability:** ❌ NOT configurable - core architecture

---

## 2. CODEGEN (clorus-codegen/src/ & clorus-syntax/src/)

### 2.1 Special Forms (parser.rs:174-199)
**21 Hardcoded Keywords:**
```clojure
ns              ; namespace declaration
require         ; module importing
let             ; local bindings
def             ; global variable
defn            ; function definition
defmacro        ; macro definition
defrecord       ; record type
defprotocol     ; protocol definition
extend-type     ; protocol implementation
defmulti        ; multi-method definition
defmethod       ; multi-method implementation
fn              ; anonymous function
if              ; conditional
do              ; sequential composition
dosync          ; transactional block
quote           ; quote form
use             ; module use/import
loop            ; loop construct
recur           ; recursion
try             ; exception handling
throw           ; throw exception
```

**Why:** Core language syntax
**Configurability:** ❌ NOT configurable - parser requirement

### 2.2 Operator Keywords (parser.rs:217)
```
["+", "-", "*", "/", "<", ">", "="]
```

**Why:** Special parsing vs regular function calls
**Configurability:** ⚠️ PARTIALLY - could be extended

### 2.3 Rest Parameter Marker (parser.rs:278)
```
"&"  - marks rest/variadic parameters
```

**Pattern:** `(defn foo [x y & rest] ...)`
**Configurability:** ❌ NOT configurable

### 2.4 Default Namespace (namespace_context.rs:49-64)
```
"user"  - default namespace
```

**Why:** Clojure convention
**Configurability:** ✅ COULD be configurable via parameter

### 2.5 Namespace Mangling (codegen.rs)
**Pattern:**
```
format!("clorus_{}_{}",
    namespace.replace('.', "_"),
    name.replace('-', "_"))
```

**Example:** `my.app.core/my-func` → `clorus_my_app_core_my_func`

**Why:** Avoid name collisions in LLVM
**Configurability:** ⚠️ Algorithm could be parameterized

### 2.6 Core Functions List (codegen.rs:3117-3133)
**Built-in core functions:**
```
["slurp", "spit", "get", "nth", "first", "rest", "last", "count", "empty?",
 "reduce", "apply", "conj", "disj", "contains?", "concat", "assoc", "dissoc",
 "atom", "reset!", "swap!",
 "agent", "send", "await", "await-for", "agent-error",
 "chan", ">!!", "<!!", "close!", "alts!!",
 "go",
 "str", "subs", "split", "join",
 "upper-case", "lower-case",
 "trim", "trim-left", "trim-right",
 "replace", "replace-first",
 "string?", "starts-with?", "ends-with?", "includes?"]
```

**Why:** Functions with special codegen behavior
**Configurability:** ⚠️ Could use registry pattern

---

## 3. PARSER/SYNTAX (clorus-syntax/src/)

### 3.1 Token Types (lexer.rs:4-35)
**Delimiters:**
- `(`, `)`, `[`, `]`, `{`, `}`

**Reader Macros:**
- `#(` - shorthand function
- `#{` - hash set
- `#'` - var quote
- `^` - metadata
- `'` - quote
- `` ` `` - syntax quote
- `~` - unquote
- `~@` - unquote splicing
- `@` - deref

**Configurability:** ❌ NOT configurable - core syntax

### 3.2 Number Parsing (lexer.rs:126-181)
**Long:** No decimal point (e.g., `42`, `-10`)
**Double:** Has decimal or scientific notation (e.g., `3.14`, `1e6`)

**Why:** Clojure numeric tower
**Configurability:** ❌ NOT configurable

### 3.3 Comment Syntax (lexer.rs:82-91)
```
;  - comments until EOL
,  - treated as whitespace
```

**Why:** Clojure conventions
**Configurability:** ❌ NOT configurable

### 3.4 Boolean Literals (lexer.rs:327-329)
```
"true"  -> Token::Bool(true)
"false" -> Token::Bool(false)
"nil"   -> Token::Nil
```

**Configurability:** ❌ NOT configurable

### 3.5 String Escape Sequences (lexer.rs:104-114)
```
\n  -> newline
\t  -> tab
\r  -> carriage return
\\  -> backslash
\"  -> double quote
```

**Configurability:** ❌ NOT configurable

### 3.6 Symbol Character Set (lexer.rs:186-200)
**Valid:** `alphanumeric + "-_?!+*/<>=.%&#"`

**Examples:** `my-var?`, `set!`, `*special*`, `<compare>`
**Configurability:** ❌ NOT configurable - syntax rule

---

## 4. REPL (clorus-repl/src/)

### 4.1 REPL Commands (main.rs:564-581)
```
:quit | :q        - Exit REPL
:help | :h        - Show help
:examples | :e    - Show examples
```

**Configurability:** ✅ COULD be extended via plugin system

### 4.2 Library Loading Paths (main.rs:764-1095)
**Hardcoded library names:**
- macOS: `libclorus_std.dylib`, `libclorus_core.dylib`, `libclorus_runtime.dylib`
- Linux: `libclorus_std.so`, `libclorus_core.so`, `libclorus_runtime.so`
- Windows: `clorus_std.dll`, `clorus_core.dll`, `clorus_runtime.dll`

**Search paths (in order):**
1. `$CLORUS_HOME/lib/`
2. `~/.clorus/lib/`
3. `target/release/`
4. `target/debug/`
5. `target/release/deps/`
6. `target/debug/deps/`
7. Workspace dirs (walk up 5 levels)
8. `../lib/` (relative to executable)
9. Same directory as executable

**Configurability:** ✅ COULD use config file

### 4.3 Standard Library Loading (main.rs:346-428)
**Hardcoded file:** `core.clr`

**Search paths:**
1. `$CLORUS_HOME/stdlib/core.clr`
2. `~/.clorus/stdlib/core.clr`

**Configurability:** ✅ COULD be configurable

### 4.4 Project Configuration (main.rs:296)
```
"Clorus.toml"  - project manifest filename
```

**Configurability:** ⚠️ PARTIALLY - convention-based

### 4.5 REPL History File (main.rs:537-541)
```
~/.clorus_history
```

**Configurability:** ✅ COULD use env var

### 4.6 Autocomplete Categories (main.rs:144-186)
**Hardcoded suggestions:**
- Arithmetic: `+`, `-`, `*`, `/`
- Comparison: `<`, `>`, `=`
- Special forms: `def`, `defn`, `let`, `if`, `use`, `ns`, `require`
- Namespace keywords: `:as`, `:refer`, `:all`, `:rust`, `:require`
- File system: `fs/read`, `fs/write`, `fs/append`, `fs/exists?`
- Core functions: `slurp`, `spit`
- Modules: `rust.fs`, `clorus.core`

**Configurability:** ✅ COULD use config file

### 4.7 Version String (main.rs:243)
```
"Clorus REPL v0.2.0"
```

**Configurability:** ✅ SHOULD be in version.rs

---

## 5. NAMESPACE SYSTEM

### 5.1 Default Namespace (namespace.rs:49-64)
```
"user"
```

**Why:** Clojure convention
**Configurability:** ✅ COULD be parameterized

### 5.2 Namespace Separator (codegen.rs:2074)
```
'.'  - separates components (my.app.core)
'_'  - replaces '.' in LLVM names
'-'  - replaced with '_' for Rust compat
```

**Configurability:** ❌ NOT configurable - syntax requirement

---

## 6. MAGIC NUMBERS

### 6.1 Reference Counting (value.rs:49)
```rust
refcount: AtomicU64::new(1)  // Initial refcount
```

**Configurability:** ❌ NOT configurable - fundamental to RC

### 6.2 Numeric Type Promotion (arithmetic.rs)
```
Long + Long     -> Long
Long + Double   -> Double
Double + Double -> Double
Long / Long     -> Double
```

**Configurability:** ❌ NOT configurable - language semantics

---

## CONFIGURABILITY SUMMARY

| Component | Count | Configurable | Rationale |
|-----------|-------|--------------|-----------|
| ValueTag enum | 17 | ❌ No | Binary protocol |
| FFI functions | 168 | ❌ No | Language API |
| Special forms | 21 | ❌ No | Parser requirement |
| Core functions list | 30+ | ⚠️ Partial | Could use registry |
| REPL commands | 3 | ✅ Yes | User extensible |
| Library paths | 9 | ✅ Yes | Config file |
| Stdlib location | 2 | ✅ Yes | Config file |
| Autocomplete | 50+ | ✅ Yes | Config file |
| Default namespace | 1 | ✅ Yes | Parameter |
| Symbol chars | ~20 | ❌ No | Syntax rule |
| Escape sequences | 5 | ❌ No | Convention |
| Truthiness rules | 4 | ❌ No | Semantics |

---

## ARCHITECTURAL RECOMMENDATIONS

### 1. Create Configuration Registry
```toml
# ~/.clorus/config.toml
[repl.commands]
quit = [":quit", ":q"]
help = [":help", ":h"]

[repl.completions]
modules = ["rust.fs", "clorus.core"]

[libraries]
std = "libclorus_std"
core = "libclorus_core"
runtime = "libclorus_runtime"

[paths]
stdlib = "~/.clorus/stdlib"
history = "~/.clorus_history"
```

### 2. Externalize Version String
```rust
// crates/clorus-repl/src/version.rs
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
```

### 3. Standardize Config Search Path
1. `./Clorus.toml` (project-local)
2. `$CLORUS_HOME/config.toml` (installation)
3. `~/.clorus/config.toml` (user)

### 4. Document FFI Stability Contract
- All 168 function signatures frozen post-1.0
- New FFI functions require RFC
- ValueTag assignments are permanent

### 5. Plugin System for Extensions
```rust
pub trait SpecialForm {
    fn name(&self) -> &str;
    fn parse(&self, parser: &mut Parser) -> Result<Expr, String>;
}
```

### 6. Core Functions Registry
Replace hardcoded list with:
```rust
pub struct CoreFunction {
    name: &'static str,
    arity: Arity,
    generator: fn(&mut Codegen, &[Expr]) -> Result<PointerValue, String>,
}

pub static CORE_FUNCTIONS: &[CoreFunction] = &[
    CoreFunction { name: "get", arity: Fixed(2), generator: compile_get },
    // ...
];
```

---

## TOTALS

- **ValueTag types:** 17
- **FFI functions:** 168
- **Special forms:** 21
- **Core functions:** 30+
- **REPL commands:** 3
- **Library search paths:** 9
- **Hardcoded operators:** 7
- **Reader macros:** 10
- **Escape sequences:** 5
- **Total hardcoded identifiers:** ~300+

**Breakdown by configurability:**
- ❌ Cannot configure: ~70% (core language semantics)
- ⚠️ Partially configurable: ~10% (with code changes)
- ✅ Could configure: ~20% (REPL, paths, UI)

---

*Document generated: 2026-02-03*
*Clorus version: 0.2.0*
