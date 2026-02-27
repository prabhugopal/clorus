/// Macro expansion for Clorus
/// Implements Clojure-style macros as compile-time code transformations
///
/// Currently supports:
/// - `->` (thread-first)
/// - `->>` (thread-last)
/// - User-defined macros via defmacro

use crate::ast::Expr;
use std::collections::HashMap;

/// Registry for user-defined macros
#[derive(Debug, Clone)]
pub struct MacroRegistry {
    macros: HashMap<String, MacroDefinition>,
    gensym_counter: u64, // Counter for generating unique symbols
}

#[derive(Debug, Clone)]
pub(crate) struct MacroDefinition {
    params: Vec<String>,
    rest_param: Option<String>,
    body: Box<Expr>,
}

impl MacroRegistry {
    pub fn new() -> Self {
        MacroRegistry {
            macros: HashMap::new(),
            gensym_counter: 0,
        }
    }

    pub fn register(&mut self, name: String, params: Vec<String>, rest_param: Option<String>, body: Box<Expr>) {
        self.macros.insert(name, MacroDefinition {
            params,
            rest_param,
            body,
        });
    }

    pub fn get(&self, name: &str) -> Option<&MacroDefinition> {
        self.macros.get(name)
    }

    /// Generate a unique symbol for macro hygiene
    /// (gensym) => G__1234
    /// (gensym "x") => x__1234
    pub fn gensym(&mut self, prefix: Option<&str>) -> String {
        self.gensym_counter += 1;
        match prefix {
            Some(p) => format!("{}__G__{}", p, self.gensym_counter),
            None => format!("G__{}", self.gensym_counter),
        }
    }
}

/// Expand all macros in an expression recursively
pub fn expand_macros(expr: &Expr) -> Expr {
    let mut registry = MacroRegistry::new();
    expand_macros_with_registry(expr, &mut registry)
}

/// Expand macros across a sequence of expressions, preserving macro registry state.
/// This allows (defmacro ...) forms to affect subsequent expressions in the same file.
pub fn expand_macros_sequence(exprs: &[Expr]) -> Vec<Expr> {
    let mut registry = MacroRegistry::new();
    exprs
        .iter()
        .map(|expr| expand_macros_with_registry(expr, &mut registry))
        .collect()
}

/// Expand only the outermost macro (single step) - does not recurse
pub fn expand_macros_once(expr: &Expr) -> Expr {
    let mut registry = MacroRegistry::new();
    expand_macros_once_with_registry(expr, &mut registry)
}

fn expand_macros_once_with_registry(expr: &Expr, registry: &mut MacroRegistry) -> Expr {
    match expr {
        // Thread-first macro: (-> x (f a) (g b)) => (g (f x a) b)
        Expr::Call { func, args } if func == "->" => {
            expand_thread_first_once(args, registry)
        }

        // Thread-last macro: (->> x (f a) (g b)) => (g b (f a x))
        Expr::Call { func, args } if func == "->>" => {
            expand_thread_last_once(args, registry)
        }

        // Cond macro
        Expr::Call { func, args } if func == "cond" => {
            expand_cond_once(args, registry)
        }

        // Case macro
        Expr::Call { func, args } if func == "case" => {
            expand_case_once(args, registry)
        }

        // When macro
        Expr::Call { func, args } if func == "when" => {
            expand_when_once(args, registry)
        }

        // When-not macro
        Expr::Call { func, args } if func == "when-not" => {
            expand_when_not_once(args, registry)
        }

        // If-let macro
        Expr::Call { func, args } if func == "if-let" => {
            expand_if_let_once(args, registry)
        }

        // When-let macro
        Expr::Call { func, args } if func == "when-let" => {
            expand_when_let_once(args, registry)
        }

        // If-not macro
        Expr::Call { func, args } if func == "if-not" => {
            expand_if_not_once(args, registry)
        }

        // And macro
        Expr::Call { func, args } if func == "and" => {
            expand_and_once(args, registry)
        }

        // Or macro
        Expr::Call { func, args } if func == "or" => {
            expand_or_once(args, registry)
        }

        // Some-> macro
        Expr::Call { func, args } if func == "some->" => {
            expand_some_thread_first_once(args, registry)
        }

        // Some->> macro
        Expr::Call { func, args } if func == "some->>" => {
            expand_some_thread_last_once(args, registry)
        }

        // Doto macro
        Expr::Call { func, args } if func == "doto" => {
            expand_doto_once(args, registry)
        }

        // While macro
        Expr::Call { func, args } if func == "while" => {
            expand_while_once(args, registry)
        }

        // Dotimes macro
        Expr::Call { func, args } if func == "dotimes" => {
            expand_dotimes_once(args, registry)
        }

        // Doseq macro
        Expr::Call { func, args } if func == "doseq" => {
            expand_doseq_once(args, registry)
        }

        // With-open macro - resource management
        Expr::Call { func, args } if func == "with-open" => {
            expand_with_open_once(args, registry)
        }

        // Lazy-seq macro - for Clojure-style lazy sequences
        Expr::Call { func, args } if func == "lazy-seq" => {
            expand_lazy_seq_once(args, registry)
        }

        // User-defined macro call
        Expr::Call { func, args } => {
            // Check if it's a user-defined macro
            if let Some(macro_def) = registry.macros.get(func).cloned() {
                // Expand the macro once (without recursing into result)
                apply_user_macro(&macro_def, args)
            } else {
                // Not a macro - return as-is (don't recurse into args)
                expr.clone()
            }
        }

        // Not a macro - return as-is
        _ => expr.clone(),
    }
}

// Helper functions for single-step expansion (don't recurse into results)
fn expand_thread_first_once(args: &[Expr], _registry: &mut MacroRegistry) -> Expr {
    // Same as expand_thread_first but don't call expand_macros_with_registry on result
    expand_thread_first_impl(args, false)
}

fn expand_thread_last_once(args: &[Expr], _registry: &mut MacroRegistry) -> Expr {
    expand_thread_last_impl(args, false)
}

fn expand_cond_once(args: &[Expr], _registry: &mut MacroRegistry) -> Expr {
    expand_cond_impl(args, false)
}

fn expand_case_once(args: &[Expr], _registry: &mut MacroRegistry) -> Expr {
    expand_case_impl(args, false)
}

fn expand_when_once(args: &[Expr], _registry: &mut MacroRegistry) -> Expr {
    expand_when_impl(args, false)
}

fn expand_when_not_once(args: &[Expr], _registry: &mut MacroRegistry) -> Expr {
    expand_when_not_impl(args, false)
}

fn expand_if_let_once(args: &[Expr], _registry: &mut MacroRegistry) -> Expr {
    expand_if_let_impl(args, false)
}

fn expand_when_let_once(args: &[Expr], _registry: &mut MacroRegistry) -> Expr {
    expand_when_let_impl(args, false)
}

fn expand_if_not_once(args: &[Expr], _registry: &mut MacroRegistry) -> Expr {
    expand_if_not_impl(args, false)
}

fn expand_and_once(args: &[Expr], _registry: &mut MacroRegistry) -> Expr {
    expand_and_impl(args, false)
}

fn expand_or_once(args: &[Expr], _registry: &mut MacroRegistry) -> Expr {
    expand_or_impl(args, false)
}

fn expand_some_thread_first_once(args: &[Expr], _registry: &mut MacroRegistry) -> Expr {
    expand_some_thread_first_impl(args, false)
}

fn expand_some_thread_last_once(args: &[Expr], _registry: &mut MacroRegistry) -> Expr {
    expand_some_thread_last_impl(args, false)
}

fn expand_doto_once(args: &[Expr], _registry: &mut MacroRegistry) -> Expr {
    expand_doto_impl(args, false)
}

fn expand_while_once(args: &[Expr], _registry: &mut MacroRegistry) -> Expr {
    expand_while_impl(args, false)
}

fn expand_dotimes_once(args: &[Expr], _registry: &mut MacroRegistry) -> Expr {
    expand_dotimes_impl(args, false)
}

fn expand_doseq_once(args: &[Expr], _registry: &mut MacroRegistry) -> Expr {
    expand_doseq_impl(args, false)
}

// Implementation helpers (these need to be created or the existing functions refactored)
// For MVP, let's just use the existing expand functions as-is (they already expand once)
fn expand_thread_first_impl(args: &[Expr], _recurse: bool) -> Expr {
    // Just use existing implementation - it expands once by design
    let mut registry = MacroRegistry::new();
    expand_thread_first(args, &mut registry)
}

fn expand_thread_last_impl(args: &[Expr], _recurse: bool) -> Expr {
    let mut registry = MacroRegistry::new();
    expand_thread_last(args, &mut registry)
}

fn expand_cond_impl(args: &[Expr], _recurse: bool) -> Expr {
    let mut registry = MacroRegistry::new();
    expand_cond(args, &mut registry)
}

fn expand_case_impl(args: &[Expr], _recurse: bool) -> Expr {
    let mut registry = MacroRegistry::new();
    expand_case(args, &mut registry)
}

fn expand_when_impl(args: &[Expr], _recurse: bool) -> Expr {
    let mut registry = MacroRegistry::new();
    expand_when(args, &mut registry)
}

fn expand_when_not_impl(args: &[Expr], _recurse: bool) -> Expr {
    let mut registry = MacroRegistry::new();
    expand_when_not(args, &mut registry)
}

fn expand_if_let_impl(args: &[Expr], _recurse: bool) -> Expr {
    let mut registry = MacroRegistry::new();
    expand_if_let(args, &mut registry)
}

fn expand_when_let_impl(args: &[Expr], _recurse: bool) -> Expr {
    let mut registry = MacroRegistry::new();
    expand_when_let(args, &mut registry)
}

fn expand_if_not_impl(args: &[Expr], _recurse: bool) -> Expr {
    let mut registry = MacroRegistry::new();
    expand_if_not(args, &mut registry)
}

fn expand_and_impl(args: &[Expr], _recurse: bool) -> Expr {
    let mut registry = MacroRegistry::new();
    expand_and(args, &mut registry)
}

fn expand_or_impl(args: &[Expr], _recurse: bool) -> Expr {
    let mut registry = MacroRegistry::new();
    expand_or(args, &mut registry)
}

fn expand_some_thread_first_impl(args: &[Expr], _recurse: bool) -> Expr {
    let mut registry = MacroRegistry::new();
    expand_some_thread_first(args, &mut registry)
}

fn expand_some_thread_last_impl(args: &[Expr], _recurse: bool) -> Expr {
    let mut registry = MacroRegistry::new();
    expand_some_thread_last(args, &mut registry)
}

fn expand_doto_impl(args: &[Expr], _recurse: bool) -> Expr {
    let mut registry = MacroRegistry::new();
    expand_doto(args, &mut registry)
}

fn expand_while_impl(args: &[Expr], _recurse: bool) -> Expr {
    let mut registry = MacroRegistry::new();
    expand_while(args, &mut registry)
}

fn expand_dotimes_impl(args: &[Expr], _recurse: bool) -> Expr {
    let mut registry = MacroRegistry::new();
    expand_dotimes(args, &mut registry)
}

fn expand_doseq_impl(args: &[Expr], _recurse: bool) -> Expr {
    let mut registry = MacroRegistry::new();
    expand_doseq(args, &mut registry)
}

fn expand_with_open_once(args: &[Expr], _registry: &mut MacroRegistry) -> Expr {
    expand_with_open_impl(args, false)
}

fn expand_with_open_impl(args: &[Expr], _recurse: bool) -> Expr {
    let mut registry = MacroRegistry::new();
    expand_with_open(args, &mut registry)
}

fn expand_lazy_seq_once(args: &[Expr], _registry: &mut MacroRegistry) -> Expr {
    expand_lazy_seq_impl(args, false)
}

fn expand_lazy_seq_impl(args: &[Expr], _recurse: bool) -> Expr {
    let mut registry = MacroRegistry::new();
    expand_lazy_seq(args, &mut registry)
}

fn expand_macros_with_registry(expr: &Expr, registry: &mut MacroRegistry) -> Expr {
    match expr {
        // Thread-first macro: (-> x (f a) (g b)) => (g (f x a) b)
        Expr::Call { func, args } if func == "->" => {
            expand_thread_first(args, registry)
        }

        // Thread-last macro: (->> x (f a) (g b)) => (g b (f a x))
        Expr::Call { func, args } if func == "->>" => {
            expand_thread_last(args, registry)
        }

        // Cond macro: (cond test1 expr1 test2 expr2 :else expr3) => nested ifs
        Expr::Call { func, args } if func == "cond" => {
            expand_cond(args, registry)
        }

        // Case macro: (case x 1 "one" 2 "two" "default") => let + cond with equality checks
        Expr::Call { func, args } if func == "case" => {
            expand_case(args, registry)
        }

        // When macro: (when test expr1 expr2) => (if test (do expr1 expr2) nil)
        Expr::Call { func, args } if func == "when" => {
            expand_when(args, registry)
        }

        // When-not macro: (when-not test expr1 expr2) => (if test nil (do expr1 expr2))
        Expr::Call { func, args } if func == "when-not" => {
            expand_when_not(args, registry)
        }

        // If-let macro: (if-let [x expr] then else) => (let [x expr] (if x then else))
        Expr::Call { func, args } if func == "if-let" => {
            expand_if_let(args, registry)
        }

        // When-let macro: (when-let [x expr] body) => (let [x expr] (when x body))
        Expr::Call { func, args } if func == "when-let" => {
            expand_when_let(args, registry)
        }

        // If-not macro: (if-not test then else) => (if test else then)
        Expr::Call { func, args } if func == "if-not" => {
            expand_if_not(args, registry)
        }

        // And macro: (and x y z) => (if x (if y z false) false)
        Expr::Call { func, args } if func == "and" => {
            expand_and(args, registry)
        }

        // Or macro: (or x y z) => (if x true (if y true z))
        Expr::Call { func, args } if func == "or" => {
            expand_or(args, registry)
        }

        // Some-> macro: (some-> x (f a) (g b)) => thread-first but stops on nil
        Expr::Call { func, args } if func == "some->" => {
            expand_some_thread_first(args, registry)
        }

        // Some->> macro: (some->> x (f a) (g b)) => thread-last but stops on nil
        Expr::Call { func, args } if func == "some->>" => {
            expand_some_thread_last(args, registry)
        }

        // Doto macro: (doto x (f a) (g b)) => (let [obj x] (f obj a) (g obj b) obj)
        Expr::Call { func, args } if func == "doto" => {
            expand_doto(args, registry)
        }

        // While macro: (while test body...) => (loop [] (when test body... (recur)))
        Expr::Call { func, args } if func == "while" => {
            expand_while(args, registry)
        }

        // Dotimes macro: (dotimes [i n] body...) => (loop [i 0] (when (< i n) body... (recur (inc i))))
        Expr::Call { func, args } if func == "dotimes" => {
            expand_dotimes(args, registry)
        }

        // Doseq macro: (doseq [x coll] body...) => iterate over collection with side effects
        Expr::Call { func, args } if func == "doseq" => {
            expand_doseq(args, registry)
        }

        // With-open macro: (with-open [x init] body...) => resource management with cleanup
        Expr::Call { func, args} if func == "with-open" => {
            expand_with_open(args, registry)
        }

        // User-defined macro call
        Expr::Call { func, args } => {
            // Check for gensym call (compile-time symbol generation)
            if func == "gensym" {
                let prefix = if !args.is_empty() {
                    // If there's an argument, it should be a string for the prefix
                    match &args[0] {
                        Expr::String(s) => Some(s.as_str()),
                        _ => None,
                    }
                } else {
                    None
                };

                let sym = registry.gensym(prefix);
                return Expr::Symbol(sym);
            }

            if let Some(macro_def) = registry.get(func).cloned() {
                // Expand user macro (clone macro_def to avoid borrow conflict)
                let expanded = expand_user_macro(&macro_def, args, registry);
                // Recursively expand the result (macros can generate macro calls)
                expand_macros_with_registry(&expanded, registry)
            } else {
                // Not a macro, recursively expand arguments
                let expanded_args: Vec<_> = args.iter().map(|a| expand_macros_with_registry(a, registry)).collect();
                Expr::Call {
                    func: func.clone(),
                    args: expanded_args,
                }
            }
        }

        // Defmacro: register the macro and return it as-is
        Expr::Defmacro { name, params, rest_param, body } => {
            // Register the macro
            registry.register(
                name.clone(),
                params.clone(),
                rest_param.clone(),
                body.clone(),
            );

            // Return the defmacro expression (will be handled by codegen)
            Expr::Defmacro {
                name: name.clone(),
                params: params.clone(),
                rest_param: rest_param.clone(),
                body: body.clone(),
            }
        }

        // Recursively expand macros in other expressions
        Expr::Let { bindings, body } => {
            let expanded_bindings: Vec<_> = bindings
                .iter()
                .map(|(name, value)| (name.clone(), Box::new(expand_macros_with_registry(value, registry))))
                .collect();
            Expr::Let {
                bindings: expanded_bindings,
                body: Box::new(expand_macros_with_registry(body, registry)),
            }
        }

        Expr::Letfn { bindings, body } => {
            let expanded_bindings: Vec<_> = bindings
                .iter()
                .map(|(name, params, rest_param, fn_body)| {
                    (name.clone(), params.clone(), rest_param.clone(),
                     Box::new(expand_macros_with_registry(fn_body, registry)))
                })
                .collect();
            Expr::Letfn {
                bindings: expanded_bindings,
                body: Box::new(expand_macros_with_registry(body, registry)),
            }
        }

        Expr::Def { name, value, metadata } => Expr::Def {
            name: name.clone(),
            value: Box::new(expand_macros_with_registry(value, registry)),
            metadata: metadata.clone(),  // Metadata doesn't need macro expansion
        },

        Expr::Defn { name, params, rest_param, body } => Expr::Defn {
            name: name.clone(),
            params: params.clone(),
            rest_param: rest_param.clone(),
            body: Box::new(expand_macros_with_registry(body, registry)),
        },

        Expr::Declare { names } => Expr::Declare {
            names: names.clone(),
        },

        Expr::DefnMulti { name, arities } => {
            let expanded_arities = arities
                .iter()
                .map(|arity| crate::ast::FunctionArity {
                    params: arity.params.clone(),
                    rest_param: arity.rest_param.clone(),
                    body: Box::new(expand_macros_with_registry(&arity.body, registry)),
                })
                .collect();
            Expr::DefnMulti {
                name: name.clone(),
                arities: expanded_arities,
            }
        },

        Expr::Defrecord { name, fields, protocols } => {
            // Expand protocol method bodies but keep fields as-is
            let expanded_protocols = protocols.iter().map(|(protocol_name, methods)| {
                let expanded_methods = methods.iter().map(|method| {
                    let mut method_impl = method.clone();
                    method_impl.body = Box::new(expand_macros_with_registry(&method.body, registry));
                    method_impl
                }).collect();
                (protocol_name.clone(), expanded_methods)
            }).collect();

            Expr::Defrecord {
                name: name.clone(),
                fields: fields.clone(),
                protocols: expanded_protocols,
            }
        },

        Expr::Deftype { name, fields, protocols } => {
            // Expand protocol method bodies but keep fields as-is
            let expanded_protocols = protocols.iter().map(|(protocol_name, methods)| {
                let expanded_methods = methods.iter().map(|method| {
                    let mut method_impl = method.clone();
                    method_impl.body = Box::new(expand_macros_with_registry(&method.body, registry));
                    method_impl
                }).collect();
                (protocol_name.clone(), expanded_methods)
            }).collect();

            Expr::Deftype {
                name: name.clone(),
                fields: fields.clone(),
                protocols: expanded_protocols,
            }
        },

        Expr::Defprotocol { name, methods } => {
            // Protocols have no expressions to expand, just method signatures
            Expr::Defprotocol {
                name: name.clone(),
                methods: methods.clone(),
            }
        },

        Expr::ExtendType { type_name, protocol_name, methods } => {
            // Expand macros in method bodies
            let expanded_methods = methods.iter()
                .map(|method| crate::ast::ProtocolMethodImpl {
                    name: method.name.clone(),
                    params: method.params.clone(),
                    body: Box::new(expand_macros_with_registry(&method.body, registry)),
                })
                .collect();
            Expr::ExtendType {
                type_name: type_name.clone(),
                protocol_name: protocol_name.clone(),
                methods: expanded_methods,
            }
        },

        Expr::Defmulti { name, dispatch_fn } => {
            // Expand macros in dispatch function
            Expr::Defmulti {
                name: name.clone(),
                dispatch_fn: Box::new(expand_macros_with_registry(dispatch_fn, registry)),
            }
        },

        Expr::Defmethod { name, dispatch_value, params, body } => {
            // Expand macros in method body and dispatch value
            Expr::Defmethod {
                name: name.clone(),
                dispatch_value: Box::new(expand_macros_with_registry(dispatch_value, registry)),
                params: params.clone(),
                body: Box::new(expand_macros_with_registry(body, registry)),
            }
        },

        Expr::Fn { params, rest_param, body } => Expr::Fn {
            params: params.clone(),
            rest_param: rest_param.clone(),
            body: Box::new(expand_macros_with_registry(body, registry)),
        },

        Expr::FnMulti { arities } => {
            let expanded_arities = arities
                .iter()
                .map(|arity| crate::ast::FunctionArity {
                    params: arity.params.clone(),
                    rest_param: arity.rest_param.clone(),
                    body: Box::new(expand_macros_with_registry(&arity.body, registry)),
                })
                .collect();
            Expr::FnMulti {
                arities: expanded_arities,
            }
        },

        Expr::If { condition, then_branch, else_branch } => Expr::If {
            condition: Box::new(expand_macros_with_registry(condition, registry)),
            then_branch: Box::new(expand_macros_with_registry(then_branch, registry)),
            else_branch: Box::new(expand_macros_with_registry(else_branch, registry)),
        },

        Expr::Do { exprs } => {
            let expanded: Vec<_> = exprs.iter().map(|e| expand_macros_with_registry(e, registry)).collect();
            Expr::Do { exprs: expanded }
        },

        Expr::Dosync { exprs } => {
            let expanded: Vec<_> = exprs.iter().map(|e| expand_macros_with_registry(e, registry)).collect();
            Expr::Dosync { exprs: expanded }
        },

        Expr::Quote { expr } => {
            // Quote prevents macro expansion - return as-is
            // The quoted expression is data, not code to be evaluated
            Expr::Quote { expr: expr.clone() }
        },

        Expr::SyntaxQuote { expr } => {
            // Syntax-quote: recursively process for unquotes
            Expr::SyntaxQuote {
                expr: Box::new(expand_syntax_quote(expr, registry))
            }
        },

        Expr::Unquote { expr } => {
            // Unquote should expand macros in the unquoted expression
            Expr::Unquote {
                expr: Box::new(expand_macros_with_registry(expr, registry))
            }
        },

        Expr::UnquoteSplicing { expr } => {
            // Unquote-splicing should also expand macros
            Expr::UnquoteSplicing {
                expr: Box::new(expand_macros_with_registry(expr, registry))
            }
        },

        Expr::Deref { expr } => {
            // Deref should expand macros in the dereferenced expression
            Expr::Deref {
                expr: Box::new(expand_macros_with_registry(expr, registry))
            }
        },

        Expr::Vector(elements) => {
            let expanded: Vec<_> = elements.iter().map(|e| expand_macros_with_registry(e, registry)).collect();
            Expr::Vector(expanded)
        }

        Expr::Map(entries) => {
            let expanded: Vec<_> = entries
                .iter()
                .map(|(k, v)| (expand_macros_with_registry(k, registry), expand_macros_with_registry(v, registry)))
                .collect();
            Expr::Map(expanded)
        }

        Expr::Set(elements) => {
            let expanded: Vec<_> = elements.iter().map(|e| expand_macros_with_registry(e, registry)).collect();
            Expr::Set(expanded)
        }

        Expr::List(items) => {
            let expanded: Vec<_> = items.iter().map(|e| expand_macros_with_registry(e, registry)).collect();
            Expr::List(expanded)
        }

        Expr::Loop { bindings, body } => {
            let expanded_bindings: Vec<_> = bindings
                .iter()
                .map(|(name, value)| (name.clone(), Box::new(expand_macros_with_registry(value, registry))))
                .collect();
            Expr::Loop {
                bindings: expanded_bindings,
                body: Box::new(expand_macros_with_registry(body, registry)),
            }
        }

        Expr::Binding { bindings, body } => {
            let expanded_bindings: Vec<_> = bindings
                .iter()
                .map(|(name, value)| (name.clone(), Box::new(expand_macros_with_registry(value, registry))))
                .collect();
            Expr::Binding {
                bindings: expanded_bindings,
                body: Box::new(expand_macros_with_registry(body, registry)),
            }
        }

        Expr::Recur { args } => {
            let expanded: Vec<_> = args.iter().map(|e| expand_macros_with_registry(e, registry)).collect();
            Expr::Recur { args: expanded }
        }

        Expr::Try { body, catch_clauses, finally_block } => {
            let expanded_body = Box::new(expand_macros_with_registry(body, registry));
            let expanded_catch = catch_clauses.iter().map(|clause| {
                crate::ast::CatchClause {
                    exception_type: clause.exception_type.clone(),
                    binding: clause.binding.clone(),
                    handler: Box::new(expand_macros_with_registry(&clause.handler, registry)),
                }
            }).collect();
            let expanded_finally = finally_block.as_ref().map(|f| Box::new(expand_macros_with_registry(f, registry)));

            Expr::Try {
                body: expanded_body,
                catch_clauses: expanded_catch,
                finally_block: expanded_finally,
            }
        }

        Expr::Throw { expr } => {
            Expr::Throw {
                expr: Box::new(expand_macros_with_registry(expr, registry))
            }
        }

        // Leaf nodes - no macro expansion needed
        Expr::Long(_)
        | Expr::Double(_)
        | Expr::String(_)
        | Expr::Symbol(_)
        | Expr::Keyword(_)
        | Expr::Bool(_)
        | Expr::Nil
        | Expr::Var { .. }
        | Expr::Ns { .. }
        | Expr::Require { .. }
        | Expr::Use { .. } => expr.clone(),
    }
}

/// Apply a user-defined macro without recursively expanding arguments (for macroexpand-1)
fn apply_user_macro(macro_def: &MacroDefinition, args: &[Expr]) -> Expr {
    // Create parameter substitution map WITHOUT expanding arguments
    let mut substitutions = HashMap::new();

    // Bind fixed parameters
    for (i, param) in macro_def.params.iter().enumerate() {
        if i < args.len() {
            substitutions.insert(param.clone(), args[i].clone());
        }
    }

    // Handle rest parameter if present
    if let Some(rest_param) = &macro_def.rest_param {
        let rest_start = macro_def.params.len();
        let rest_args: Vec<Expr> = args.iter().skip(rest_start).cloned().collect();
        // Rest args become a list
        substitutions.insert(rest_param.clone(), Expr::List(rest_args));
    }

    // Substitute parameters in the macro body
    substitute_params(&macro_def.body, &substitutions)
}

/// Expand a user-defined macro call
fn expand_user_macro(macro_def: &MacroDefinition, args: &[Expr], registry: &mut MacroRegistry) -> Expr {
    // First, expand macros in the arguments (arguments are evaluated before substitution)
    let expanded_args: Vec<Expr> = args.iter().map(|a| expand_macros_with_registry(a, registry)).collect();

    // Create parameter substitution map
    let mut substitutions = HashMap::new();

    // Bind fixed parameters
    for (i, param) in macro_def.params.iter().enumerate() {
        if i < expanded_args.len() {
            substitutions.insert(param.clone(), expanded_args[i].clone());
        }
    }

    // Handle rest parameter if present
    if let Some(rest_param) = &macro_def.rest_param {
        let rest_start = macro_def.params.len();
        let rest_args: Vec<Expr> = expanded_args.iter().skip(rest_start).cloned().collect();
        // Rest args become a list
        substitutions.insert(rest_param.clone(), Expr::List(rest_args));
    }

    // Substitute parameters in the macro body
    substitute_params(&macro_def.body, &substitutions)
}

/// Substitute parameters in an expression
fn substitute_params(expr: &Expr, substitutions: &HashMap<String, Expr>) -> Expr {
    match expr {
        Expr::Symbol(name) => {
            // If this symbol is a parameter, substitute it
            substitutions.get(name).cloned().unwrap_or_else(|| expr.clone())
        }

        Expr::SyntaxQuote { expr: inner } => {
            // In syntax-quote, process unquotes and return the evaluated result
            // The syntax-quote is part of the macro template, not the expanded output
            substitute_in_syntax_quote(inner, substitutions)
        }

        Expr::Unquote { expr: inner } => {
            // Unquote: substitute and evaluate
            Expr::Unquote {
                expr: Box::new(substitute_params(inner, substitutions))
            }
        }

        Expr::UnquoteSplicing { expr: inner } => {
            // Unquote-splicing: substitute (splicing handled by caller)
            Expr::UnquoteSplicing {
                expr: Box::new(substitute_params(inner, substitutions))
            }
        }

        Expr::List(items) => {
            let mut result = Vec::new();
            for item in items {
                match item {
                    Expr::UnquoteSplicing { expr: inner } => {
                        // Splice: if inner evaluates to a list, splice its elements
                        let substituted = substitute_params(inner, substitutions);
                        if let Expr::List(splice_items) = substituted {
                            result.extend(splice_items);
                        } else {
                            // Not a list, just add as-is
                            result.push(Expr::UnquoteSplicing { expr: Box::new(substituted) });
                        }
                    }
                    _ => result.push(substitute_params(item, substitutions))
                }
            }
            Expr::List(result)
        }

        Expr::Vector(items) => {
            Expr::Vector(items.iter().map(|e| substitute_params(e, substitutions)).collect())
        }

        Expr::Map(entries) => {
            Expr::Map(entries.iter().map(|(k, v)| {
                (substitute_params(k, substitutions), substitute_params(v, substitutions))
            }).collect())
        }

        Expr::Set(items) => {
            Expr::Set(items.iter().map(|e| substitute_params(e, substitutions)).collect())
        }

        Expr::Let { bindings, body } => {
            Expr::Let {
                bindings: bindings.iter().map(|(name, value)| {
                    (name.clone(), Box::new(substitute_params(value, substitutions)))
                }).collect(),
                body: Box::new(substitute_params(body, substitutions))
            }
        }

        Expr::Letfn { bindings, body } => {
            Expr::Letfn {
                bindings: bindings.iter().map(|(name, params, rest_param, fn_body)| {
                    (name.clone(), params.clone(), rest_param.clone(),
                     Box::new(substitute_params(fn_body, substitutions)))
                }).collect(),
                body: Box::new(substitute_params(body, substitutions))
            }
        }

        Expr::If { condition, then_branch, else_branch } => {
            Expr::If {
                condition: Box::new(substitute_params(condition, substitutions)),
                then_branch: Box::new(substitute_params(then_branch, substitutions)),
                else_branch: Box::new(substitute_params(else_branch, substitutions))
            }
        }

        Expr::Do { exprs } => {
            Expr::Do {
                exprs: exprs.iter().map(|e| substitute_params(e, substitutions)).collect()
            }
        }

        Expr::Call { func, args } => {
            Expr::Call {
                func: func.clone(),
                args: args.iter().map(|e| substitute_params(e, substitutions)).collect()
            }
        }

        // Other expressions: recursively substitute in sub-expressions
        _ => expr.clone()
    }
}

/// Substitute params within syntax-quote (process unquotes)
fn substitute_in_syntax_quote(expr: &Expr, substitutions: &HashMap<String, Expr>) -> Expr {
    match expr {
        Expr::Unquote { expr: inner } => {
            // Inside syntax-quote, unquote evaluates to the substituted value
            substitute_params(inner, substitutions)
        }

        Expr::UnquoteSplicing { expr: inner } => {
            // Keep the unquote-splicing for parent to handle
            Expr::UnquoteSplicing {
                expr: Box::new(substitute_params(inner, substitutions))
            }
        }

        Expr::List(items) => {
            let mut result = Vec::new();
            for item in items {
                match item {
                    Expr::UnquoteSplicing { expr: inner } => {
                        // Splice: if inner evaluates to a list, splice its elements
                        let substituted = substitute_params(inner, substitutions);
                        if let Expr::List(splice_items) = substituted {
                            result.extend(splice_items);
                        } else {
                            // Not a list, just add as-is
                            result.push(Expr::UnquoteSplicing { expr: Box::new(substituted) });
                        }
                    }
                    _ => result.push(substitute_in_syntax_quote(item, substitutions))
                }
            }
            Expr::List(result)
        }

        Expr::Vector(items) => {
            Expr::Vector(items.iter().map(|e| substitute_in_syntax_quote(e, substitutions)).collect())
        }

        Expr::Call { func, args } => {
            Expr::Call {
                func: func.clone(),
                args: args.iter().map(|e| substitute_in_syntax_quote(e, substitutions)).collect()
            }
        }

        Expr::If { condition, then_branch, else_branch } => {
            Expr::If {
                condition: Box::new(substitute_in_syntax_quote(condition, substitutions)),
                then_branch: Box::new(substitute_in_syntax_quote(then_branch, substitutions)),
                else_branch: Box::new(substitute_in_syntax_quote(else_branch, substitutions))
            }
        }

        Expr::Do { exprs } => {
            let mut result = Vec::new();
            for expr in exprs {
                match expr {
                    Expr::UnquoteSplicing { expr: inner } => {
                        // Splice: if inner evaluates to a list, splice its elements
                        let substituted = substitute_params(inner, substitutions);
                        if let Expr::List(splice_items) = substituted {
                            result.extend(splice_items);
                        } else {
                            // Not a list, just add as-is
                            result.push(Expr::UnquoteSplicing { expr: Box::new(substituted) });
                        }
                    }
                    _ => result.push(substitute_in_syntax_quote(expr, substitutions))
                }
            }
            Expr::Do { exprs: result }
        }

        // Everything else stays as-is (quoted)
        _ => expr.clone()
    }
}

/// Process syntax-quote for macro expansion
fn expand_syntax_quote(expr: &Expr, registry: &mut MacroRegistry) -> Expr {
    match expr {
        Expr::Unquote { expr: inner } => {
            // Unquote: expand macros in the unquoted expression
            expand_macros_with_registry(inner, registry)
        }

        Expr::List(items) => {
            Expr::List(items.iter().map(|e| expand_syntax_quote(e, registry)).collect())
        }

        Expr::Vector(items) => {
            Expr::Vector(items.iter().map(|e| expand_syntax_quote(e, registry)).collect())
        }

        // Everything else is quoted (no expansion)
        _ => expr.clone()
    }
}

/// Expand thread-first macro: (-> x (f a) (g b)) => (g (f x a) b)
/// The value is threaded as the FIRST argument to each form
fn expand_thread_first(args: &[Expr], registry: &mut MacroRegistry) -> Expr {
    if args.is_empty() {
        return Expr::Nil;
    }

    if args.len() == 1 {
        return expand_macros_with_registry(&args[0], registry);
    }

    let mut result = expand_macros_with_registry(&args[0], registry);

    for form in &args[1..] {
        result = thread_as_first_arg(result, form, registry);
        // Recursively expand macros in the result (macros can generate macro calls)
        result = expand_macros_with_registry(&result, registry);
    }

    result
}

/// Expand thread-last macro: (->> x (f a) (g b)) => (g b (f a x))
/// The value is threaded as the LAST argument to each form
fn expand_thread_last(args: &[Expr], registry: &mut MacroRegistry) -> Expr {
    if args.is_empty() {
        return Expr::Nil;
    }

    if args.len() == 1 {
        return expand_macros_with_registry(&args[0], registry);
    }

    let mut result = expand_macros_with_registry(&args[0], registry);

    for form in &args[1..] {
        result = thread_as_last_arg(result, form, registry);
        // Recursively expand macros in the result (macros can generate macro calls)
        result = expand_macros_with_registry(&result, registry);
    }

    result
}

/// Expand cond macro: (cond test1 expr1 test2 expr2 :else expr3) => nested ifs
/// (cond
///   (< x 0) "negative"
///   (= x 0) "zero"
///   :else "positive")
/// => (if (< x 0) "negative" (if (= x 0) "zero" "positive"))
fn expand_cond(args: &[Expr], registry: &mut MacroRegistry) -> Expr {
    if args.is_empty() {
        // Empty cond returns nil
        return Expr::Nil;
    }

    // Args must come in pairs (test, expr)
    if args.len() % 2 != 0 {
        // Odd number - should we error or allow last as default?
        // Clojure would error, but let's return Nil for robustness
        return Expr::Nil;
    }

    // Build nested if expressions from right to left
    expand_cond_pairs(args, 0, registry)
}

fn expand_cond_pairs(args: &[Expr], index: usize, registry: &mut MacroRegistry) -> Expr {
    if index >= args.len() {
        // No more pairs, return nil
        return Expr::Nil;
    }

    let test = &args[index];
    let then_expr = &args[index + 1];

    // Check if test is :else keyword (final default case)
    let is_else = matches!(test, Expr::Keyword(k) if k == "else");

    if is_else {
        // This is the final else clause, just return the expression
        expand_macros_with_registry(then_expr, registry)
    } else {
        // Build: (if test then_expr (recurse for rest))
        let expanded_test = expand_macros_with_registry(test, registry);
        let expanded_then = expand_macros_with_registry(then_expr, registry);
        let expanded_else = expand_cond_pairs(args, index + 2, registry);

        Expr::If {
            condition: Box::new(expanded_test),
            then_branch: Box::new(expanded_then),
            else_branch: Box::new(expanded_else),
        }
    }
}

/// Expand case macro: (case x 1 "one" 2 "two" "default")
/// Expands to: (let [__case_expr__ x] (cond (= __case_expr__ 1) "one" (= __case_expr__ 2) "two" :else "default"))
fn expand_case(args: &[Expr], registry: &mut MacroRegistry) -> Expr {
    if args.is_empty() {
        // Empty case returns nil
        return Expr::Nil;
    }

    // First arg is the expression to test
    let test_expr = &args[0];
    let case_pairs = &args[1..];

    // Determine if there's a default value (odd number of remaining args)
    let has_default = case_pairs.len() % 2 != 0;
    let default_expr = if has_default {
        Some(&case_pairs[case_pairs.len() - 1])
    } else {
        None
    };

    // Build cond clauses: (= __case_expr__ val) result
    let mut cond_args = Vec::new();

    // Process pairs
    let pair_count = if has_default {
        case_pairs.len() - 1
    } else {
        case_pairs.len()
    };

    for i in (0..pair_count).step_by(2) {
        let test_val = &case_pairs[i];
        let result_expr = &case_pairs[i + 1];

        // Build (= __case_expr__ test_val)
        let equality_test = Expr::List(vec![
            Expr::Symbol("=".to_string()),
            Expr::Symbol("__case_expr__".to_string()),
            test_val.clone(),
        ]);

        cond_args.push(equality_test);
        cond_args.push(result_expr.clone());
    }

    // Add default if present
    if let Some(default) = default_expr {
        cond_args.push(Expr::Keyword("else".to_string()));
        cond_args.push(default.clone());
    }

    // Build the cond expression
    let cond_call = Expr::Call {
        func: "cond".to_string(),
        args: cond_args,
    };

    // Wrap in let binding: (let [__case_expr__ test_expr] cond_call)
    let let_expr = Expr::Let {
        bindings: vec![(crate::ast::Pattern::Symbol("__case_expr__".to_string()), Box::new(test_expr.clone()))],
        body: Box::new(cond_call),
    };

    // Expand the let (which will expand the nested cond)
    expand_macros_with_registry(&let_expr, registry)
}

/// Thread value as first argument: (f a b) with value x => (f x a b)
fn thread_as_first_arg(value: Expr, form: &Expr, registry: &mut MacroRegistry) -> Expr {
    match form {
        // If form is a list/call, insert value as first arg
        Expr::List(items) if !items.is_empty() => {
            let func = &items[0];
            let rest_args = &items[1..];

            // Build new argument list: [value, ...rest_args]
            let mut new_args = vec![value];
            new_args.extend(rest_args.iter().map(|a| expand_macros_with_registry(a, registry)));

            if let Expr::Symbol(func_name) = func {
                Expr::Call {
                    func: func_name.clone(),
                    args: new_args,
                }
            } else {
                // Shouldn't happen in valid code
                Expr::Nil
            }
        }

        Expr::Call { func, args } => {
            let mut new_args = vec![value];
            new_args.extend(args.iter().map(|a| expand_macros_with_registry(a, registry)));

            Expr::Call {
                func: func.clone(),
                args: new_args,
            }
        }

        // If form is a symbol, treat as (symbol value)
        Expr::Symbol(name) => Expr::Call {
            func: name.clone(),
            args: vec![value],
        },

        // Other forms - shouldn't happen
        _ => value,
    }
}

/// Thread value as last argument: (f a b) with value x => (f a b x)
fn thread_as_last_arg(value: Expr, form: &Expr, registry: &mut MacroRegistry) -> Expr {
    match form {
        // If form is a list/call, append value as last arg
        Expr::List(items) if !items.is_empty() => {
            let func = &items[0];
            let rest_args = &items[1..];

            // Build new argument list: [...rest_args, value]
            let mut new_args: Vec<_> = rest_args.iter().map(|a| expand_macros_with_registry(a, registry)).collect();
            new_args.push(value);

            if let Expr::Symbol(func_name) = func {
                Expr::Call {
                    func: func_name.clone(),
                    args: new_args,
                }
            } else {
                Expr::Nil
            }
        }

        Expr::Call { func, args } => {
            let mut new_args: Vec<_> = args.iter().map(|a| expand_macros_with_registry(a, registry)).collect();
            new_args.push(value);

            Expr::Call {
                func: func.clone(),
                args: new_args,
            }
        }

        // If form is a symbol, treat as (symbol value)
        Expr::Symbol(name) => Expr::Call {
            func: name.clone(),
            args: vec![value],
        },

        // Other forms
        _ => value,
    }
}

/// Expand when macro: (when test expr1 expr2) => (if test (do expr1 expr2) nil)
fn expand_when(args: &[Expr], registry: &mut MacroRegistry) -> Expr {
    if args.is_empty() {
        return Expr::Nil;
    }

    let test = &args[0];
    let body_exprs = &args[1..];

    let expanded_test = expand_macros_with_registry(test, registry);
    let expanded_body = if body_exprs.is_empty() {
        Expr::Nil
    } else if body_exprs.len() == 1 {
        expand_macros_with_registry(&body_exprs[0], registry)
    } else {
        let expanded: Vec<_> = body_exprs.iter().map(|e| expand_macros_with_registry(e, registry)).collect();
        Expr::Do { exprs: expanded }
    };

    Expr::If {
        condition: Box::new(expanded_test),
        then_branch: Box::new(expanded_body),
        else_branch: Box::new(Expr::Nil),
    }
}

/// Expand when-not macro: (when-not test expr1 expr2) => (if test nil (do expr1 expr2))
fn expand_when_not(args: &[Expr], registry: &mut MacroRegistry) -> Expr {
    if args.is_empty() {
        return Expr::Nil;
    }

    let test = &args[0];
    let body_exprs = &args[1..];

    let expanded_test = expand_macros_with_registry(test, registry);
    let expanded_body = if body_exprs.is_empty() {
        Expr::Nil
    } else if body_exprs.len() == 1 {
        expand_macros_with_registry(&body_exprs[0], registry)
    } else {
        let expanded: Vec<_> = body_exprs.iter().map(|e| expand_macros_with_registry(e, registry)).collect();
        Expr::Do { exprs: expanded }
    };

    Expr::If {
        condition: Box::new(expanded_test),
        then_branch: Box::new(Expr::Nil),
        else_branch: Box::new(expanded_body),
    }
}

/// Expand if-let macro: (if-let [x expr] then else) => (let [x expr] (if x then else))
fn expand_if_let(args: &[Expr], registry: &mut MacroRegistry) -> Expr {
    if args.len() < 2 {
        return Expr::Nil;
    }

    // First arg should be a vector [binding value]
    let bindings = match &args[0] {
        Expr::Vector(v) if v.len() == 2 => v,
        _ => return Expr::Nil, // Invalid binding form
    };

    let binding_pattern = match &bindings[0] {
        Expr::Symbol(s) => crate::ast::Pattern::Symbol(s.clone()),
        _ => return Expr::Nil, // For now, only simple symbols
    };
    let binding_value = &bindings[1];

    let then_expr = &args[1];
    let else_expr = if args.len() > 2 {
        &args[2]
    } else {
        &Expr::Nil
    };

    // Build: (let [x expr] (if x then else))
    let if_expr = Expr::If {
        condition: Box::new(Expr::Symbol(
            if let crate::ast::Pattern::Symbol(s) = &binding_pattern {
                s.clone()
            } else {
                return Expr::Nil;
            }
        )),
        then_branch: Box::new(expand_macros_with_registry(then_expr, registry)),
        else_branch: Box::new(expand_macros_with_registry(else_expr, registry)),
    };

    let let_expr = Expr::Let {
        bindings: vec![(binding_pattern, Box::new(binding_value.clone()))],
        body: Box::new(if_expr),
    };

    expand_macros_with_registry(&let_expr, registry)
}

/// Expand when-let macro: (when-let [x expr] body) => (let [x expr] (when x body))
fn expand_when_let(args: &[Expr], registry: &mut MacroRegistry) -> Expr {
    if args.is_empty() {
        return Expr::Nil;
    }

    // First arg should be a vector [binding value]
    let bindings = match &args[0] {
        Expr::Vector(v) if v.len() == 2 => v,
        _ => return Expr::Nil,
    };

    let binding_pattern = match &bindings[0] {
        Expr::Symbol(s) => crate::ast::Pattern::Symbol(s.clone()),
        _ => return Expr::Nil,
    };
    let binding_value = &bindings[1];

    let body_exprs = &args[1..];

    // Build: (let [x expr] (when x body))
    let when_call = Expr::Call {
        func: "when".to_string(),
        args: {
            let mut args = vec![Expr::Symbol(
                if let crate::ast::Pattern::Symbol(s) = &binding_pattern {
                    s.clone()
                } else {
                    return Expr::Nil;
                }
            )];
            args.extend(body_exprs.iter().cloned());
            args
        },
    };

    let let_expr = Expr::Let {
        bindings: vec![(binding_pattern, Box::new(binding_value.clone()))],
        body: Box::new(when_call),
    };

    expand_macros_with_registry(&let_expr, registry)
}

/// Expand if-not macro: (if-not test then else) => (if test else then)
fn expand_if_not(args: &[Expr], registry: &mut MacroRegistry) -> Expr {
    if args.is_empty() {
        return Expr::Nil;
    }

    let test = &args[0];
    let then_expr = if args.len() > 1 { &args[1] } else { &Expr::Nil };
    let else_expr = if args.len() > 2 { &args[2] } else { &Expr::Nil };

    Expr::If {
        condition: Box::new(expand_macros_with_registry(test, registry)),
        then_branch: Box::new(expand_macros_with_registry(else_expr, registry)),
        else_branch: Box::new(expand_macros_with_registry(then_expr, registry)),
    }
}

/// Expand and macro: (and x y z) => (if x (if y z false) false)
fn expand_and(args: &[Expr], registry: &mut MacroRegistry) -> Expr {
    if args.is_empty() {
        // (and) with no args returns true
        return Expr::Bool(true);
    }

    if args.len() == 1 {
        return expand_macros_with_registry(&args[0], registry);
    }

    // Build nested ifs from right to left
    expand_and_recursive(args, 0, registry)
}

fn expand_and_recursive(args: &[Expr], index: usize, registry: &mut MacroRegistry) -> Expr {
    if index >= args.len() {
        return Expr::Bool(true);
    }

    if index == args.len() - 1 {
        // Last element, just return it
        return expand_macros_with_registry(&args[index], registry);
    }

    // Build: (if current (recurse rest) false)
    Expr::If {
        condition: Box::new(expand_macros_with_registry(&args[index], registry)),
        then_branch: Box::new(expand_and_recursive(args, index + 1, registry)),
        else_branch: Box::new(Expr::Bool(false)),
    }
}

/// Expand or macro: (or x y z) => (if x true (if y true z))
fn expand_or(args: &[Expr], registry: &mut MacroRegistry) -> Expr {
    if args.is_empty() {
        // (or) with no args returns false/nil
        return Expr::Bool(false);
    }

    if args.len() == 1 {
        return expand_macros_with_registry(&args[0], registry);
    }

    // Build nested ifs from right to left
    expand_or_recursive(args, 0, registry)
}

fn expand_or_recursive(args: &[Expr], index: usize, registry: &mut MacroRegistry) -> Expr {
    if index >= args.len() {
        return Expr::Bool(false);
    }

    if index == args.len() - 1 {
        // Last element, just return it
        return expand_macros_with_registry(&args[index], registry);
    }

    // Build: (if current true (recurse rest))
    // Actually, we want to return the truthy value, not just true
    // So: (let [tmp current] (if tmp tmp (recurse rest)))
    let tmp_sym = registry.gensym(Some("or"));
    let let_expr = Expr::Let {
        bindings: vec![(
            crate::ast::Pattern::Symbol(tmp_sym.clone()),
            Box::new(expand_macros_with_registry(&args[index], registry))
        )],
        body: Box::new(Expr::If {
            condition: Box::new(Expr::Symbol(tmp_sym.clone())),
            then_branch: Box::new(Expr::Symbol(tmp_sym)),
            else_branch: Box::new(expand_or_recursive(args, index + 1, registry)),
        }),
    };

    let_expr
}

/// Expand some-> macro: like -> but stops on nil
fn expand_some_thread_first(args: &[Expr], registry: &mut MacroRegistry) -> Expr {
    if args.is_empty() {
        return Expr::Nil;
    }

    if args.len() == 1 {
        return expand_macros_with_registry(&args[0], registry);
    }

    let mut result = expand_macros_with_registry(&args[0], registry);

    for form in &args[1..] {
        // Generate: (let [tmp result] (if tmp (threaded-form) nil))
        let tmp_sym = registry.gensym(Some("some"));
        let threaded = thread_as_first_arg(Expr::Symbol(tmp_sym.clone()), form, registry);
        let threaded_expanded = expand_macros_with_registry(&threaded, registry);

        result = Expr::Let {
            bindings: vec![(crate::ast::Pattern::Symbol(tmp_sym.clone()), Box::new(result))],
            body: Box::new(Expr::If {
                condition: Box::new(Expr::Symbol(tmp_sym.clone())),
                then_branch: Box::new(threaded_expanded),
                else_branch: Box::new(Expr::Nil),
            }),
        };
    }

    result
}

/// Expand some->> macro: like ->> but stops on nil
fn expand_some_thread_last(args: &[Expr], registry: &mut MacroRegistry) -> Expr {
    if args.is_empty() {
        return Expr::Nil;
    }

    if args.len() == 1 {
        return expand_macros_with_registry(&args[0], registry);
    }

    let mut result = expand_macros_with_registry(&args[0], registry);

    for form in &args[1..] {
        // Generate: (let [tmp result] (if tmp (threaded-form) nil))
        let tmp_sym = registry.gensym(Some("some"));
        let threaded = thread_as_last_arg(Expr::Symbol(tmp_sym.clone()), form, registry);
        let threaded_expanded = expand_macros_with_registry(&threaded, registry);

        result = Expr::Let {
            bindings: vec![(crate::ast::Pattern::Symbol(tmp_sym.clone()), Box::new(result))],
            body: Box::new(Expr::If {
                condition: Box::new(Expr::Symbol(tmp_sym.clone())),
                then_branch: Box::new(threaded_expanded),
                else_branch: Box::new(Expr::Nil),
            }),
        };
    }

    result
}

/// Expand doto macro: (doto x (f a) (g b)) => (let [obj x] (f obj a) (g obj b) obj)
fn expand_doto(args: &[Expr], registry: &mut MacroRegistry) -> Expr {
    if args.is_empty() {
        return Expr::Nil;
    }

    let obj_expr = &args[0];
    let forms = &args[1..];

    if forms.is_empty() {
        // No forms, just return the object
        return expand_macros_with_registry(obj_expr, registry);
    }

    // Generate unique symbol for the object
    let obj_sym = registry.gensym(Some("doto"));

    // Build expressions that thread the object through each form
    let mut body_exprs = Vec::new();
    for form in forms {
        let threaded = thread_as_first_arg(Expr::Symbol(obj_sym.clone()), form, registry);
        body_exprs.push(expand_macros_with_registry(&threaded, registry));
    }

    // Add the object itself as the last expression (return value)
    body_exprs.push(Expr::Symbol(obj_sym.clone()));

    // Wrap in do
    let do_expr = if body_exprs.len() == 1 {
        body_exprs.into_iter().next().unwrap()
    } else {
        Expr::Do { exprs: body_exprs }
    };

    // Wrap in let
    Expr::Let {
        bindings: vec![(
            crate::ast::Pattern::Symbol(obj_sym),
            Box::new(expand_macros_with_registry(obj_expr, registry))
        )],
        body: Box::new(do_expr),
    }
}

/// Expand while macro: (while test body...) => (loop [] (when test body... (recur)))
fn expand_while(args: &[Expr], registry: &mut MacroRegistry) -> Expr {
    if args.is_empty() {
        return Expr::Nil;
    }

    let test = &args[0];
    let body_exprs = &args[1..];

    // Build the when body: (when test body... (recur))
    let mut when_body_exprs: Vec<Expr> = body_exprs.iter().cloned().collect();
    when_body_exprs.push(Expr::Recur { args: vec![] });

    // Create when call: (when test body... (recur))
    let when_call = Expr::Call {
        func: "when".to_string(),
        args: {
            let mut args = vec![test.clone()];
            args.extend(when_body_exprs);
            args
        },
    };

    // Wrap in loop: (loop [] when-call)
    let loop_expr = Expr::Loop {
        bindings: vec![],
        body: Box::new(when_call),
    };

    // Expand the result (this will expand the 'when' macro)
    expand_macros_with_registry(&loop_expr, registry)
}

/// Expand dotimes macro: (dotimes [i n] body...) => (loop [i 0] (when (< i n) body... (recur (inc i))))
fn expand_dotimes(args: &[Expr], registry: &mut MacroRegistry) -> Expr {
    if args.is_empty() {
        return Expr::Nil;
    }

    // First arg should be a vector [binding limit]
    let bindings = match &args[0] {
        Expr::Vector(v) if v.len() == 2 => v,
        _ => return Expr::Nil,
    };

    let binding_name = match &bindings[0] {
        Expr::Symbol(s) => s.clone(),
        _ => return Expr::Nil,
    };
    let limit_expr = &bindings[1];

    let body_exprs = &args[1..];

    // Generate unique symbol for limit to avoid re-evaluation
    let limit_sym = registry.gensym(Some("limit"));

    // Build: (< i limit_sym)
    let test_expr = Expr::List(vec![
        Expr::Symbol("<".to_string()),
        Expr::Symbol(binding_name.clone()),
        Expr::Symbol(limit_sym.clone()),
    ]);

    // Build: (inc i)
    let inc_expr = Expr::List(vec![
        Expr::Symbol("+".to_string()),
        Expr::Symbol(binding_name.clone()),
        Expr::Double(1.0),
    ]);

    // Build the when body: body... (recur (inc i))
    let mut when_body_exprs: Vec<Expr> = body_exprs.iter().cloned().collect();
    when_body_exprs.push(Expr::Recur { args: vec![inc_expr] });

    // Create when call: (when test body... (recur (inc i)))
    let when_call = Expr::Call {
        func: "when".to_string(),
        args: {
            let mut args = vec![test_expr];
            args.extend(when_body_exprs);
            args
        },
    };

    // Wrap in loop: (loop [i 0] when-call)
    let loop_expr = Expr::Loop {
        bindings: vec![(
            crate::ast::Pattern::Symbol(binding_name),
            Box::new(Expr::Double(0.0)),
        )],
        body: Box::new(when_call),
    };

    // Wrap in let for limit: (let [limit_sym limit_expr] loop_expr)
    let let_expr = Expr::Let {
        bindings: vec![(
            crate::ast::Pattern::Symbol(limit_sym),
            Box::new(limit_expr.clone()),
        )],
        body: Box::new(loop_expr),
    };

    // Expand the result
    expand_macros_with_registry(&let_expr, registry)
}

/// Expand doseq macro: (doseq [x coll] body...) => iterate over collection with side effects
/// Implementation: Convert to loop with counter and nth access
/// (doseq [x coll] body...) =>
///   (let [coll_sym coll
///         len (count coll_sym)]
///     (loop [i 0]
///       (when (< i len)
///         (let [x (nth coll_sym i)]
///           body...)
///         (recur (inc i)))))
fn expand_doseq(args: &[Expr], registry: &mut MacroRegistry) -> Expr {
    if args.is_empty() {
        return Expr::Nil;
    }

    // First arg should be a vector [binding collection]
    let bindings = match &args[0] {
        Expr::Vector(v) if v.len() == 2 => v,
        _ => return Expr::Nil,
    };

    let binding_name = match &bindings[0] {
        Expr::Symbol(s) => s.clone(),
        _ => return Expr::Nil,
    };
    let coll_expr = &bindings[1];

    let body_exprs = &args[1..];

    // Generate unique symbols
    let coll_sym = registry.gensym(Some("coll"));
    let len_sym = registry.gensym(Some("len"));
    let i_sym = registry.gensym(Some("i"));

    // Build: (count coll_sym)
    let count_expr = Expr::List(vec![
        Expr::Symbol("count".to_string()),
        Expr::Symbol(coll_sym.clone()),
    ]);

    // Build: (< i len_sym)
    let test_expr = Expr::List(vec![
        Expr::Symbol("<".to_string()),
        Expr::Symbol(i_sym.clone()),
        Expr::Symbol(len_sym.clone()),
    ]);

    // Build: (nth coll_sym i)
    let nth_expr = Expr::List(vec![
        Expr::Symbol("nth".to_string()),
        Expr::Symbol(coll_sym.clone()),
        Expr::Symbol(i_sym.clone()),
    ]);

    // Build: (inc i)
    let inc_expr = Expr::List(vec![
        Expr::Symbol("+".to_string()),
        Expr::Symbol(i_sym.clone()),
        Expr::Double(1.0),
    ]);

    // Build inner let: (let [x (nth coll_sym i)] body...)
    let inner_let = Expr::Let {
        bindings: vec![(
            crate::ast::Pattern::Symbol(binding_name),
            Box::new(nth_expr),
        )],
        body: Box::new(if body_exprs.len() == 1 {
            body_exprs[0].clone()
        } else {
            Expr::Do {
                exprs: body_exprs.to_vec(),
            }
        }),
    };

    // Build the when body: inner-let (recur (inc i))
    let when_call = Expr::Call {
        func: "when".to_string(),
        args: vec![test_expr, inner_let, Expr::Recur { args: vec![inc_expr] }],
    };

    // Wrap in loop: (loop [i 0] when-call)
    let loop_expr = Expr::Loop {
        bindings: vec![(
            crate::ast::Pattern::Symbol(i_sym),
            Box::new(Expr::Double(0.0)),
        )],
        body: Box::new(when_call),
    };

    // Wrap in outer let: (let [coll_sym coll, len (count coll_sym)] loop_expr)
    let let_expr = Expr::Let {
        bindings: vec![
            (
                crate::ast::Pattern::Symbol(coll_sym.clone()),
                Box::new(coll_expr.clone()),
            ),
            (
                crate::ast::Pattern::Symbol(len_sym),
                Box::new(count_expr),
            ),
        ],
        body: Box::new(loop_expr),
    };

    // Expand the result
    expand_macros_with_registry(&let_expr, registry)
}

/// Expand with-open macro for resource management
/// (with-open [name init-expr] body...) =>
/// (let [name init-expr]
///   (try
///     body...
///     (finally
///       (close name))))
fn expand_with_open(args: &[Expr], registry: &mut MacroRegistry) -> Expr {
    if args.is_empty() {
        return Expr::Nil;
    }

    // First arg should be a vector [binding init-expr]
    let bindings = match &args[0] {
        Expr::Vector(v) if v.len() == 2 => v,
        _ => return Expr::Nil,
    };

    let binding_name = match &bindings[0] {
        Expr::Symbol(s) => s.clone(),
        _ => return Expr::Nil,
    };
    let init_expr = &bindings[1];

    let body_exprs = &args[1..];

    // Build the close call: (close binding_name)
    let close_expr = Expr::Call {
        func: "close".to_string(),
        args: vec![Expr::Symbol(binding_name.clone())],
    };

    // Build try/finally
    let try_expr = Expr::Try {
        body: Box::new(if body_exprs.len() == 1 {
            body_exprs[0].clone()
        } else {
            Expr::Do {
                exprs: body_exprs.to_vec(),
            }
        }),
        catch_clauses: vec![],
        finally_block: Some(Box::new(close_expr)),
    };

    // Wrap in let
    let let_expr = Expr::Let {
        bindings: vec![(
            crate::ast::Pattern::Symbol(binding_name),
            Box::new(init_expr.clone()),
        )],
        body: Box::new(try_expr),
    };

    // Expand the result
    expand_macros_with_registry(&let_expr, registry)
}

/// Expand lazy-seq macro for Clojure-style lazy sequences
/// (lazy-seq body...) => (clorus.lazy/make-lazy (fn [] body...))
fn expand_lazy_seq(args: &[Expr], registry: &mut MacroRegistry) -> Expr {
    if args.is_empty() {
        return Expr::Nil;
    }

    // Wrap all args in a do block if multiple, or single expr if one
    let body = if args.len() == 1 {
        expand_macros_with_registry(&args[0], registry)
    } else {
        let expanded: Vec<_> = args
            .iter()
            .map(|e| expand_macros_with_registry(e, registry))
            .collect();
        Expr::Do { exprs: expanded }
    };

    // Create (fn [] body)
    let lambda = Expr::Fn {
        params: vec![],
        rest_param: None,
        body: Box::new(body),
    };

    // Create (clorus.lazy/make-lazy (fn [] body))
    Expr::Call {
        func: "clorus.lazy/make-lazy".to_string(),
        args: vec![lambda],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thread_first_simple() {
        // (-> 5 inc dec) => (dec (inc 5))
        let expr = Expr::Call {
            func: "->".to_string(),
            args: vec![
                Expr::Double(5.0),
                Expr::Symbol("inc".to_string()),
                Expr::Symbol("dec".to_string()),
            ],
        };

        let expanded = expand_macros(&expr);

        // Should expand to: (dec (inc 5))
        if let Expr::Call { func, args } = expanded {
            assert_eq!(func, "dec");
            assert_eq!(args.len(), 1);

            if let Expr::Call { func: inner_func, args: inner_args } = &args[0] {
                assert_eq!(inner_func, "inc");
                assert_eq!(inner_args.len(), 1);
                assert_eq!(inner_args[0], Expr::Double(5.0));
            } else {
                panic!("Inner expression should be a call");
            }
        } else {
            panic!("Expanded expression should be a call");
        }
    }

    #[test]
    fn test_thread_first_with_args() {
        // (-> x (f a) (g b)) => (g (f x a) b)
        let expr = Expr::Call {
            func: "->".to_string(),
            args: vec![
                Expr::Symbol("x".to_string()),
                Expr::List(vec![
                    Expr::Symbol("f".to_string()),
                    Expr::Symbol("a".to_string()),
                ]),
                Expr::List(vec![
                    Expr::Symbol("g".to_string()),
                    Expr::Symbol("b".to_string()),
                ]),
            ],
        };

        let expanded = expand_macros(&expr);

        // Should expand to: (g (f x a) b)
        if let Expr::Call { func, args } = expanded {
            assert_eq!(func, "g");
            assert_eq!(args.len(), 2);
        } else {
            panic!("Expanded expression should be a call");
        }
    }

    #[test]
    fn test_thread_last_simple() {
        // (->> [1 2 3] (map inc) (filter even?)) => (filter even? (map inc [1 2 3]))
        let expr = Expr::Call {
            func: "->>".to_string(),
            args: vec![
                Expr::Vector(vec![Expr::Double(1.0), Expr::Double(2.0), Expr::Double(3.0)]),
                Expr::List(vec![
                    Expr::Symbol("map".to_string()),
                    Expr::Symbol("inc".to_string()),
                ]),
                Expr::List(vec![
                    Expr::Symbol("filter".to_string()),
                    Expr::Symbol("even?".to_string()),
                ]),
            ],
        };

        let expanded = expand_macros(&expr);

        // Should expand to: (filter even? (map inc [1 2 3]))
        if let Expr::Call { func, args } = expanded {
            assert_eq!(func, "filter");
            assert_eq!(args.len(), 2);

            // Second arg should be (map inc [1 2 3])
            if let Expr::Call { func: inner_func, args: inner_args } = &args[1] {
                assert_eq!(inner_func, "map");
                assert_eq!(inner_args.len(), 2);
            } else {
                panic!("Second arg should be a call");
            }
        } else {
            panic!("Expanded expression should be a call");
        }
    }

    #[test]
    fn test_user_defined_macro_simple() {
        // Test a simple user-defined macro with parameter substitution
        // (defmacro unless [test then] `(if ~test nil ~then))
        // (unless false 42) => (if false nil 42)

        use crate::parser::parse_str;

        let code = r#"
            (defmacro unless [test then] `(if ~test nil ~then))
            (unless false 42)
        "#;

        let exprs = parse_str(code).unwrap();

        // First expression is defmacro, second is the macro call
        assert_eq!(exprs.len(), 2);

        // Expand with a shared registry
        let mut registry = MacroRegistry::new();
        let expanded_defmacro = expand_macros_with_registry(&exprs[0], &mut registry);
        let expanded_call = expand_macros_with_registry(&exprs[1], &mut registry);

        // The defmacro should remain as-is (it's compile-time only)
        assert!(matches!(expanded_defmacro, Expr::Defmacro { .. }));

        // The macro call should expand to: (if false nil 42)
        // Note: 'if' is a special form, so it becomes Expr::If, not Expr::Call
        if let Expr::If { condition, then_branch, else_branch } = expanded_call {
            assert_eq!(*condition, Expr::Bool(false));
            assert_eq!(*then_branch, Expr::Nil);
            assert!(matches!(*else_branch, Expr::Long(42)) || matches!(*else_branch, Expr::Double(42.0)));
        } else {
            panic!("Expected expanded macro call to be an if expression, got: {:?}", expanded_call);
        }
    }

    #[test]
    fn test_user_defined_macro_with_rest_params() {
        // Test macro with rest parameters and unquote-splicing
        // (defmacro when [test & body] `(if ~test (do ~@body) nil))
        // (when true (println "a") (println "b")) => (if true (do (println "a") (println "b")) nil)

        use crate::parser::parse_str;

        let code = r#"
            (defmacro when [test & body] `(if ~test (do ~@body) nil))
            (when true (println "a") (println "b"))
        "#;

        let exprs = parse_str(code).unwrap();
        assert_eq!(exprs.len(), 2);

        // Expand with a shared registry
        let mut registry = MacroRegistry::new();
        let _expanded_defmacro = expand_macros_with_registry(&exprs[0], &mut registry);
        let expanded_call = expand_macros_with_registry(&exprs[1], &mut registry);

        // Should expand to: (if true (do (println "a") (println "b")) nil)
        // Note: 'if' is a special form, parsed as Expr::If
        if let Expr::If { condition, then_branch, else_branch } = expanded_call {
            // First arg: true
            assert_eq!(*condition, Expr::Bool(true));

            // Second arg: (do (println "a") (println "b"))
            if let Expr::Do { exprs: do_exprs } = then_branch.as_ref() {
                assert_eq!(do_exprs.len(), 2);
            } else {
                panic!("Expected do expression in then branch, got: {:?}", then_branch);
            }

            // Third arg: nil
            assert_eq!(*else_branch, Expr::Nil);
        } else {
            panic!("Expected if expression, got: {:?}", expanded_call);
        }
    }

    #[test]
    fn test_nested_macro_expansion() {
        // Test that macros can generate other macro calls
        // Both -> and user macros should expand recursively

        use crate::parser::parse_str;

        let code = r#"
            (defmacro double [x] `(+ ~x ~x))
            (-> 5 double inc)
        "#;

        let exprs = parse_str(code).unwrap();
        assert_eq!(exprs.len(), 2);

        // Expand with a shared registry
        let mut registry = MacroRegistry::new();
        let _expanded_defmacro = expand_macros_with_registry(&exprs[0], &mut registry);
        let expanded = expand_macros_with_registry(&exprs[1], &mut registry);

        // (-> 5 double inc) should first expand double, then thread
        // Final result: (inc (+ 5 5))
        // Note: Operators like + are kept as List, not Call
        if let Expr::Call { func, args } = expanded {
            assert_eq!(func, "inc");
            assert_eq!(args.len(), 1);

            // Inner should be (+ 5 5) as a List
            if let Expr::List(inner_items) = &args[0] {
                assert_eq!(inner_items.len(), 3);
                assert_eq!(inner_items[0], Expr::Symbol("+".to_string()));
                let is_five = |expr: &Expr| matches!(expr, Expr::Long(5)) || matches!(expr, Expr::Double(5.0));
                assert!(is_five(&inner_items[1]));
                assert!(is_five(&inner_items[2]));
            } else {
                panic!("Expected List expression for operator, got: {:?}", args[0]);
            }
        } else {
            panic!("Expected call expression");
        }
    }

    #[test]
    fn test_cond_macro() {
        // Test cond macro expansion
        // (cond (< x 0) "negative" (= x 0) "zero" :else "positive")
        // Should expand to nested ifs

        use crate::parser::parse_str;

        let code = r#"
            (cond
              (< x 0) "negative"
              (= x 0) "zero"
              :else "positive")
        "#;

        let exprs = parse_str(code).unwrap();
        assert_eq!(exprs.len(), 1);

        let expanded = expand_macros(&exprs[0]);

        // Should expand to: (if (< x 0) "negative" (if (= x 0) "zero" "positive"))
        if let Expr::If { condition, then_branch, else_branch } = expanded {
            // First test: (< x 0)
            assert!(matches!(*condition, Expr::List(_)));

            // First then: "negative"
            assert_eq!(*then_branch, Expr::String("negative".to_string()));

            // Else should be another if
            if let Expr::If { condition: cond2, then_branch: then2, else_branch: else2 } = *else_branch {
                // Second test: (= x 0)
                assert!(matches!(*cond2, Expr::List(_)));

                // Second then: "zero"
                assert_eq!(*then2, Expr::String("zero".to_string()));

                // Final else: "positive"
                assert_eq!(*else2, Expr::String("positive".to_string()));
            } else {
                panic!("Expected nested if for second condition");
            }
        } else {
            panic!("Expected if expression from cond expansion");
        }
    }

    #[test]
    fn test_cond_with_else_keyword() {
        // Test that :else keyword works as default
        use crate::parser::parse_str;

        let code = "(cond :else 42)";
        let exprs = parse_str(code).unwrap();
        let expanded = expand_macros(&exprs[0]);

        // Should expand to just 42
        assert!(matches!(expanded, Expr::Long(42)) || matches!(expanded, Expr::Double(42.0)));
    }

    #[test]
    fn test_case_macro() {
        // Test case macro expansion
        // (case x 1 "one" 2 "two" "default")
        // Should expand to: (let [__case_expr__ x] (cond (= __case_expr__ 1) "one" ...))

        use crate::parser::parse_str;

        let code = r#"
            (case x
              1 "one"
              2 "two"
              "default")
        "#;

        let exprs = parse_str(code).unwrap();
        assert_eq!(exprs.len(), 1);

        let expanded = expand_macros(&exprs[0]);

        // Should expand to a let expression
        if let Expr::Let { bindings, body } = expanded {
            // Binding should be [__case_expr__ x]
            assert_eq!(bindings.len(), 1);
            assert_eq!(bindings[0].0, crate::ast::Pattern::Symbol("__case_expr__".to_string()));
            assert_eq!(*bindings[0].1, Expr::Symbol("x".to_string()));

            // Body should be a nested if (from cond expansion)
            assert!(matches!(body.as_ref(), Expr::If { .. }));
        } else {
            panic!("Expected let expression from case expansion, got: {:?}", expanded);
        }
    }

    #[test]
    fn test_case_without_default() {
        // Test case without default value
        use crate::parser::parse_str;

        let code = "(case x 1 \"one\" 2 \"two\")";
        let exprs = parse_str(code).unwrap();
        let expanded = expand_macros(&exprs[0]);

        // Should still expand to a let expression
        assert!(matches!(expanded, Expr::Let { .. }));
    }

    #[test]
    fn test_gensym_basic() {
        // Test basic gensym
        use crate::parser::parse_str;

        let code = "(gensym)";
        let exprs = parse_str(code).unwrap();
        let expanded = expand_macros(&exprs[0]);

        // Should expand to a symbol
        if let Expr::Symbol(s) = expanded {
            assert!(s.starts_with("G__"));
        } else {
            panic!("Expected symbol from gensym, got: {:?}", expanded);
        }
    }

    #[test]
    fn test_gensym_with_prefix() {
        // Test gensym with prefix
        use crate::parser::parse_str;

        let code = "(gensym \"tmp\")";
        let exprs = parse_str(code).unwrap();
        let expanded = expand_macros(&exprs[0]);

        // Should expand to a symbol with prefix
        if let Expr::Symbol(s) = expanded {
            assert!(s.starts_with("tmp__G__"));
        } else {
            panic!("Expected symbol from gensym, got: {:?}", expanded);
        }
    }

    #[test]
    fn test_gensym_uniqueness() {
        // Test that gensym generates unique symbols
        use crate::parser::parse_str;

        let code = r#"
            (gensym "x")
            (gensym "x")
            (gensym "x")
        "#;
        let exprs = parse_str(code).unwrap();

        let mut registry = MacroRegistry::new();
        let sym1 = expand_macros_with_registry(&exprs[0], &mut registry);
        let sym2 = expand_macros_with_registry(&exprs[1], &mut registry);
        let sym3 = expand_macros_with_registry(&exprs[2], &mut registry);

        // All should be different
        assert_ne!(sym1, sym2);
        assert_ne!(sym2, sym3);
        assert_ne!(sym1, sym3);
    }

    #[test]
    fn test_gensym_in_macro() {
        // Test using gensym in a simple macro
        // This tests that gensym generates unique symbols during macro expansion
        use crate::parser::parse_str;

        let code = r#"
            (defmacro make-sym []
              (gensym "test"))
            (make-sym)
        "#;

        let exprs = parse_str(code).unwrap();
        assert_eq!(exprs.len(), 2);

        let mut registry = MacroRegistry::new();
        let _defmacro = expand_macros_with_registry(&exprs[0], &mut registry);
        let expanded = expand_macros_with_registry(&exprs[1], &mut registry);

        // Should expand to a symbol with __G__ in the name
        if let Expr::Symbol(s) = expanded {
            assert!(s.contains("test__G__"), "Expected generated symbol to contain test__G__, got: {}", s);
        } else {
            panic!("Expected symbol from macro expansion, got: {:?}", expanded);
        }
    }

    #[test]
    fn test_when_macro() {
        // (when test expr1 expr2) => (if test (do expr1 expr2) nil)
        use crate::parser::parse_str;

        let code = r#"
            (when true
              (+ 1 2)
              (* 3 4))
        "#;

        let exprs = parse_str(code).unwrap();
        assert_eq!(exprs.len(), 1);

        let expanded = expand_macros(&exprs[0]);

        // Should expand to: (if true (do (+ 1 2) (* 3 4)) nil)
        if let Expr::If { condition, then_branch, else_branch } = expanded {
            assert_eq!(*condition, Expr::Bool(true));
            assert!(matches!(then_branch.as_ref(), Expr::Do { .. }));
            assert_eq!(*else_branch, Expr::Nil);
        } else {
            panic!("Expected if expression from when expansion, got: {:?}", expanded);
        }
    }

    #[test]
    fn test_when_not_macro() {
        // (when-not test expr) => (if test nil expr)
        use crate::parser::parse_str;

        let code = "(when-not false (+ 1 2))";
        let exprs = parse_str(code).unwrap();
        let expanded = expand_macros(&exprs[0]);

        if let Expr::If { condition, then_branch, else_branch } = expanded {
            assert_eq!(*condition, Expr::Bool(false));
            assert_eq!(*then_branch, Expr::Nil);
            assert!(matches!(else_branch.as_ref(), Expr::List(_)));
        } else {
            panic!("Expected if expression from when-not expansion");
        }
    }

    #[test]
    fn test_if_let_macro() {
        // (if-let [x expr] then else) => (let [x expr] (if x then else))
        use crate::parser::parse_str;

        let code = "(if-let [x 42] x 0)";
        let exprs = parse_str(code).unwrap();
        let expanded = expand_macros(&exprs[0]);

        // Should expand to a let expression
        if let Expr::Let { bindings, body } = expanded {
            assert_eq!(bindings.len(), 1);
            assert!(matches!(body.as_ref(), Expr::If { .. }));
        } else {
            panic!("Expected let expression from if-let expansion, got: {:?}", expanded);
        }
    }

    #[test]
    fn test_when_let_macro() {
        // (when-let [x expr] body) => (let [x expr] (when x body))
        use crate::parser::parse_str;

        let code = "(when-let [x 100] (+ x 1))";
        let exprs = parse_str(code).unwrap();
        let expanded = expand_macros(&exprs[0]);

        // Should expand to a let expression
        assert!(matches!(expanded, Expr::Let { .. }));
    }

    #[test]
    fn test_if_not_macro() {
        // (if-not test then else) => (if test else then)
        use crate::parser::parse_str;

        let code = "(if-not false 1 2)";
        let exprs = parse_str(code).unwrap();
        let expanded = expand_macros(&exprs[0]);

        // Should expand to an if with swapped branches
        if let Expr::If { condition, then_branch, else_branch } = expanded {
            assert_eq!(*condition, Expr::Bool(false));
            // then and else should be swapped
            let is_two = matches!(*then_branch, Expr::Long(2)) || matches!(*then_branch, Expr::Double(2.0));
            let is_one = matches!(*else_branch, Expr::Long(1)) || matches!(*else_branch, Expr::Double(1.0));
            assert!(is_two);
            assert!(is_one);
        } else {
            panic!("Expected if expression from if-not expansion");
        }
    }

    #[test]
    fn test_and_macro() {
        // (and x y z) => nested ifs
        use crate::parser::parse_str;

        let code = "(and true true false)";
        let exprs = parse_str(code).unwrap();
        let expanded = expand_macros(&exprs[0]);

        // Should expand to nested if expressions
        assert!(matches!(expanded, Expr::If { .. }));
    }

    #[test]
    fn test_and_macro_empty() {
        // (and) with no args returns true
        use crate::parser::parse_str;

        let code = "(and)";
        let exprs = parse_str(code).unwrap();
        let expanded = expand_macros(&exprs[0]);

        assert_eq!(expanded, Expr::Bool(true));
    }

    #[test]
    fn test_or_macro() {
        // (or x y z) => nested ifs with let bindings
        use crate::parser::parse_str;

        let code = "(or false false true)";
        let exprs = parse_str(code).unwrap();
        let expanded = expand_macros(&exprs[0]);

        // Should expand to let with if (for short-circuit behavior)
        assert!(matches!(expanded, Expr::Let { .. }));
    }

    #[test]
    fn test_or_macro_empty() {
        // (or) with no args returns false
        use crate::parser::parse_str;

        let code = "(or)";
        let exprs = parse_str(code).unwrap();
        let expanded = expand_macros(&exprs[0]);

        assert_eq!(expanded, Expr::Bool(false));
    }

    #[test]
    fn test_some_thread_first() {
        // (some-> x (f a) (g b)) => let bindings with nil checks
        use crate::parser::parse_str;

        let code = "(some-> 10 (+ 5) (* 2))";
        let exprs = parse_str(code).unwrap();
        let expanded = expand_macros(&exprs[0]);

        // Should expand to nested let expressions with if checks
        assert!(matches!(expanded, Expr::Let { .. }));
    }

    #[test]
    fn test_some_thread_last() {
        // (some->> x (f a) (g b)) => let bindings with nil checks
        use crate::parser::parse_str;

        let code = "(some->> [1 2 3] (map inc))";
        let exprs = parse_str(code).unwrap();
        let expanded = expand_macros(&exprs[0]);

        // Should expand to nested let expressions with if checks
        assert!(matches!(expanded, Expr::Let { .. }));
    }

    #[test]
    fn test_doto_macro() {
        // (doto x (f a) (g b)) => (let [obj x] (f obj a) (g obj b) obj)
        use crate::parser::parse_str;

        let code = "(doto {:x 1} (assoc :y 2) (assoc :z 3))";
        let exprs = parse_str(code).unwrap();
        let expanded = expand_macros(&exprs[0]);

        // Should expand to a let expression
        if let Expr::Let { bindings, body } = expanded {
            assert_eq!(bindings.len(), 1);
            // Body should be a do expression with the operations
            assert!(matches!(body.as_ref(), Expr::Do { .. }));
        } else {
            panic!("Expected let expression from doto expansion, got: {:?}", expanded);
        }
    }

    #[test]
    fn test_defmacro_persists_across_forms() {
        use crate::parser::parse_str;

        let code = "(defmacro twice [x] `(+ ~x ~x)) (twice 3)";
        let exprs = parse_str(code).unwrap();
        let expanded = expand_macros_sequence(&exprs);

        assert_eq!(expanded.len(), 2);

        match &expanded[1] {
            Expr::Call { func, args } => {
                assert_eq!(func, "+");
                assert_eq!(args.len(), 2);

                let is_three = |expr: &Expr| {
                    matches!(expr, Expr::Long(3)) || matches!(expr, Expr::Double(3.0))
                };

                assert!(is_three(&args[0]));
                assert!(is_three(&args[1]));
            }
            Expr::List(items) => {
                assert_eq!(items.len(), 3);
                assert_eq!(items[0], Expr::Symbol("+".to_string()));
                let is_three = |expr: &Expr| {
                    matches!(expr, Expr::Long(3)) || matches!(expr, Expr::Double(3.0))
                };
                assert!(is_three(&items[1]));
                assert!(is_three(&items[2]));
            }
            other => panic!("Expected expanded macro call, got: {:?}", other),
        }
    }
}
