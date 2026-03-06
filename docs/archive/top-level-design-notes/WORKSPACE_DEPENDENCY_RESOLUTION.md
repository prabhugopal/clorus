# Workspace Dependency Resolution (Phase 3)

> **Status:** Archived (Historical)
> **Canonical replacement:** `docs/WORKSPACE_MONOREPO_ANALYSIS.md`
>
> This document is kept for historical context. For current behavior and parity status, use the canonical document above.


**Status:** ✅ Implemented
**Date:** February 10, 2026

---

## Overview

Workspace dependency resolution allows workspace members to reference shared dependencies defined at the workspace level using `{ workspace = true }`. This eliminates duplication and ensures consistent versions across all workspace members.

## Features

### 1. Workspace Dependency References

Members can reference workspace-level dependencies:

```toml
# Root Clorus.toml
[workspace]
members = ["coral-ui", "coral-gfx", "examples/*"]

[workspace.dependencies]
coral-gfx = { path = "coral-gfx" }
```

```toml
# coral-ui/Clorus.toml
[package]
name = "coral-ui"
version = "1.0.0"

[dependencies]
# Reference workspace dependency
coral-gfx = { workspace = true }
```

### 2. Automatic Path Resolution

Relative paths in workspace dependencies are automatically resolved from the workspace root:

```toml
[workspace.dependencies]
# Relative to workspace root
some-lib = { path = "libs/some-lib-1.0.0.clip" }
```

Members referencing this with `{ workspace = true }` will get the absolute path resolved automatically.

### 3. Recursive Resolution

Workspace dependencies that themselves reference other workspace dependencies are resolved recursively:

```toml
[workspace.dependencies]
core = { path = "core" }
utils = { workspace = true }  # References another workspace dep
```

---

## Implementation Details

### Core Types

**ClorusDependency Enum:**
```rust
pub enum ClorusDependency {
    Workspace { workspace: bool },
    Path { path: String },
    Git { git: String, branch: Option<String>, ... },
    Simple(String),
}
```

### Key Methods

**1. `ClorusDependency::is_workspace()`**
- Checks if dependency is a workspace reference
- Returns `true` for `{ workspace = true }`

**2. `ClorusDependency::resolve_with_workspace()`**
- Resolves dependency using workspace context
- Looks up dependency in workspace manifest
- Resolves relative paths from workspace root
- Handles recursive resolution

**3. `Manifest::resolve_dependencies()`**
- Resolves all dependencies for a manifest
- Automatically detects workspace context
- Returns `HashMap<String, ClorusDependency>` of resolved deps

### Resolution Algorithm

```
1. Check if current directory is in a workspace
   └─ Walk up directories looking for Clorus.toml with [workspace]

2. For each dependency in manifest:
   a. If { workspace = true }:
      - Load workspace manifest
      - Look up dependency in [workspace.dependencies]
      - Recursively resolve (in case it's also workspace ref)

   b. If { path = "..." }:
      - If relative path and in workspace:
        - Resolve from workspace root
      - Else:
        - Use as-is

   c. Otherwise:
      - Return unchanged (Git, Simple, etc.)

3. Return HashMap of resolved dependencies
```

---

## Usage Examples

### Example 1: Basic Workspace Dependency

**Workspace root:**
```toml
[workspace]
members = ["lib1", "lib2", "app"]

[workspace.dependencies]
lib1 = { path = "lib1" }
lib2 = { path = "lib2" }
```

**app/Clorus.toml:**
```toml
[package]
name = "app"
version = "1.0.0"

[dependencies]
lib1 = { workspace = true }
lib2 = { workspace = true }
```

When building `app`, dependencies are automatically resolved to:
- `lib1` → `<workspace-root>/lib1`
- `lib2` → `<workspace-root>/lib2`

### Example 2: Mixed Dependencies

```toml
[workspace.dependencies]
# Workspace members
coral-gfx = { path = "coral-gfx" }

# External .clip packages
math-lib = { path = "libs/math-lib-1.0.0.clip" }
```

**Member Clorus.toml:**
```toml
[dependencies]
# Workspace member
coral-gfx = { workspace = true }

# External package (resolved from workspace root)
math-lib = { workspace = true }

# Direct dependency (not from workspace)
custom-lib = { path = "../other/custom-lib.clip" }
```

### Example 3: Recursive Resolution

```toml
[workspace.dependencies]
base = { path = "base" }
utils = { workspace = true }  # References base internally
app = { workspace = true }    # References utils internally
```

Resolution chain:
1. `app` → Look up in workspace.dependencies
2. Found `{ workspace = true }` → Recursively resolve
3. `utils` → Look up in workspace.dependencies
4. Found `{ workspace = true }` → Recursively resolve
5. `base` → Found `{ path = "base" }` → Resolve path
6. Return resolved path for `base`
7. Return resolved path for `utils` (depends on `base`)
8. Return resolved path for `app` (depends on `utils`)

---

## Integration Points

### Build System

The workspace dependency resolution is automatically used when:

1. **Loading .clip dependencies** (`pack::load_clip_dependencies`)
   - Calls `manifest.resolve_dependencies()`
   - Resolves workspace references before loading packages

2. **Building workspace members** (`commands::build_workspace`)
   - Each member's dependencies are resolved with workspace context
   - Relative paths resolved from workspace root

3. **Packaging workspace** (`commands::pack_workspace`)
   - Dependencies resolved before packaging
   - Ensures consistent references

### Error Handling

**Error Messages:**
```rust
// Missing workspace manifest
"Dependency 'foo' uses workspace = true but no workspace manifest found"

// Missing [workspace] section
"Dependency 'foo' uses workspace = true but no [workspace] section found"

// Dependency not in workspace
"Dependency 'foo' not found in workspace dependencies"
```

---

## Testing

### Test Cases

**Test 1: Basic Resolution**
```bash
# Create workspace structure
mkdir test-workspace && cd test-workspace
cat > Clorus.toml << 'EOF'
[workspace]
members = ["member1", "member2"]

[workspace.dependencies]
shared = { path = "shared" }
EOF

# member1 uses workspace dependency
mkdir member1
cat > member1/Clorus.toml << 'EOF'
[package]
name = "member1"
version = "1.0.0"

[dependencies]
shared = { workspace = true }
EOF

# Build should resolve 'shared' from workspace
cd member1
clorus build
```

**Test 2: Path Resolution**
```bash
# Workspace with external .clip
cat > Clorus.toml << 'EOF'
[workspace]
members = ["app"]

[workspace.dependencies]
external = { path = "libs/external-1.0.0.clip" }
EOF

# app references external
mkdir app
cat > app/Clorus.toml << 'EOF'
[package]
name = "app"
version = "1.0.0"

[dependencies]
external = { workspace = true }
EOF

# Should resolve to absolute path
cd app
clorus build  # Loads external from <workspace-root>/libs/external-1.0.0.clip
```

**Test 3: Error Cases**
```bash
# Member references non-existent workspace dep
[dependencies]
missing = { workspace = true }

# Should error:
# "Dependency 'missing' not found in workspace dependencies"
```

---

## Benefits

1. **DRY Principle**: Define dependency versions once at workspace level
2. **Consistency**: All members use same dependency versions
3. **Maintainability**: Update dependency version in one place
4. **Clarity**: Clear which dependencies are workspace-managed
5. **Flexibility**: Mix workspace and direct dependencies
6. **Automatic Path Resolution**: No manual path calculations needed

---

## Comparison with Cargo

Similar to Cargo's workspace dependencies:

**Cargo:**
```toml
[workspace.dependencies]
serde = "1.0"

[dependencies]
serde = { workspace = true }
```

**Clorus:**
```toml
[workspace.dependencies]
coral-ui = { path = "coral-ui" }

[dependencies]
coral-ui = { workspace = true }
```

---

## Future Enhancements

1. **Version Constraints**: Add version matching for .clip packages
2. **Registry Support**: Workspace deps from package registry
3. **Dependency Features**: Enable/disable features per member
4. **Dependency Overrides**: Allow members to override workspace versions
5. **Dependency Inheritance**: Inherit all workspace deps by default

---

## Files Modified

- **`crates/clorus-cli/src/manifest.rs`**:
  - Added `ClorusDependency::is_workspace()`
  - Added `ClorusDependency::resolve_with_workspace()`
  - Added `Manifest::resolve_dependencies()`

- **`crates/clorus-cli/src/pack.rs`**:
  - Updated `load_clip_dependencies()` to use resolved dependencies

---

## Examples in the Wild

### Coral Workspace

```toml
# coral/Clorus.toml
[workspace]
members = ["coral-gfx", "coral-ui", "examples/*"]

[workspace.dependencies]
coral-gfx = { path = "coral-gfx" }
coral-ui = { path = "coral-ui" }
```

```toml
# coral/coral-ui/Clorus.toml
[dependencies]
coral-gfx = { workspace = true }  # No need to specify path
```

```toml
# coral/examples/gallery/Clorus.toml
[dependencies]
coral-ui = { workspace = true }   # Automatically resolves
coral-gfx = { workspace = true }  # No manual path calculation
```

---

## Conclusion

Phase 3 (Workspace Dependency Resolution) is now complete. Members can reference workspace-level dependencies using `{ workspace = true }`, eliminating duplication and ensuring consistency across the workspace.

**Next Steps:**
- Test with coral workspace structure
- Add integration tests for workspace resolution
- Document in main README
