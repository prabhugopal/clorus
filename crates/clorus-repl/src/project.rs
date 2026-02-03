/// Project configuration from Clorus.toml
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct ProjectConfig {
    pub package: Package,
    #[serde(default)]
    pub build: Build,
}

#[derive(Debug, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Deserialize)]
pub struct Build {
    #[serde(default = "default_entry")]
    pub entry: String,
}

impl Default for Build {
    fn default() -> Self {
        Build {
            entry: default_entry(),
        }
    }
}

fn default_entry() -> String {
    "src/main.clrs".to_string()
}

impl ProjectConfig {
    /// Try to load project config from current directory
    /// Returns None if not in a project directory
    pub fn load() -> Option<Self> {
        let manifest_path = Path::new("Clorus.toml");
        if !manifest_path.exists() {
            return None;
        }

        let content = fs::read_to_string(manifest_path).ok()?;
        toml::from_str(&content).ok()
    }

    /// Get the namespace name for this project
    /// Derives from entry point following Clojure conventions:
    /// - src/gui/demo/core.clrs → gui.demo.core
    /// - src/gui_demo/core.clrs → gui-demo.core (underscores become dashes)
    /// - src/gui-demo/core.clrs → gui-demo.core (dashes stay as dashes)
    pub fn namespace(&self) -> String {
        use std::path::Path;

        // Get the entry file path
        let entry_path = Path::new(&self.build.entry);

        // Start building namespace from path components
        let mut namespace_parts = Vec::new();

        // Iterate through path components, skipping "src" if present
        for component in entry_path.components() {
            if let Some(comp_str) = component.as_os_str().to_str() {
                // Skip "src" directory
                if comp_str == "src" {
                    continue;
                }

                // For the last component (filename), remove extension
                if component == entry_path.components().last().unwrap() {
                    if let Some(stem) = Path::new(comp_str).file_stem() {
                        if let Some(name) = stem.to_str() {
                            // Convert underscores to dashes (Clojure convention)
                            namespace_parts.push(name.replace("_", "-"));
                        }
                    }
                } else {
                    // Directory names - convert underscores to dashes
                    namespace_parts.push(comp_str.replace("_", "-"));
                }
            }
        }

        // Join with dots to form namespace
        if namespace_parts.is_empty() {
            // Fallback to package name if path parsing fails
            self.package.name.replace("_", "-")
        } else {
            namespace_parts.join(".")
        }
    }
}
