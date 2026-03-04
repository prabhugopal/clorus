/// Namespace context for symbol resolution
///
/// Handles namespace tracking, aliases, and symbol imports

use std::collections::{HashMap, HashSet};
use clorus_syntax::RequireSpec;

/// Resolved symbol with namespace information
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedSymbol {
    /// Symbol defined in current namespace
    Local {
        namespace: String,
        symbol: String,
    },
    /// Qualified symbol: namespace/symbol or alias/symbol
    Qualified {
        namespace: String,
        symbol: String,
    },
    /// Imported symbol via :refer
    Imported {
        namespace: String,
        symbol: String,
    },
}

/// Namespace context for compilation
#[derive(Debug, Clone)]
pub struct NamespaceContext {
    /// Current namespace (e.g., "my.app.core")
    pub current: String,

    /// Namespace aliases: alias -> full namespace
    /// Example: "str" -> "clorus.string"
    pub aliases: HashMap<String, String>,

    /// Imported symbols: symbol -> (namespace, original_name)
    /// Example: "sin" -> ("clorus.math", "sin")
    imports: HashMap<String, (String, String)>,

    /// Symbols defined in current namespace
    definitions: HashSet<String>,

    /// Namespaces that have :refer :all
    refer_all_namespaces: HashSet<String>,
}

impl NamespaceContext {
    /// Create a new namespace context
    pub fn new(namespace: &str) -> Self {
        NamespaceContext {
            current: namespace.to_string(),
            aliases: HashMap::new(),
            imports: HashMap::new(),
            definitions: HashSet::new(),
            refer_all_namespaces: HashSet::new(),
        }
    }

    /// Create a default namespace context (for files without ns declaration)
    pub fn default_namespace() -> Self {
        Self::new("user")
    }

    /// Set the current namespace
    pub fn set_namespace(&mut self, namespace: &str) {
        self.current = namespace.to_string();
    }

    /// Get the current namespace
    pub fn current_namespace(&self) -> &str {
        &self.current
    }

    /// Add a namespace alias
    ///
    /// Example: add_alias("str", "clorus.string")
    pub fn add_alias(&mut self, alias: &str, namespace: &str) {
        self.aliases.insert(alias.to_string(), namespace.to_string());
    }

    /// Add an imported symbol
    ///
    /// Example: add_import("sin", "clorus.math", "sin")
    pub fn add_import(&mut self, symbol: &str, namespace: &str, original_name: &str) {
        self.imports.insert(
            symbol.to_string(),
            (namespace.to_string(), original_name.to_string()),
        );
    }

    /// Mark a namespace as :refer :all
    pub fn add_refer_all(&mut self, namespace: &str) {
        self.refer_all_namespaces.insert(namespace.to_string());
    }

    /// Add a definition to current namespace
    pub fn define(&mut self, symbol: &str) {
        self.definitions.insert(symbol.to_string());
    }

    /// Check if a symbol is defined in current namespace
    pub fn is_defined(&self, symbol: &str) -> bool {
        self.definitions.contains(symbol)
    }

    /// Process a require spec and update the context
    pub fn process_require(&mut self, spec: &RequireSpec) {
        // Add alias if specified
        if let Some(alias) = &spec.alias {
            self.add_alias(alias, &spec.module);
        }

        // Add specific imports if specified
        if !spec.refer.is_empty() {
            for symbol in &spec.refer {
                self.add_import(symbol, &spec.module, symbol);
            }
        }

        for (source_symbol, local_symbol) in &spec.rename {
            self.add_import(local_symbol, &spec.module, source_symbol);
        }

        // Add :refer :all if specified
        if spec.refer_all {
            self.add_refer_all(&spec.module);
        }
    }

    /// Resolve a symbol to its full namespace and name
    ///
    /// Handles:
    /// - Qualified symbols: namespace/symbol or alias/symbol
    /// - Imported symbols via :refer
    /// - Local symbols in current namespace
    pub fn resolve_symbol(&self, name: &str) -> ResolvedSymbol {
        // Check for qualified symbol: prefix/symbol
        if let Some((prefix, symbol)) = name.split_once('/') {
            // Check if prefix is an alias
            if let Some(full_ns) = self.aliases.get(prefix) {
                return ResolvedSymbol::Qualified {
                    namespace: full_ns.clone(),
                    symbol: symbol.to_string(),
                };
            }

            // Otherwise treat prefix as a full namespace
            return ResolvedSymbol::Qualified {
                namespace: prefix.to_string(),
                symbol: symbol.to_string(),
            };
        }

        // Check imported symbols
        if let Some((ns, orig_name)) = self.imports.get(name) {
            return ResolvedSymbol::Imported {
                namespace: ns.clone(),
                symbol: orig_name.clone(),
            };
        }

        // Local symbol in current namespace
        ResolvedSymbol::Local {
            namespace: self.current.clone(),
            symbol: name.to_string(),
        }
    }

    /// Get the mangled name for a resolved symbol
    ///
    /// Example: my.app.core/add-numbers -> clorus_my_app_core_add_numbers
    pub fn mangle_symbol(resolved: &ResolvedSymbol) -> String {
        match resolved {
            ResolvedSymbol::Local { namespace, symbol }
            | ResolvedSymbol::Qualified { namespace, symbol }
            | ResolvedSymbol::Imported { namespace, symbol } => {
                format!(
                    "clorus_{}_{}",
                    namespace.replace('.', "_"),
                    symbol.replace('-', "_")
                )
            }
        }
    }

    /// Clear all imports and aliases (for REPL namespace switching)
    pub fn clear_imports(&mut self) {
        self.aliases.clear();
        self.imports.clear();
        self.refer_all_namespaces.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_symbol() {
        let ctx = NamespaceContext::new("my.app.core");
        let resolved = ctx.resolve_symbol("add");

        assert_eq!(
            resolved,
            ResolvedSymbol::Local {
                namespace: "my.app.core".to_string(),
                symbol: "add".to_string(),
            }
        );
    }

    #[test]
    fn test_qualified_symbol_with_alias() {
        let mut ctx = NamespaceContext::new("my.app.core");
        ctx.add_alias("str", "clorus.string");

        let resolved = ctx.resolve_symbol("str/join");

        assert_eq!(
            resolved,
            ResolvedSymbol::Qualified {
                namespace: "clorus.string".to_string(),
                symbol: "join".to_string(),
            }
        );
    }

    #[test]
    fn test_qualified_symbol_without_alias() {
        let ctx = NamespaceContext::new("my.app.core");
        let resolved = ctx.resolve_symbol("clorus.math/sin");

        assert_eq!(
            resolved,
            ResolvedSymbol::Qualified {
                namespace: "clorus.math".to_string(),
                symbol: "sin".to_string(),
            }
        );
    }

    #[test]
    fn test_imported_symbol() {
        let mut ctx = NamespaceContext::new("my.app.core");
        ctx.add_import("sin", "clorus.math", "sin");

        let resolved = ctx.resolve_symbol("sin");

        assert_eq!(
            resolved,
            ResolvedSymbol::Imported {
                namespace: "clorus.math".to_string(),
                symbol: "sin".to_string(),
            }
        );
    }

    #[test]
    fn test_process_require_with_alias() {
        let mut ctx = NamespaceContext::new("my.app.core");

        let spec = RequireSpec {
            module: "clorus.string".to_string(),
            alias: Some("str".to_string()),
            refer: vec![],
            refer_all: false,
            rename: vec![],
        };

        ctx.process_require(&spec);

        assert_eq!(ctx.aliases.get("str"), Some(&"clorus.string".to_string()));
    }

    #[test]
    fn test_process_require_with_refer() {
        let mut ctx = NamespaceContext::new("my.app.core");

        let spec = RequireSpec {
            module: "clorus.math".to_string(),
            alias: None,
            refer: vec!["sin".to_string(), "cos".to_string()],
            refer_all: false,
            rename: vec![],
        };

        ctx.process_require(&spec);

        assert_eq!(
            ctx.imports.get("sin"),
            Some(&("clorus.math".to_string(), "sin".to_string()))
        );
        assert_eq!(
            ctx.imports.get("cos"),
            Some(&("clorus.math".to_string(), "cos".to_string()))
        );
    }

    #[test]
    fn test_process_require_with_rename() {
        let mut ctx = NamespaceContext::new("my.app.core");

        let spec = RequireSpec {
            module: "clorus.set".to_string(),
            alias: None,
            refer: vec!["union".to_string()],
            refer_all: false,
            rename: vec![("union".to_string(), "set-union".to_string())],
        };

        ctx.process_require(&spec);

        assert_eq!(
            ctx.imports.get("set-union"),
            Some(&("clorus.set".to_string(), "union".to_string()))
        );
    }

    #[test]
    fn test_process_require_with_refer_all() {
        let mut ctx = NamespaceContext::new("my.app.core");

        let spec = RequireSpec {
            module: "clorus.test".to_string(),
            alias: None,
            refer: vec![],
            refer_all: true,
            rename: vec![],
        };

        ctx.process_require(&spec);

        assert!(ctx.refer_all_namespaces.contains("clorus.test"));
    }

    #[test]
    fn test_mangle_symbol() {
        let resolved = ResolvedSymbol::Local {
            namespace: "my.app.core".to_string(),
            symbol: "add-numbers".to_string(),
        };

        let mangled = NamespaceContext::mangle_symbol(&resolved);
        assert_eq!(mangled, "clorus_my_app_core_add_numbers");
    }

    #[test]
    fn test_definitions() {
        let mut ctx = NamespaceContext::new("my.app.core");

        assert!(!ctx.is_defined("add"));

        ctx.define("add");

        assert!(ctx.is_defined("add"));
    }
}
