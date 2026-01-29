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
    /// Derives from entry point: src/main.clrs → my_app.main
    pub fn namespace(&self) -> String {
        use std::path::Path;

        // Get the entry file path
        let entry_path = Path::new(&self.build.entry);

        // Extract the module name from the file (without extension)
        let module_name = entry_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("main");

        // Replace hyphens with underscores for valid namespace
        let sanitized_package = self.package.name.replace("-", "_");
        let sanitized_module = module_name.replace("-", "_");

        format!("{}.{}", sanitized_package, sanitized_module)
    }
}
