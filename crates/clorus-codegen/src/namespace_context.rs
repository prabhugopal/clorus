/// Minimal namespace context for codegen
/// This is a simplified version for code generation purposes

use std::collections::HashMap;

/// Namespace context for code generation
#[derive(Debug, Clone)]
pub struct NamespaceContext {
    /// Current namespace
    pub current: String,

    /// Namespace aliases: alias -> full namespace
    pub aliases: HashMap<String, String>,

    /// Imported symbols: symbol -> namespace
    pub imports: HashMap<String, String>,
}

impl NamespaceContext {
    /// Create a new namespace context
    pub fn new(namespace: &str) -> Self {
        NamespaceContext {
            current: namespace.to_string(),
            aliases: HashMap::new(),
            imports: HashMap::new(),
        }
    }

    /// Create default namespace (for files without ns declaration)
    pub fn default_namespace() -> Self {
        Self::new("user")
    }

    /// Resolve a symbol name (handles qualified names and aliases)
    pub fn resolve_symbol(&self, name: &str) -> (String, String) {
        // Check for qualified symbol: prefix/symbol
        if let Some((prefix, symbol)) = name.split_once('/') {
            // Check if prefix is an alias
            if let Some(full_ns) = self.aliases.get(prefix) {
                return (full_ns.clone(), symbol.to_string());
            }
            // Otherwise treat prefix as a full namespace
            return (prefix.to_string(), symbol.to_string());
        }

        // Check imported symbols
        if let Some(ns) = self.imports.get(name) {
            return (ns.clone(), name.to_string());
        }

        // Local symbol in current namespace
        (self.current.clone(), name.to_string())
    }

    /// Get mangled name for a symbol
    /// Example: (my.app.core, add-numbers) -> clorus_my_app_core_add_numbers
    pub fn mangle_name(namespace: &str, symbol: &str) -> String {
        format!(
            "clorus_{}_{}",
            namespace.replace('.', "_"),
            symbol.replace('-', "_")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_local() {
        let ctx = NamespaceContext::new("my.app.core");
        let (ns, sym) = ctx.resolve_symbol("add");

        assert_eq!(ns, "my.app.core");
        assert_eq!(sym, "add");
    }

    #[test]
    fn test_resolve_qualified() {
        let ctx = NamespaceContext::new("my.app.core");
        let (ns, sym) = ctx.resolve_symbol("other.lib/func");

        assert_eq!(ns, "other.lib");
        assert_eq!(sym, "func");
    }

    #[test]
    fn test_resolve_with_alias() {
        let mut ctx = NamespaceContext::new("my.app.core");
        ctx.aliases.insert("lib".to_string(), "other.lib".to_string());

        let (ns, sym) = ctx.resolve_symbol("lib/func");

        assert_eq!(ns, "other.lib");
        assert_eq!(sym, "func");
    }

    #[test]
    fn test_mangle_name() {
        let mangled = NamespaceContext::mangle_name("my.app.core", "add-numbers");
        assert_eq!(mangled, "clorus_my_app_core_add_numbers");
    }
}
