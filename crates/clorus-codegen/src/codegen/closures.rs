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
            // Literals and other non-recursive cases
            _ => {}
        }
    }
}
