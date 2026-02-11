/// Abstract Syntax Tree definitions for Clorus
/// Represents Clojure-like S-expressions

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Long integers: 42, -100, 1000000
    Long(i64),

    /// Double floating-point: 3.14, 1e6, -2.5
    Double(f64),

    /// Strings: "hello world"
    String(String),

    /// Symbols: +, defn, my-var (used for variable references and operators)
    Symbol(String),

    /// Keywords: :name, :type
    Keyword(String),

    /// Boolean values: true, false
    Bool(bool),

    /// Nil value
    Nil,

    /// Lists: (+ 1 2), (defn foo [] 42)
    List(Vec<Expr>),

    /// Vectors: [1 2 3]
    Vector(Vec<Expr>),

    /// Maps: {:name "Alice" :age 30}
    Map(Vec<(Expr, Expr)>),

    /// Sets: #{1 2 3}
    Set(Vec<Expr>),

    /// Let bindings: (let [x 10 y 20] (+ x y))
    /// bindings are pairs of (pattern, value), body is the expression to evaluate
    /// Supports destructuring: (let [[a b c] [1 2 3]] ...)
    Let {
        bindings: Vec<(Pattern, Box<Expr>)>,
        body: Box<Expr>,
    },

    /// Letfn expression: (letfn [(f [x] ...) (g [y] ...)] body)
    /// Defines local functions with mutual recursion support
    /// All functions can reference each other in their bodies
    /// bindings are tuples of (name, params, rest_param, body)
    Letfn {
        bindings: Vec<(String, Vec<Pattern>, Option<String>, Box<Expr>)>,
        body: Box<Expr>,
    },

    /// Global definition: (def x 10) or (def ^:dynamic *x* 10)
    /// Supports metadata: ^:dynamic, ^:private, ^{:doc "..."}, etc.
    Def {
        name: String,
        value: Box<Expr>,
        /// Metadata map - e.g., {:dynamic true, :doc "Output stream"}
        /// Parsed from ^:dynamic or ^{:key val}
        metadata: Option<Vec<(Expr, Expr)>>,
    },

    /// Function definition: (defn add [x y] (+ x y))
    /// Supports variadic functions: (defn sum [x & rest] ...)
    /// Supports destructuring: (defn process [[a b] c] ...)
    Defn {
        name: String,
        params: Vec<Pattern>,
        rest_param: Option<String>, // Parameter after &, collects remaining args
        body: Box<Expr>,
    },

    /// Multi-arity function definition: (defn greet ([] "Hi") ([name] (str "Hi " name)))
    /// Each arity is (params, rest_param, body)
    DefnMulti {
        name: String,
        arities: Vec<FunctionArity>,
    },

    /// Forward declaration: (declare foo bar baz)
    /// Declares function names that will be defined later, enabling forward references
    /// Example: (declare helper) (defn main [] (helper 42)) (defn helper [x] (+ x 1))
    Declare {
        names: Vec<String>,
    },

    /// Anonymous function: (fn [x y] (+ x y))
    /// Creates a lambda that can be stored in variables or passed as argument
    /// Supports variadic functions: (fn [x & rest] ...)
    /// Supports destructuring: (fn [[a b] c] ...)
    Fn {
        params: Vec<Pattern>,
        rest_param: Option<String>, // Parameter after &, collects remaining args
        body: Box<Expr>,
    },

    /// Multi-arity anonymous function: (fn ([] 0) ([x] x) ([x y] (+ x y)))
    FnMulti {
        arities: Vec<FunctionArity>,
    },

    /// Function call: (add 1 2)
    /// This is distinguished from List during semantic analysis
    Call {
        func: String,
        args: Vec<Expr>,
    },

    /// If expression: (if condition then-expr else-expr)
    If {
        condition: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Box<Expr>,
    },

    /// Do expression: (do expr1 expr2 expr3 ...)
    /// Evaluates multiple expressions in sequence, returns value of last expression
    Do {
        exprs: Vec<Expr>,
    },

    /// Quote: (quote x) or 'x
    /// Prevents evaluation and returns the expression as data
    /// Example: 'x returns the symbol x, '(1 2 3) returns a list [1 2 3]
    Quote {
        expr: Box<Expr>,
    },

    /// Syntax-quote: `x or `(...)
    /// Like quote but allows unquoting and resolves symbols to namespaces
    /// Example: `(+ 1 ~x) with x=2 => (+ 1 2)
    SyntaxQuote {
        expr: Box<Expr>,
    },

    /// Unquote: ~x (only valid inside syntax-quote)
    /// Evaluates the expression and splices the result into the syntax-quote
    /// Example: `(+ 1 ~(* 2 3)) => (+ 1 6)
    Unquote {
        expr: Box<Expr>,
    },

    /// Unquote-splicing: ~@xs (only valid inside syntax-quote)
    /// Evaluates to a sequence and splices elements into the parent
    /// Example: `(1 ~@[2 3] 4) => (1 2 3 4)
    UnquoteSplicing {
        expr: Box<Expr>,
    },

    /// Deref: @my-atom or (deref my-atom)
    /// Dereferences an atom/ref to get its current value
    /// Example: @counter => current value of counter atom
    Deref {
        expr: Box<Expr>,
    },

    /// Var reference: (var x) or #'x
    /// Returns the Var object itself (not its value)
    /// Example: #'*out* => #<Var *out*>, not the value
    /// Used for alter-var-root, var metadata access
    Var {
        name: String,
    },

    /// Binding expression: (binding [*var1* val1 *var2* val2] body)
    /// Temporarily rebinds dynamic vars for the duration of body
    /// Thread-local, automatically restored on exit
    Binding {
        bindings: Vec<(String, Box<Expr>)>,  // var name -> new value
        body: Box<Expr>,
    },

    /// Use/import statement: (use rust.fs) or (use rust.fs [read write])
    Use {
        module: String,        // e.g., "rust.fs"
        imports: Vec<String>,  // Specific imports, empty = import all
    },

    /// Namespace declaration: (ns my.app.core (:require [lib :as l]) (:rust [egui :as gui]))
    Ns {
        name: String,                  // e.g., "my.app.core"
        requires: Vec<RequireSpec>,    // (:require clauses)
        rust_imports: Vec<RustImport>, // (:rust clauses)
    },

    /// Require statement: (require '[my.lib :as lib :refer [func1 func2]])
    Require {
        specs: Vec<RequireSpec>,
    },

    /// Loop expression: (loop [x 0 y 10] (if (< x y) (recur (+ x 1) y) x))
    /// Creates a recursion point with bindings, can be jumped to with recur
    /// Supports destructuring: (loop [[a b] [1 2]] ...)
    Loop {
        bindings: Vec<(Pattern, Box<Expr>)>,
        body: Box<Expr>,
    },

    /// Dosync expression: (dosync (alter ref1 inc) (ref-set ref2 val))
    /// Transaction block for coordinated updates to multiple refs
    /// Uses STM (Software Transactional Memory) with MVCC
    /// Retries on conflict up to a limit
    Dosync {
        exprs: Vec<Expr>,
    },

    /// Recur expression: (recur new-x new-y)
    /// Jumps back to the nearest loop (or function start) with new binding values
    /// Must be in tail position
    Recur {
        args: Vec<Expr>,
    },

    /// Macro definition: (defmacro when [test & body] `(if ~test (do ~@body) nil))
    /// Defines a compile-time transformation that operates on code
    Defmacro {
        name: String,
        params: Vec<String>,
        rest_param: Option<String>,
        body: Box<Expr>,
    },

    /// Record type definition: (defrecord Person [name age email])
    /// Creates a named data structure with fields
    /// Generates constructor function ->RecordName and map->RecordName
    /// Defrecord: Immutable record type with fields
    /// (defrecord Person [name age email])
    /// Creates a constructor: (->Person "Alice" 30 "alice@example.com")
    /// Supports keyword field access: (:name person-instance)
    /// Supports assoc/dissoc operations
    Defrecord {
        name: String,        // Record type name, e.g., "Person"
        fields: Vec<String>, // Field names, e.g., ["name", "age", "email"]
    },

    /// Deftype: Type with fields and inline protocol implementations
    /// (deftype Button [props state]
    ///   IComponent
    ///   (render [this ctx] ...)
    ///   (layout [this bounds] ...))
    /// Like defrecord but allows implementing protocols inline
    Deftype {
        name: String,                           // Type name, e.g., "Button"
        fields: Vec<String>,                    // Field names, e.g., ["props", "state"]
        protocols: Vec<(String, Vec<ProtocolMethodImpl>)>,  // (protocol_name, methods)
    },

    /// Protocol definition: (defprotocol Drawable (draw [this]) (bounds [this]))
    /// Defines a set of polymorphic methods that types can implement
    /// Methods are abstract - only signatures, no implementations
    Defprotocol {
        name: String,                      // Protocol name, e.g., "Drawable"
        methods: Vec<ProtocolMethod>,      // Method signatures
    },

    /// Extend a type to implement a protocol
    /// (extend-type Point Drawable (draw [this] ...) (bounds [this] ...))
    ExtendType {
        type_name: String,                 // Type being extended, e.g., "Point"
        protocol_name: String,             // Protocol being implemented, e.g., "Drawable"
        methods: Vec<ProtocolMethodImpl>,  // Method implementations
    },

    /// Multimethod definition: (defmulti area :type)
    /// Defines a polymorphic function that dispatches based on a dispatch function
    /// The dispatch function is called with the arguments to determine which method to invoke
    Defmulti {
        name: String,          // Multimethod name, e.g., "area"
        dispatch_fn: Box<Expr>, // Dispatch function, e.g., :type or (fn [x] (:type x))
    },

    /// Method implementation for a multimethod
    /// (defmethod area :circle [shape] (* 3.14 (* (:radius shape) (:radius shape))))
    Defmethod {
        name: String,              // Multimethod name, e.g., "area"
        dispatch_value: Box<Expr>, // Dispatch value, e.g., :circle
        params: Vec<Pattern>,      // Parameters
        body: Box<Expr>,           // Implementation body
    },

    /// Try/Catch/Finally: (try body (catch type var handler) (finally cleanup))
    /// Exception handling with optional catch clauses and finally block
    Try {
        body: Box<Expr>,
        catch_clauses: Vec<CatchClause>,
        finally_block: Option<Box<Expr>>,
    },

    /// Throw: (throw exception-value)
    /// Throws an exception/error
    Throw {
        expr: Box<Expr>,
    },
}

/// Specification for requiring a module
#[derive(Debug, Clone, PartialEq)]
pub struct RequireSpec {
    pub module: String,            // e.g., "my.lib.math"
    pub alias: Option<String>,     // :as math
    pub refer: Vec<String>,        // :refer [sin cos]
    pub refer_all: bool,           // :refer :all
}

/// Specification for importing a Rust FFI library
#[derive(Debug, Clone, PartialEq)]
pub struct RustImport {
    pub library: String,           // e.g., "egui-hello"
    pub alias: Option<String>,     // :as gui
}

/// Single arity of a multi-arity function
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionArity {
    pub params: Vec<Pattern>,
    pub rest_param: Option<String>,
    pub body: Box<Expr>,
}

/// Single catch clause in a try expression
#[derive(Debug, Clone, PartialEq)]
pub struct CatchClause {
    pub exception_type: Option<String>, // None = catch all
    pub binding: String,                 // Variable name to bind exception to
    pub handler: Box<Expr>,              // Handler expression
}

/// Protocol method signature
#[derive(Debug, Clone, PartialEq)]
pub struct ProtocolMethod {
    pub name: String,              // Method name, e.g., "draw"
    pub params: Vec<String>,       // Parameter names (first is always 'this')
    pub docstring: Option<String>, // Optional documentation
}

/// Protocol method implementation
#[derive(Debug, Clone, PartialEq)]
pub struct ProtocolMethodImpl {
    pub name: String,              // Method name, e.g., "draw"
    pub params: Vec<Pattern>,      // Parameters with destructuring support
    pub body: Box<Expr>,           // Implementation body
}

/// Destructuring patterns for let bindings and function parameters
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// Simple binding: x
    Symbol(String),

    /// Vector destructuring: [a b c] or [a b & rest] or [a b :as all]
    Vector {
        elements: Vec<Pattern>,
        rest: Option<String>, // & rest parameter
        as_binding: Option<String>, // :as binding for whole collection
    },

    /// Map destructuring: {:keys [x y]} or {x :x, y :y} or {:keys [x y] :or {x 0}}
    Map {
        bindings: Vec<(MapPatternKey, Pattern)>,
        defaults: Option<Vec<(String, Box<Expr>)>>, // :or defaults map
    },

    /// Ignore binding: _
    Ignore,
}

/// Key in a map destructuring pattern
#[derive(Debug, Clone, PartialEq)]
pub enum MapPatternKey {
    /// Keyword key: :name (from :keys)
    Keyword(String),
    /// Symbol used as both key and binding: x => {:x x}
    Symbol(String),
    /// String key: "name" (from :strs)
    Str(String),
    /// Symbol key: 'x (from :syms)
    Sym(String),
}

impl Expr {
    pub fn is_atom(&self) -> bool {
        matches!(
            self,
            Expr::Long(_)
                | Expr::Double(_)
                | Expr::String(_)
                | Expr::Symbol(_)
                | Expr::Keyword(_)
                | Expr::Bool(_)
                | Expr::Nil
        )
    }
}
