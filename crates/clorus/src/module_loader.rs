/// Module loading system for Clorus namespaces
///
/// Converts namespace names to file paths and loads .clrs files
/// Example: my.app.core -> src/my/app/core.clrs

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use clorus_syntax::Expr;

/// Module loader with caching
pub struct ModuleLoader {
    /// Search paths for modules (e.g., ["src", "stdlib", "."])
    search_paths: Vec<PathBuf>,

    /// Cache of loaded modules: namespace -> parsed expressions
    cache: HashMap<String, Vec<Expr>>,

    /// Track loading modules to detect circular dependencies
    loading: Vec<String>,
}

impl ModuleLoader {
    /// Create a new module loader with default search paths
    pub fn new() -> Self {
        ModuleLoader {
            search_paths: vec![
                PathBuf::from("src"),
                PathBuf::from("."),
            ],
            cache: HashMap::new(),
            loading: Vec::new(),
        }
    }

    /// Create a module loader with custom search paths
    pub fn with_search_paths(paths: Vec<PathBuf>) -> Self {
        ModuleLoader {
            search_paths: paths,
            cache: HashMap::new(),
            loading: Vec::new(),
        }
    }

    /// Add a search path for modules
    pub fn add_search_path(&mut self, path: PathBuf) {
        if !self.search_paths.contains(&path) {
            self.search_paths.push(path);
        }
    }

    /// Load a module by namespace name
    ///
    /// Returns a reference to the cached parsed expressions
    pub fn load_module(&mut self, namespace: &str) -> Result<&Vec<Expr>, String> {
        // Check if already loaded
        if self.cache.contains_key(namespace) {
            return Ok(self.cache.get(namespace).unwrap());
        }

        // Check for circular dependency
        if self.loading.contains(&namespace.to_string()) {
            return Err(format!(
                "Circular dependency detected while loading: {}",
                namespace
            ));
        }

        // Mark as loading
        self.loading.push(namespace.to_string());

        // Find and parse the module
        let result = self.load_and_parse(namespace);

        // Remove from loading list
        self.loading.pop();

        // Handle result
        match result {
            Ok(exprs) => {
                self.cache.insert(namespace.to_string(), exprs);
                Ok(self.cache.get(namespace).unwrap())
            }
            Err(e) => Err(e),
        }
    }

    /// Load and parse a module file
    fn load_and_parse(&self, namespace: &str) -> Result<Vec<Expr>, String> {
        let file_path = self.find_module_file(namespace)?;

        // Read the file
        let source = fs::read_to_string(&file_path)
            .map_err(|e| format!("Failed to read {}: {}", file_path.display(), e))?;

        // Parse the source and expand macros
        clorus_syntax::parse_and_expand(&source)
            .map_err(|e| format!("Parse error in {}: {}", file_path.display(), e))
    }

    /// Find the file path for a namespace
    ///
    /// Example: my.app.core -> src/my/app/core.clrs
    fn find_module_file(&self, namespace: &str) -> Result<PathBuf, String> {
        // Convert namespace to relative path
        // my.app.core -> my/app/core.clrs
        let rel_path = self.namespace_to_path(namespace);

        // Try each search path
        for search_path in &self.search_paths {
            let full_path = search_path.join(&rel_path);
            if full_path.exists() && full_path.is_file() {
                return Ok(full_path);
            }
        }

        Err(format!(
            "Module not found: {} (searched {:?} for {})",
            namespace,
            self.search_paths,
            rel_path.display()
        ))
    }

    /// Convert namespace to relative file path
    ///
    /// my.app.core -> my/app/core.clrs
    fn namespace_to_path(&self, namespace: &str) -> PathBuf {
        let mut path = PathBuf::new();

        // Split on dots and add as path components
        for part in namespace.split('.') {
            path.push(part);
        }

        // Add .clrs extension
        let mut path_str = path.to_string_lossy().to_string();
        path_str.push_str(".clrs");

        PathBuf::from(path_str)
    }

    /// Check if a module is already loaded
    pub fn is_loaded(&self, namespace: &str) -> bool {
        self.cache.contains_key(namespace)
    }

    /// Get a loaded module (returns None if not loaded)
    pub fn get_module(&self, namespace: &str) -> Option<&Vec<Expr>> {
        self.cache.get(namespace)
    }

    /// Clear the module cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Get list of loaded module names
    pub fn loaded_modules(&self) -> Vec<String> {
        self.cache.keys().cloned().collect()
    }
}

impl Default for ModuleLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_namespace_to_path() {
        let loader = ModuleLoader::new();

        assert_eq!(
            loader.namespace_to_path("my.app.core"),
            PathBuf::from("my/app/core.clrs")
        );

        assert_eq!(
            loader.namespace_to_path("simple"),
            PathBuf::from("simple.clrs")
        );

        assert_eq!(
            loader.namespace_to_path("a.b.c.d"),
            PathBuf::from("a/b/c/d.clrs")
        );
    }

    #[test]
    fn test_search_paths() {
        let mut loader = ModuleLoader::new();

        assert_eq!(loader.search_paths.len(), 2);
        assert!(loader.search_paths.contains(&PathBuf::from("src")));

        loader.add_search_path(PathBuf::from("lib"));
        assert_eq!(loader.search_paths.len(), 3);

        // Adding duplicate doesn't increase count
        loader.add_search_path(PathBuf::from("lib"));
        assert_eq!(loader.search_paths.len(), 3);
    }

    #[test]
    fn test_cache() {
        let mut loader = ModuleLoader::new();

        assert!(!loader.is_loaded("test.module"));
        assert_eq!(loader.loaded_modules().len(), 0);

        // Manually add to cache for testing
        loader.cache.insert("test.module".to_string(), vec![]);

        assert!(loader.is_loaded("test.module"));
        assert_eq!(loader.loaded_modules().len(), 1);
        assert!(loader.loaded_modules().contains(&"test.module".to_string()));
    }
}
