/// Clorus.toml manifest parser
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
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
    Path { path: String },
    Simple(String),
}

impl ClorusDependency {
    pub fn get_path(&self) -> &str {
        match self {
            ClorusDependency::Path { path } => path,
            ClorusDependency::Simple(path) => path,
        }
    }
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
    #[serde(default = "default_entry")]
    pub entry: String,
    /// Multiple source directories (like deps.edn src = ["src" "resources"])
    #[serde(default)]
    pub src: Vec<String>,
}

impl Default for Build {
    fn default() -> Self {
        Build {
            entry: default_entry(),
            src: Vec::new(),
        }
    }
}

fn default_entry() -> String {
    "src/main.clrs".to_string()
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
}
