/// Clorus.toml manifest parser
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize)]
pub struct Manifest {
    pub package: Package,
    #[serde(default)]
    pub build: Build,
    #[serde(rename = "rust-dependencies", default)]
    pub rust_dependencies: HashMap<String, RustDependency>,
    #[serde(default)]
    pub dependencies: HashMap<String, ClorusDependency>,
    #[serde(default)]
    pub link: Link,
    /// Optional workspace configuration (for root Clorus.toml)
    #[serde(default)]
    pub workspace: Option<WorkspaceConfig>,
}

/// Workspace configuration (for multi-package projects)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkspaceConfig {
    /// Member package paths (supports globs like "examples/*")
    pub members: Vec<String>,
    /// Excluded paths (don't build)
    #[serde(default)]
    pub exclude: Vec<String>,
    /// Optional workspace-level package metadata
    #[serde(default)]
    pub package: Option<WorkspacePackage>,
    /// Shared dependencies (members can reference with workspace = true)
    #[serde(default)]
    pub dependencies: HashMap<String, ClorusDependency>,
}

/// Workspace-level package metadata
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkspacePackage {
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub repository: Option<String>,
}

impl WorkspaceConfig {
    /// Resolve all workspace members (expand globs)
    /// Returns absolute paths to member directories
    pub fn resolve_members(&self, workspace_root: &Path) -> Result<Vec<PathBuf>, String> {
        let mut members = Vec::new();

        for pattern in &self.members {
            if pattern.contains('*') {
                // Glob expansion
                let glob_pattern = workspace_root.join(pattern);
                let glob_str = glob_pattern.to_str()
                    .ok_or_else(|| format!("Invalid path: {}", glob_pattern.display()))?;

                match glob::glob(glob_str) {
                    Ok(paths) => {
                        for path_result in paths {
                            let path = path_result
                                .map_err(|e| format!("Glob error: {}", e))?;

                            // Only include directories that have Clorus.toml
                            if path.is_dir() && path.join("Clorus.toml").exists() {
                                if !self.is_excluded(&path, workspace_root) {
                                    members.push(path);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        return Err(format!("Invalid glob pattern '{}': {}", pattern, e));
                    }
                }
            } else {
                // Exact path
                let path = workspace_root.join(pattern);
                if path.exists() && path.is_dir() {
                    if !self.is_excluded(&path, workspace_root) {
                        members.push(path);
                    }
                } else {
                    return Err(format!("Member not found: {} (looking for {})", pattern, path.display()));
                }
            }
        }

        Ok(members)
    }

    /// Check if a path is excluded from the workspace
    fn is_excluded(&self, path: &Path, workspace_root: &Path) -> bool {
        let relative = match path.strip_prefix(workspace_root) {
            Ok(rel) => rel,
            Err(_) => return false,
        };

        let rel_str = relative.to_string_lossy();

        for exclude_pattern in &self.exclude {
            if exclude_pattern.contains('*') {
                // Glob match
                if let Ok(pattern) = glob::Pattern::new(exclude_pattern) {
                    if pattern.matches(&rel_str) {
                        return true;
                    }
                }
            } else {
                // Exact match
                if rel_str == *exclude_pattern {
                    return true;
                }
            }
        }

        false
    }
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct Link {
    /// macOS frameworks to link against (e.g., ["OpenGL", "Carbon"])
    #[serde(default)]
    pub frameworks: Vec<String>,
    /// System libraries to link against (e.g., ["pthread", "m"])
    #[serde(default)]
    pub libraries: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum ClorusDependency {
    /// Workspace dependency reference
    Workspace {
        workspace: bool,
    },
    /// Git repository with optional branch/tag/rev
    Git {
        git: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        branch: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tag: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        rev: Option<String>,
    },
    /// Local path (file system or relative path)
    Path {
        path: String,
    },
    /// Simple version string (from registry)
    Simple(String),
}

impl ClorusDependency {
    /// Get the local path if this is a path dependency
    pub fn get_path(&self) -> Option<&str> {
        match self {
            ClorusDependency::Path { path } => Some(path),
            _ => None,
        }
    }

    /// Get the git URL if this is a git dependency
    pub fn get_git(&self) -> Option<&str> {
        match self {
            ClorusDependency::Git { git, .. } => Some(git),
            _ => None,
        }
    }

    /// Get the version string if this is a registry dependency
    pub fn get_version(&self) -> Option<&str> {
        match self {
            ClorusDependency::Simple(version) => Some(version),
            _ => None,
        }
    }

    /// Get git reference (branch, tag, or rev)
    pub fn get_git_ref(&self) -> Option<GitRef> {
        match self {
            ClorusDependency::Git { branch, tag, rev, .. } => {
                if let Some(b) = branch {
                    Some(GitRef::Branch(b.clone()))
                } else if let Some(t) = tag {
                    Some(GitRef::Tag(t.clone()))
                } else if let Some(r) = rev {
                    Some(GitRef::Rev(r.clone()))
                } else {
                    Some(GitRef::Branch("main".to_string()))
                }
            }
            _ => None,
        }
    }

    /// Check if this is a workspace dependency reference
    pub fn is_workspace(&self) -> bool {
        matches!(self, ClorusDependency::Workspace { workspace: true })
    }

    /// Resolve this dependency to a concrete dependency using workspace context
    /// Returns the resolved dependency or self if no workspace resolution needed
    pub fn resolve_with_workspace(
        &self,
        dep_name: &str,
        workspace_manifest: Option<&Manifest>,
        workspace_root: Option<&Path>,
    ) -> Result<ClorusDependency, String> {
        match self {
            ClorusDependency::Workspace { workspace: true } => {
                // Look up dependency in workspace manifest
                let ws_manifest = workspace_manifest
                    .ok_or_else(|| format!(
                        "Dependency '{}' uses workspace = true but no workspace manifest found",
                        dep_name
                    ))?;

                let ws_config = ws_manifest.workspace.as_ref()
                    .ok_or_else(|| format!(
                        "Dependency '{}' uses workspace = true but no [workspace] section found",
                        dep_name
                    ))?;

                // Look up in workspace.dependencies
                let ws_dep = ws_config.dependencies.get(dep_name)
                    .ok_or_else(|| format!(
                        "Dependency '{}' not found in workspace dependencies",
                        dep_name
                    ))?;

                // Recursively resolve in case workspace dep also needs resolution
                ws_dep.resolve_with_workspace(dep_name, workspace_manifest, workspace_root)
            }
            ClorusDependency::Path { path } => {
                // If path is relative and we have workspace root, make it absolute from workspace root
                if let Some(ws_root) = workspace_root {
                    let path_buf = PathBuf::from(path);
                    if path_buf.is_relative() {
                        let absolute = ws_root.join(&path_buf);
                        Ok(ClorusDependency::Path {
                            path: absolute.to_string_lossy().to_string(),
                        })
                    } else {
                        Ok(self.clone())
                    }
                } else {
                    Ok(self.clone())
                }
            }
            _ => Ok(self.clone()),
        }
    }
}

#[derive(Debug, Clone)]
pub enum GitRef {
    Branch(String),
    Tag(String),
    Rev(String),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum RustDependency {
    WithInterface {
        path: String,
        #[serde(deserialize_with = "deserialize_interface")]
        interface: InterfaceSpec,
    },
    Path {
        path: String,
    },
    Simple(String),
}

#[derive(Debug, Clone)]
pub enum InterfaceSpec {
    Path(String),
    Auto,
}

impl serde::Serialize for InterfaceSpec {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            InterfaceSpec::Path(s) => serializer.serialize_str(s),
            InterfaceSpec::Auto => serializer.serialize_bool(true),
        }
    }
}

fn deserialize_interface<'de, D>(deserializer: D) -> Result<InterfaceSpec, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum InterfaceValue {
        Bool(bool),
        String(String),
    }

    match InterfaceValue::deserialize(deserializer)? {
        InterfaceValue::Bool(true) => Ok(InterfaceSpec::Auto),
        InterfaceValue::Bool(false) => Err(serde::de::Error::custom("interface = false not supported")),
        InterfaceValue::String(s) => Ok(InterfaceSpec::Path(s)),
    }
}

impl RustDependency {
    pub fn get_path(&self) -> Option<&str> {
        match self {
            RustDependency::WithInterface { path, .. } => Some(path),
            RustDependency::Path { path } => Some(path),
            RustDependency::Simple(_) => None,
        }
    }

    pub fn get_version(&self) -> Option<&str> {
        match self {
            RustDependency::Simple(version) => Some(version),
            _ => None,
        }
    }

    pub fn get_interface(&self) -> Option<InterfaceSpec> {
        match self {
            RustDependency::WithInterface { interface, .. } => Some(interface.clone()),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Build {
    /// Entry point for applications (e.g., "src/main.clrs")
    /// For libraries, omit this and use src/lib.clrs or lib.clrs instead
    #[serde(default)]
    pub entry: Option<String>,
    /// Multiple source directories (like deps.edn src = ["src" "resources"])
    #[serde(default)]
    pub src: Vec<String>,
    /// Whether to auto-load stdlib (core.clr + transducers.clr) during compile
    #[serde(default = "default_true")]
    pub stdlib: bool,
}

impl Default for Build {
    fn default() -> Self {
        Build {
            entry: None,  // Applications: use entry, Libraries: use src/lib.clrs
            src: Vec::new(),
            stdlib: true,
        }
    }
}

fn default_true() -> bool {
    true
}

impl Manifest {
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read Clorus.toml: {}", e))?;

        toml::from_str(&content)
            .map_err(|e| format!("Failed to parse Clorus.toml: {}", e))
    }

    pub fn load(filename: &str) -> Result<Self, String> {
        Self::from_file(Path::new(filename))
    }

    pub fn find_in_current_dir() -> Result<Self, String> {
        let manifest_path = Path::new("Clorus.toml");
        if !manifest_path.exists() {
            return Err("Could not find Clorus.toml in current directory".to_string());
        }
        Self::from_file(manifest_path)
    }

    /// Find workspace root by walking up directories
    /// Returns None if no workspace root is found
    pub fn find_workspace_root() -> Option<PathBuf> {
        let mut current = std::env::current_dir().ok()?;

        loop {
            let manifest_path = current.join("Clorus.toml");
            if manifest_path.exists() {
                if let Ok(content) = fs::read_to_string(&manifest_path) {
                    if let Ok(manifest) = toml::from_str::<Manifest>(&content) {
                        if manifest.workspace.is_some() {
                            return Some(current);
                        }
                    }
                }
            }

            // Move to parent directory
            current = current.parent()?.to_path_buf();
        }
    }

    /// Check if current directory is inside a workspace
    pub fn is_in_workspace() -> bool {
        Self::find_workspace_root().is_some()
    }

    /// Load workspace manifest
    /// Returns (manifest, workspace_root_path)
    pub fn load_workspace() -> Result<(Manifest, PathBuf), String> {
        let workspace_root = Self::find_workspace_root()
            .ok_or("Not in a workspace (no Clorus.toml with [workspace] found)")?;
        let manifest = Self::from_file(&workspace_root.join("Clorus.toml"))?;
        Ok((manifest, workspace_root))
    }

    /// Resolve all dependencies for this manifest, considering workspace context
    /// Returns a HashMap of resolved dependencies (name -> resolved ClorusDependency)
    pub fn resolve_dependencies(&self) -> Result<HashMap<String, ClorusDependency>, String> {
        let mut resolved = HashMap::new();

        // Try to load workspace context if we're in a workspace
        let workspace_context = if Self::is_in_workspace() {
            Self::load_workspace().ok()
        } else {
            None
        };

        let (workspace_manifest, workspace_root) = match &workspace_context {
            Some((manifest, root)) => (Some(manifest), Some(root.as_path())),
            None => (None, None),
        };

        // Resolve each dependency
        for (name, dep) in &self.dependencies {
            let resolved_dep = dep.resolve_with_workspace(
                name,
                workspace_manifest,
                workspace_root,
            )?;
            resolved.insert(name.clone(), resolved_dep);
        }

        Ok(resolved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_dependency_simple_version_parses() {
        let toml = r#"
[package]
name = "demo"
version = "0.1.0"

[rust-dependencies]
smol = "2.0"
"#;
        let manifest: Manifest = toml::from_str(toml).expect("manifest should parse");
        let dep = manifest
            .rust_dependencies
            .get("smol")
            .expect("smol dependency should exist");
        assert_eq!(dep.get_version(), Some("2.0"));
        assert_eq!(dep.get_path(), None);
    }

    #[test]
    fn rust_dependency_path_with_interface_parses() {
        let toml = r#"
[package]
name = "demo"
version = "0.1.0"

[rust-dependencies]
demo-lib = { path = "../demo-lib", interface = "interfaces/demo-lib.clri" }
"#;
        let manifest: Manifest = toml::from_str(toml).expect("manifest should parse");
        let dep = manifest
            .rust_dependencies
            .get("demo-lib")
            .expect("demo-lib dependency should exist");
        assert_eq!(dep.get_path(), Some("../demo-lib"));
        assert!(matches!(
            dep.get_interface(),
            Some(InterfaceSpec::Path(ref p)) if p == "interfaces/demo-lib.clri"
        ));
    }
}
