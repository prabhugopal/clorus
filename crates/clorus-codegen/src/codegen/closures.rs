use super::CodeGen;
use clorus_syntax::{Expr, Pattern};

impl<'ctx> CodeGen<'ctx> {
    /// Find free variables in an expression
    /// Returns a list of variable names that are referenced but not locally bound
    pub fn find_free_variables(&self, expr: &Expr) -> Vec<String> {
        use std::collections::HashSet;
        let mut free_vars = Vec::new();
        let mut seen = HashSet::new();
        self.collect_free_vars(expr, &mut free_vars, &mut seen, &HashSet::new());
        free_vars
    }

    /// Helper to recursively collect free variables
    pub(super) fn collect_free_vars(
        &self,
        expr: &Expr,
        free_vars: &mut Vec<String>,
        seen: &mut std::collections::HashSet<String>,
        bound: &std::collections::HashSet<String>,
    ) {
        match expr {
            Expr::Symbol(name) => {
                // If it's not bound locally and not already collected, it's free
                // Check both local variables and parameter context (for nested closures)
                if !bound.contains(name)
                    && !seen.contains(name)
                    && (self.variables.contains_key(name)
                        || self.parameter_context.contains_key(name))
                {
                    free_vars.push(name.clone());
                    seen.insert(name.clone());
                }
            }
            Expr::Let { bindings, body } => {
                // Variables bound in let are not free within the body
                let mut new_bound = bound.clone();
                for (pattern, value_expr) in bindings {
                    // Value expression can reference outer variables
                    self.collect_free_vars(value_expr, free_vars, seen, bound);
                    // Extract variable names from pattern (simplified - only handles Symbol patterns)
                    if let Pattern::Symbol(name) = pattern {
                        new_bound.insert(name.clone());
                    }
                }
                self.collect_free_vars(body, free_vars, seen, &new_bound);
            }
            Expr::Letfn { bindings, body } => {
                // Functions bound in letfn can reference each other (mutual recursion)
                let mut new_bound = bound.clone();
                // First, add all function names to bound set
                for (name, _params, _rest, _body) in bindings {
                    new_bound.insert(name.clone());
                }
                // Then collect free vars from all function bodies with all names bound
                for (_name, _params, _rest, fn_body) in bindings {
                    self.collect_free_vars(fn_body, free_vars, seen, &new_bound);
                }
                // Collect free vars from letfn body
                self.collect_free_vars(body, free_vars, seen, &new_bound);
            }
            Expr::Call { func, args } => {
                // func is a String (function name) - check if it's a free variable
                // Check both local variables and parameter context (for nested closures)
                if !bound.contains(func)
                    && !seen.contains(func)
                    && (self.variables.contains_key(func)
                        || self.parameter_context.contains_key(func))
                {
                    free_vars.push(func.clone());
                    seen.insert(func.clone());
                }
                // Check arguments for free variables
                for arg in args {
                    self.collect_free_vars(arg, free_vars, seen, bound);
                }
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.collect_free_vars(condition, free_vars, seen, bound);
                self.collect_free_vars(then_branch, free_vars, seen, bound);
                self.collect_free_vars(else_branch, free_vars, seen, bound);
            }
            Expr::Vector(elements) => {
                for elem in elements {
                    self.collect_free_vars(elem, free_vars, seen, bound);
                }
            }
            Expr::List(elements) => {
                for elem in elements {
                    self.collect_free_vars(elem, free_vars, seen, bound);
                }
            }
            Expr::Map(pairs) => {
                for (key, val) in pairs {
                    self.collect_free_vars(key, free_vars, seen, bound);
                    self.collect_free_vars(val, free_vars, seen, bound);
                }
            }
            Expr::Fn {
                params,
                rest_param,
                body,
            } => {
                // Nested function - parameters are bound within the function body
                let mut new_bound = bound.clone();
                for param in params {
                    if let Pattern::Symbol(name) = param {
                        new_bound.insert(name.clone());
                    }
                }
                if let Some(rest_name) = rest_param {
                    new_bound.insert(rest_name.clone());
                }
                // Collect free vars from body with new bound set
                self.collect_free_vars(body, free_vars, seen, &new_bound);
            }
            Expr::FnMulti { arities } => {
                // Multi-arity nested function - parameters from all arities are bound
                let mut new_bound = bound.clone();
                for arity in arities {
                    for param in &arity.params {
                        if let Pattern::Symbol(name) = param {
                            new_bound.insert(name.clone());
                        }
                    }
                    if let Some(rest_name) = &arity.rest_param {
                        new_bound.insert(rest_name.clone());
                    }
                }
                // Collect free vars from all arity bodies
                for arity in arities {
                    self.collect_free_vars(&arity.body, free_vars, seen, &new_bound);
                }
            }
            Expr::Set(elements) => {
                for elem in elements {
                    self.collect_free_vars(elem, free_vars, seen, bound);
                }
            }
            Expr::Do { exprs } | Expr::Dosync { exprs } => {
                for e in exprs {
                    self.collect_free_vars(e, free_vars, seen, bound);
                }
            }
            Expr::Recur { args } => {
                for arg in args {
                    self.collect_free_vars(arg, free_vars, seen, bound);
                }
            }
            Expr::Loop { bindings, body } => {
                // Same shape as Let: binding values see the outer scope, the loop
                // body sees the loop-bound names added on top of it.
                let mut new_bound = bound.clone();
                for (pattern, value_expr) in bindings {
                    self.collect_free_vars(value_expr, free_vars, seen, bound);
                    if let Pattern::Symbol(name) = pattern {
                        new_bound.insert(name.clone());
                    }
                }
                self.collect_free_vars(body, free_vars, seen, &new_bound);
            }
            Expr::Binding { bindings, body } => {
                // (binding [*dyn-var* val] body) - *dyn-var* names a global dynamic
                // var, not a local binding, so it never enters `bound`; only the
                // rebind value expressions and the body can reference free vars.
                for (_var_name, value_expr) in bindings {
                    self.collect_free_vars(value_expr, free_vars, seen, bound);
                }
                self.collect_free_vars(body, free_vars, seen, bound);
            }
            Expr::SetBang { target, value } => {
                // `target` is itself a variable reference being mutated.
                if !bound.contains(target)
                    && !seen.contains(target)
                    && (self.variables.contains_key(target)
                        || self.parameter_context.contains_key(target))
                {
                    free_vars.push(target.clone());
                    seen.insert(target.clone());
                }
                self.collect_free_vars(value, free_vars, seen, bound);
            }
            Expr::Deref { expr }
            | Expr::Throw { expr } => {
                self.collect_free_vars(expr, free_vars, seen, bound);
            }
            Expr::Try {
                body,
                catch_clauses,
                finally_block,
            } => {
                self.collect_free_vars(body, free_vars, seen, bound);
                for clause in catch_clauses {
                    let mut new_bound = bound.clone();
                    new_bound.insert(clause.binding.clone());
                    self.collect_free_vars(&clause.handler, free_vars, seen, &new_bound);
                }
                if let Some(finally_expr) = finally_block {
                    self.collect_free_vars(finally_expr, free_vars, seen, bound);
                }
            }
            Expr::Def { name: _, value, metadata } => {
                self.collect_free_vars(value, free_vars, seen, bound);
                if let Some(pairs) = metadata {
                    for (k, v) in pairs {
                        self.collect_free_vars(k, free_vars, seen, bound);
                        self.collect_free_vars(v, free_vars, seen, bound);
                    }
                }
            }
            Expr::Defn {
                params,
                rest_param,
                body,
                metadata,
                ..
            } => {
                let mut new_bound = bound.clone();
                for param in params {
                    if let Pattern::Symbol(name) = param {
                        new_bound.insert(name.clone());
                    }
                }
                if let Some(rest_name) = rest_param {
                    new_bound.insert(rest_name.clone());
                }
                self.collect_free_vars(body, free_vars, seen, &new_bound);
                if let Some(pairs) = metadata {
                    for (k, v) in pairs {
                        self.collect_free_vars(k, free_vars, seen, bound);
                        self.collect_free_vars(v, free_vars, seen, bound);
                    }
                }
            }
            Expr::DefnMulti { arities, metadata, .. } => {
                let mut new_bound = bound.clone();
                for arity in arities {
                    for param in &arity.params {
                        if let Pattern::Symbol(name) = param {
                            new_bound.insert(name.clone());
                        }
                    }
                    if let Some(rest_name) = &arity.rest_param {
                        new_bound.insert(rest_name.clone());
                    }
                }
                for arity in arities {
                    self.collect_free_vars(&arity.body, free_vars, seen, &new_bound);
                }
                if let Some(pairs) = metadata {
                    for (k, v) in pairs {
                        self.collect_free_vars(k, free_vars, seen, bound);
                        self.collect_free_vars(v, free_vars, seen, bound);
                    }
                }
            }
            Expr::Defmacro {
                params,
                rest_param,
                body,
                ..
            } => {
                let mut new_bound = bound.clone();
                for name in params {
                    new_bound.insert(name.clone());
                }
                if let Some(rest_name) = rest_param {
                    new_bound.insert(rest_name.clone());
                }
                self.collect_free_vars(body, free_vars, seen, &new_bound);
            }
            Expr::Defmulti {
                dispatch_fn,
                metadata,
                ..
            } => {
                self.collect_free_vars(dispatch_fn, free_vars, seen, bound);
                if let Some(pairs) = metadata {
                    for (k, v) in pairs {
                        self.collect_free_vars(k, free_vars, seen, bound);
                        self.collect_free_vars(v, free_vars, seen, bound);
                    }
                }
            }
            Expr::Defmethod {
                dispatch_value,
                params,
                body,
                ..
            } => {
                self.collect_free_vars(dispatch_value, free_vars, seen, bound);
                let mut new_bound = bound.clone();
                for param in params {
                    if let Pattern::Symbol(name) = param {
                        new_bound.insert(name.clone());
                    }
                }
                self.collect_free_vars(body, free_vars, seen, &new_bound);
            }
            Expr::PreferMethod {
                preferred_dispatch,
                over_dispatch,
                ..
            } => {
                self.collect_free_vars(preferred_dispatch, free_vars, seen, bound);
                self.collect_free_vars(over_dispatch, free_vars, seen, bound);
            }
            Expr::RemoveMethod { dispatch_value, .. } => {
                self.collect_free_vars(dispatch_value, free_vars, seen, bound);
            }
            Expr::ExtendType { methods, .. } => {
                for method in methods {
                    let mut new_bound = bound.clone();
                    for param in &method.params {
                        if let Pattern::Symbol(name) = param {
                            new_bound.insert(name.clone());
                        }
                    }
                    self.collect_free_vars(&method.body, free_vars, seen, &new_bound);
                }
            }
            Expr::Defrecord { protocols, .. } | Expr::Deftype { protocols, .. } => {
                for (_protocol_name, methods) in protocols {
                    for method in methods {
                        let mut new_bound = bound.clone();
                        for param in &method.params {
                            if let Pattern::Symbol(name) = param {
                                new_bound.insert(name.clone());
                            }
                        }
                        self.collect_free_vars(&method.body, free_vars, seen, &new_bound);
                    }
                }
            }
            // Quoted data is never evaluated as-is, so plain symbols inside it are
            // not variable references. Only `~unquote`/`~@unquote-splicing` forms
            // inside a syntax-quote are actually evaluated in the enclosing scope.
            Expr::Quote { .. } => {}
            Expr::SyntaxQuote { expr } => {
                self.collect_free_vars_in_quoted(expr, free_vars, seen, bound);
            }
            Expr::Unquote { expr } | Expr::UnquoteSplicing { expr } => {
                // Only reachable directly here for malformed unquotes outside a
                // syntax-quote; still correct to treat the inner expr as live code.
                self.collect_free_vars(expr, free_vars, seen, bound);
            }
            // Compile-time-only / no embedded runtime expressions: nothing to capture.
            Expr::Long(_)
            | Expr::Double(_)
            | Expr::String(_)
            | Expr::Keyword(_)
            | Expr::Bool(_)
            | Expr::Nil
            | Expr::Declare { .. }
            | Expr::Var { .. }
            | Expr::Use { .. }
            | Expr::Ns { .. }
            | Expr::Require { .. }
            | Expr::Defprotocol { .. } => {}
        }
    }

    /// Like `collect_free_vars`, but for the contents of a syntax-quoted form:
    /// everything is inert data except `~x`/`~@x`, which are evaluated in the
    /// enclosing scope and spliced in, so only those can reference free variables.
    fn collect_free_vars_in_quoted(
        &self,
        expr: &Expr,
        free_vars: &mut Vec<String>,
        seen: &mut std::collections::HashSet<String>,
        bound: &std::collections::HashSet<String>,
    ) {
        match expr {
            Expr::Unquote { expr } | Expr::UnquoteSplicing { expr } => {
                self.collect_free_vars(expr, free_vars, seen, bound);
            }
            Expr::List(elements) | Expr::Vector(elements) | Expr::Set(elements) => {
                for e in elements {
                    self.collect_free_vars_in_quoted(e, free_vars, seen, bound);
                }
            }
            Expr::Map(pairs) => {
                for (k, v) in pairs {
                    self.collect_free_vars_in_quoted(k, free_vars, seen, bound);
                    self.collect_free_vars_in_quoted(v, free_vars, seen, bound);
                }
            }
            // A nested syntax-quote raises quote depth in real Clojure, but Clorus
            // macros are rarely nested this deeply; scanning through for unquotes
            // is the conservative, capture-more-not-less choice.
            Expr::SyntaxQuote { expr } => {
                self.collect_free_vars_in_quoted(expr, free_vars, seen, bound);
            }
            // Anything else appearing inside quoted data (symbols, keywords,
            // literals, nested plain quotes, ...) is just data, never evaluated.
            _ => {}
        }
    }
}
