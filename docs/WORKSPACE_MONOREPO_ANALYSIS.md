# Clorus Workspace/Monorepo Support Analysis

**Status:** Investigating multi-Clorus.toml support for root packaging
**Date:** February 10, 2026
**Context:** coral refactoring - need proper workspace structure for multiple sub-packages

---

## Current Situation

### What Works Today

1. **Single Project Structure**
   ```
   my-project/
   ├── Clorus.toml          # Single manifest
   ├── src/
   │   ├── main.clrs
   │   └── utils/
   │       └── helpers.clrs
   └── target/
   ```

2. **Multi-Module Within Single Package**
   - Projects can have nested modules (e.g., `utils.helpers`, `demos.shapes-demo`)
   - Modules loaded recursively via `(require [module.name :as alias])`
   - All modules compiled into single .o/.bc file for .clip packaging
   - Namespace isolation prevents name collisions

3. **Library Dependencies**
   - Path dependencies: `package-name = { path = "libs/package-name-1.0.0.clip" }`
   - .clip packages loaded automatically during build
   - Runtime library search is workspace-aware (walks up to find workspace root)

### What Doesn't Work (Yet)

1. **Multiple Clorus.toml Files**
   - No root Clorus.toml that manages sub-packages
   - No `[workspace]` section like Cargo
   - Each project must be built independently
   - Can't package multiple related projects together

2. **Workspace-Level Operations**
   - No `clorus build --workspace` to build all projects
   - No `clorus pack --workspace` to create multi-package distributions
   - No shared dependency resolution across workspace members

---

## Desired Workspace Structure

Based on coral's needs after refactoring:

```
coral/                           # Workspace root
├── Clorus.toml                  # WORKSPACE ROOT MANIFEST
├── coral-gfx/                   # Sub-package 1 (Rust FFI wrapper)
│   ├── Clorus.toml
│   └── interfaces/
│       └── coral-gfx.clorus-ffi
├── coral-ui/                    # Sub-package 2 (UI library)
│   ├── Clorus.toml
│   ├── src/
│   │   ├── lib.clrs
│   │   ├── widgets/
│   │   │   ├── button.clrs
│   │   │   ├── textfield.clrs
│   │   │   └── ...
│   │   └── utils/
│   │       └── constants.clrs
│   └── target/
│       └── coral-ui.o
├── coral-examples/              # Sub-package 3 (Examples)
│   ├── gallery/
│   │   ├── Clorus.toml
│   │   └── src/
│   │       ├── main.clrs
│   │       └── demos/
│   │           ├── shapes-demo.clrs
│   │           └── ...
│   └── test-app/
│       ├── Clorus.toml
│       └── src/
│           └── main.clrs
└── target/                      # WORKSPACE-LEVEL BUILD OUTPUT
    └── release/
        ├── coral-ui-1.0.0.clip
        ├── coral-gfx-1.0.0.clip
        └── gallery
```

---

## Proposed Root Clorus.toml Format

### Example 1: Workspace with Members

```toml
[workspace]
members = [
    "coral-gfx",
    "coral-ui",
    "coral-examples/gallery",
    "coral-examples/test-app"
]

# Optional: workspace-level metadata
[workspace.package]
version = "1.0.0"
authors = ["CORAL Team"]
repository = "https://github.com/your-org/coral"

# Optional: shared dependencies across workspace
[workspace.dependencies]
# coral-ui = { path = "coral-ui" }  # Other members can reference this
# coral-gfx = { path = "coral-gfx" }
```

### Example 2: Virtual Workspace (No Root Package)

```toml
[workspace]
# Virtual workspace - doesn't build anything itself
members = [
    "coral-gfx",
    "coral-ui",
    "coral-examples/*"  # Glob support
]

exclude = [
    "coral-examples/experimental",
    "scratch"
]
```

### Example 3: Workspace with Root Package

```toml
[package]
name = "coral"
version = "1.0.0"
description = "Complete CORAL graphics library suite"

[workspace]
members = [
    "coral-gfx",
    "coral-ui",
    "coral-examples/gallery"
]

[dependencies]
coral-gfx = { path = "coral-gfx" }
coral-ui = { path = "coral-ui" }
```

---

## Member Clorus.toml Changes

### coral-ui/Clorus.toml (Library)

```toml
[package]
name = "coral-ui"
version = "1.0.0"
description = "CORAL UI widget library"

[build]
# No entry = library package
# Will create coral-ui.o and coral-ui.bc

[dependencies]
# Reference workspace dependencies
coral-gfx = { path = "../coral-gfx" }
# OR use workspace: coral-gfx = { workspace = true }
```

### coral-examples/gallery/Clorus.toml (Application)

```toml
[package]
name = "coral-gallery"
version = "1.0.0"

[build]
entry = "src/main.clrs"

[dependencies]
coral-ui = { path = "../../coral-ui" }
coral-gfx = { path = "../../coral-gfx" }
# OR: coral-ui = { workspace = true }
```

---

## Implementation Plan

### Phase 1: Workspace Detection and Parsing (2 hours)

**Location:** `crates/clorus-cli/src/manifest.rs`

1. **Add workspace configuration structures**:
```rust
#[derive(Debug, Clone, Deserialize)]
pub struct WorkspaceConfig {
    /// Member package paths
    pub members: Vec<String>,
    /// Excluded paths (don't build)
    #[serde(default)]
    pub exclude: Vec<String>,
    /// Optional workspace-level package metadata
    pub package: Option<WorkspacePackage>,
    /// Shared dependencies
    #[serde(default)]
    pub dependencies: HashMap<String, ClorusDependency>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkspacePackage {
    pub version: String,
    pub authors: Vec<String>,
    pub repository: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub package: Package,
    #[serde(default)]
    pub build: Build,
    #[serde(default)]
    pub dependencies: HashMap<String, ClorusDependency>,
    #[serde(default)]
    pub link: Link,
    // NEW: workspace configuration
    pub workspace: Option<WorkspaceConfig>,
}
```

2. **Add workspace detection**:
```rust
impl Manifest {
    /// Find workspace root by walking up directories
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

            current = current.parent()?.to_path_buf();
        }
    }

    /// Check if current directory is inside a workspace
    pub fn is_in_workspace() -> bool {
        Self::find_workspace_root().is_some()
    }

    /// Load workspace manifest
    pub fn load_workspace() -> Result<(Manifest, PathBuf), String> {
        let workspace_root = Self::find_workspace_root()
            .ok_or("Not in a workspace")?;
        let manifest = Self::load(workspace_root.join("Clorus.toml"))?;
        Ok((manifest, workspace_root))
    }
}
```

3. **Add member resolution with glob support**:
```rust
impl WorkspaceConfig {
    /// Resolve all workspace members (expand globs)
    pub fn resolve_members(&self, workspace_root: &Path) -> Result<Vec<PathBuf>, String> {
        let mut members = Vec::new();

        for pattern in &self.members {
            if pattern.contains('*') {
                // Glob expansion
                let paths = glob::glob(&workspace_root.join(pattern).to_string_lossy())
                    .map_err(|e| format!("Invalid glob pattern: {}", e))?;

                for path in paths {
                    let path = path.map_err(|e| format!("Glob error: {}", e))?;
                    if !self.is_excluded(&path, workspace_root) {
                        members.push(path);
                    }
                }
            } else {
                // Exact path
                let path = workspace_root.join(pattern);
                if !self.is_excluded(&path, workspace_root) {
                    members.push(path);
                }
            }
        }

        Ok(members)
    }

    fn is_excluded(&self, path: &Path, workspace_root: &Path) -> bool {
        let relative = path.strip_prefix(workspace_root).ok();
        if let Some(rel) = relative {
            let rel_str = rel.to_string_lossy();
            return self.exclude.iter().any(|ex| {
                if ex.contains('*') {
                    // Glob match
                    glob::Pattern::new(ex).ok()
                        .map(|pat| pat.matches(&rel_str))
                        .unwrap_or(false)
                } else {
                    // Exact match
                    rel_str == ex
                }
            });
        }
        false
    }
}
```

---

### Phase 2: Workspace Commands (3 hours)

**Location:** `crates/clorus-cli/src/commands.rs`

1. **Add `--workspace` flag support**:
```rust
// In main.rs or commands module
pub fn build_workspace() -> Result<(), String> {
    let (workspace_manifest, workspace_root) = Manifest::load_workspace()?;
    let workspace_config = workspace_manifest.workspace
        .ok_or("Not a workspace")?;

    let members = workspace_config.resolve_members(&workspace_root)?;

    println!("Building workspace with {} members", members.len());

    for member_path in members {
        println!("   Building {}...", member_path.display());

        // Change to member directory
        std::env::set_current_dir(&member_path)
            .map_err(|e| format!("Failed to cd: {}", e))?;

        // Build member
        build()?;

        // Return to workspace root
        std::env::set_current_dir(&workspace_root)
            .map_err(|e| format!("Failed to cd back: {}", e))?;
    }

    println!("✅ Workspace build complete");
    Ok(())
}

pub fn clean_workspace() -> Result<(), String> {
    let (workspace_manifest, workspace_root) = Manifest::load_workspace()?;
    let workspace_config = workspace_manifest.workspace
        .ok_or("Not a workspace")?;

    let members = workspace_config.resolve_members(&workspace_root)?;

    println!("Cleaning workspace...");

    for member_path in members {
        println!("   Cleaning {}...", member_path.display());

        std::env::set_current_dir(&member_path)
            .map_err(|e| format!("Failed to cd: {}", e))?;

        clean()?;

        std::env::set_current_dir(&workspace_root)
            .map_err(|e| format!("Failed to cd back: {}", e))?;
    }

    // Clean workspace-level target directory
    let workspace_target = workspace_root.join("target");
    if workspace_target.exists() {
        fs::remove_dir_all(&workspace_target)
            .map_err(|e| format!("Failed to clean workspace target: {}", e))?;
    }

    println!("✅ Workspace cleaned");
    Ok(())
}
```

2. **Add workspace packaging**:
```rust
pub fn pack_workspace(output_dir: Option<String>) -> Result<(), String> {
    let (workspace_manifest, workspace_root) = Manifest::load_workspace()?;
    let workspace_config = workspace_manifest.workspace
        .ok_or("Not a workspace")?;

    let members = workspace_config.resolve_members(&workspace_root)?;

    // Determine output directory
    let output_dir = output_dir.unwrap_or_else(|| "dist".to_string());
    let output_path = workspace_root.join(&output_dir);
    fs::create_dir_all(&output_path)
        .map_err(|e| format!("Failed to create output dir: {}", e))?;

    println!("📦 Packaging workspace to {}", output_path.display());

    let mut packaged = Vec::new();

    for member_path in members {
        println!("   Packaging {}...", member_path.display());

        std::env::set_current_dir(&member_path)
            .map_err(|e| format!("Failed to cd: {}", e))?;

        // Load member manifest to get package name/version
        let member_manifest = Manifest::find_in_current_dir()?;
        let clip_name = format!("{}-{}.clip",
            member_manifest.package.name,
            member_manifest.package.version);

        // Pack member
        pack(Some(clip_name.clone()))?;

        // Move .clip to workspace output directory
        let clip_path = PathBuf::from(&clip_name);
        if clip_path.exists() {
            let dest = output_path.join(&clip_name);
            fs::copy(&clip_path, &dest)
                .map_err(|e| format!("Failed to copy .clip: {}", e))?;
            fs::remove_file(&clip_path)
                .map_err(|e| format!("Failed to remove .clip: {}", e))?;

            packaged.push(clip_name);
        }

        std::env::set_current_dir(&workspace_root)
            .map_err(|e| format!("Failed to cd back: {}", e))?;
    }

    println!();
    println!("✅ Packaged {} members:", packaged.len());
    for clip in &packaged {
        println!("   📦 {}", clip);
    }
    println!();
    println!("Output directory: {}", output_path.display());

    Ok(())
}
```

---

### Phase 3: Dependency Resolution (2 hours)

**Location:** `crates/clorus-cli/src/manifest.rs`

1. **Add workspace dependency support**:
```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ClorusDependency {
    Simple(String),  // version string
    Detailed {
        path: Option<String>,
        git: Option<String>,
        version: Option<String>,
        // NEW: reference workspace dependency
        workspace: Option<bool>,
    },
}

impl ClorusDependency {
    /// Resolve dependency path considering workspace
    pub fn resolve_path(&self, workspace_root: Option<&Path>) -> Option<String> {
        match self {
            ClorusDependency::Simple(_) => None,
            ClorusDependency::Detailed { path, workspace, .. } => {
                if *workspace == Some(true) {
                    // Look up in workspace dependencies
                    if let Some(ws_root) = workspace_root {
                        // TODO: Resolve from workspace.dependencies
                        None
                    } else {
                        None
                    }
                } else {
                    path.clone()
                }
            }
        }
    }
}
```

---

### Phase 4: CLI Integration (1 hour)

**Location:** `crates/clorus-cli/src/main.rs`

```rust
fn main() {
    let args: Vec<String> = std::env::args().collect();

    // ... existing commands

    // NEW: Workspace commands
    "--workspace" if cmd == "build" => {
        commands::build_workspace()
    }
    "--workspace" if cmd == "clean" => {
        commands::clean_workspace()
    }
    "--workspace" if cmd == "pack" => {
        let output_dir = args.get(3).map(|s| s.to_string());
        commands::pack_workspace(output_dir)
    }

    // ... rest of commands
}
```

---

## Usage Examples

### Building Workspace

```bash
cd coral/  # Workspace root
clorus build --workspace
# Output:
#    Building workspace with 4 members
#    Building coral-gfx...
#    Building coral-ui...
#    Building coral-examples/gallery...
#    Building coral-examples/test-app...
#    ✅ Workspace build complete
```

### Packaging Workspace

```bash
cd coral/
clorus pack --workspace
# Output:
#    📦 Packaging workspace to dist/
#    Packaging coral-gfx...
#    Packaging coral-ui...
#    ✅ Packaged 2 members:
#       📦 coral-gfx-1.0.0.clip
#       📦 coral-ui-1.0.0.clip
#    Output directory: coral/dist
```

### Building Single Member

```bash
cd coral/coral-ui/
clorus build  # Works as before
```

---

## Benefits

1. **Organized Structure**: Clear separation between libraries, examples, and tools
2. **Single Command Builds**: `clorus build --workspace` builds everything
3. **Unified Packaging**: `clorus pack --workspace` creates distribution with all .clip files
4. **Shared Dependencies**: Workspace-level dependencies reduce duplication
5. **Version Management**: Workspace-level version for related packages
6. **Cleaner Git**: Single .gitignore at workspace root
7. **Better CI/CD**: Single build command for all packages

---

## Migration Path

### Step 1: Create Workspace Root Manifest

```bash
cd coral/
cat > Clorus.toml << 'EOF'
[workspace]
members = [
    "coral-gfx",
    "coral-ui",
    "coral-examples/gallery",
    "coral-examples/test-app"
]
EOF
```

### Step 2: Update Member Dependencies

Change:
```toml
[dependencies]
coral-ui = { path = "../../coral-ui" }
```

To:
```toml
[dependencies]
coral-ui = { workspace = true }
```

### Step 3: Use Workspace Commands

```bash
clorus build --workspace
clorus pack --workspace dist/
```

---

## Implementation Timeline

| Phase | Duration | Description |
|-------|----------|-------------|
| Phase 1 | 2 hours | Workspace detection and parsing |
| Phase 2 | 3 hours | Workspace commands (build, clean, pack) |
| Phase 3 | 2 hours | Dependency resolution |
| Phase 4 | 1 hour | CLI integration |
| **Total** | **8 hours** | Full workspace support |

---

## Testing Strategy

1. **Unit Tests**: Test workspace detection, member resolution, glob expansion
2. **Integration Tests**: Build coral workspace end-to-end
3. **Migration Test**: Convert existing coral structure to workspace
4. **Backward Compatibility**: Ensure non-workspace projects still work

---

## Notes

- Workspace support is **optional** - single-project structure still works
- Implementation follows Cargo's workspace design (proven pattern)
- Glob support (`coral-examples/*`) makes managing many examples easier
- Virtual workspaces (no root package) useful for monorepos
- Workspace dependencies reduce version mismatches

---

## Current Status

**PENDING:** Waiting for coral refactoring to complete before implementing workspace support.

**Next Steps:**
1. Complete coral structure refactoring
2. Define desired workspace layout
3. Implement workspace parsing (Phase 1)
4. Add workspace commands (Phase 2-4)
5. Migrate coral to use workspace structure
