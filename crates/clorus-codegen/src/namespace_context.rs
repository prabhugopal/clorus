/// Minimal namespace context for codegen
/// This is a simplified version for code generation purposes
use std::collections::HashMap;

/// Binding for an imported symbol from another namespace.
#[derive(Debug, Clone)]
pub struct ImportBinding {
    /// Source namespace, e.g. clorus.set
    pub namespace: String,
    /// Source symbol name in that namespace, e.g. union
    pub symbol: String,
}

/// Namespace context for code generation
#[derive(Debug, Clone)]
pub struct NamespaceContext {
    /// Current namespace
    pub current: String,

    /// Namespace aliases: alias -> full namespace
    pub aliases: HashMap<String, String>,

    /// Imported symbols: local symbol -> source binding
    pub imports: HashMap<String, ImportBinding>,
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
        if let Some(binding) = self.imports.get(name) {
            return (binding.namespace.clone(), binding.symbol.clone());
        }

        // Local symbol in current namespace
        (self.current.clone(), name.to_string())
    }

    /// Register an alias (e.g. `:as s`) with conflict checking.
    pub fn register_alias(&mut self, alias: String, module: String) -> Result<(), String> {
        if let Some(existing) = self.aliases.get(&alias) {
            if existing == &module {
                // Idempotent registration.
                return Ok(());
            }
            return Err(format!(
                "Namespace alias conflict: '{}' already maps to '{}', cannot remap to '{}'",
                alias, existing, module
            ));
        }
        self.aliases.insert(alias, module);
        Ok(())
    }

    /// Register a local imported symbol with conflict checking.
    pub fn register_import(
        &mut self,
        local_symbol: String,
        namespace: String,
        source_symbol: String,
    ) -> Result<(), String> {
        if let Some(existing) = self.imports.get(&local_symbol) {
            if existing.namespace == namespace && existing.symbol == source_symbol {
                // Idempotent registration.
                return Ok(());
            }
            return Err(format!(
                "Namespace import conflict: '{}' already refers to '{}/{}', cannot also refer to '{}/{}'",
                local_symbol,
                existing.namespace,
                existing.symbol,
                namespace,
                source_symbol
            ));
        }

        self.imports.insert(
            local_symbol,
            ImportBinding {
                namespace,
                symbol: source_symbol,
            },
        );
        Ok(())
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
        ctx.aliases
            .insert("lib".to_string(), "other.lib".to_string());

        let (ns, sym) = ctx.resolve_symbol("lib/func");

        assert_eq!(ns, "other.lib");
        assert_eq!(sym, "func");
    }

    #[test]
    fn test_mangle_name() {
        let mangled = NamespaceContext::mangle_name("my.app.core", "add-numbers");
        assert_eq!(mangled, "clorus_my_app_core_add_numbers");
    }

    #[test]
    fn test_resolve_imported_rename() {
        let mut ctx = NamespaceContext::new("my.app.core");
        ctx.imports.insert(
            "set-union".to_string(),
            ImportBinding {
                namespace: "clorus.set".to_string(),
                symbol: "union".to_string(),
            },
        );

        let (ns, sym) = ctx.resolve_symbol("set-union");
        assert_eq!(ns, "clorus.set");
        assert_eq!(sym, "union");
    }

    #[test]
    fn test_register_alias_conflict() {
        let mut ctx = NamespaceContext::new("my.app.core");
        ctx.register_alias("s".to_string(), "clorus.set".to_string())
            .unwrap();

        let err = ctx
            .register_alias("s".to_string(), "other.set".to_string())
            .unwrap_err();
        assert!(err.contains("Namespace alias conflict"));
    }

    #[test]
    fn test_register_import_conflict() {
        let mut ctx = NamespaceContext::new("my.app.core");
        ctx.register_import(
            "union".to_string(),
            "clorus.set".to_string(),
            "union".to_string(),
        )
        .unwrap();

        let err = ctx
            .register_import(
                "union".to_string(),
                "other.set".to_string(),
                "union".to_string(),
            )
            .unwrap_err();
        assert!(err.contains("Namespace import conflict"));
    }
}
