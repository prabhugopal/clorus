# Linking Configuration in Clorus

## Overview

Clorus projects can specify linking requirements in `Clorus.toml` using the `[link]` section. This is particularly important for GUI applications or projects that require system frameworks and libraries.

## Configuration Format

```toml
[link]
# macOS frameworks (use -framework flag)
frameworks = ["AppKit", "OpenGL", "Carbon"]

# System libraries (use -l flag)
libraries = ["pthread", "m", "dl"]
```

## Platform-Specific Configuration

### macOS GUI Applications

GUI applications (like those using egui/eframe) require several macOS frameworks:

```toml
[package]
name = "my-gui-app"
version = "0.1.0"

[rust-dependencies]
egui-hello = { path = "../egui-hello" }

[link]
frameworks = [
    "AppKit",          # macOS UI framework
    "CoreGraphics",    # Graphics rendering
    "CoreVideo",       # Video subsystem
    "IOKit",           # Hardware I/O
    "QuartzCore",      # Core animation
    "Foundation",      # Foundation framework
    "OpenGL",          # OpenGL graphics
    "Carbon"           # Legacy APIs (keyboard/input)
]
```

**Note:** `CoreFoundation` and `Security` are always linked by default.

### Linux Applications

For Linux GUI applications, specify system libraries:

```toml
[link]
libraries = [
    "X11",        # X Window System
    "Xi",         # X Input extension
    "Xrandr",     # X Resize and Rotate
    "Xcursor",    # X Cursor management
    "GL",         # OpenGL
    "pthread"     # POSIX threads
]
```

### Windows Applications

For Windows, specify import libraries:

```toml
[link]
libraries = [
    "user32",     # User interface
    "gdi32",      # Graphics Device Interface
    "opengl32",   # OpenGL
    "dwmapi"      # Desktop Window Manager
]
```

## How It Works

1. **Build Time**: When you run `clorus build`, the build system reads the `[link]` section from `Clorus.toml`

2. **Framework Linking** (macOS):
   - Each framework in `frameworks = ["AppKit", "OpenGL"]` becomes `-framework AppKit -framework OpenGL`

3. **Library Linking** (All platforms):
   - Each library in `libraries = ["pthread", "m"]` becomes `-lpthread -lm`

4. **Automatic Deduplication**: The build system automatically removes duplicate frameworks/libraries

## Examples

### Minimal CLI Application

```toml
[package]
name = "cli-tool"
version = "0.1.0"

# No [link] section needed - defaults work fine
```

### GUI Application (Cross-platform)

```toml
[package]
name = "cross-platform-gui"
version = "0.1.0"

[rust-dependencies]
egui-hello = { path = "../egui-hello" }

[link]
# macOS
frameworks = ["AppKit", "CoreGraphics", "OpenGL"]

# Add Linux/Windows libraries here when building for those platforms
# Note: Currently only one platform can be specified per Clorus.toml
```

### Async/Networking Application

```toml
[package]
name = "async-server"
version = "0.1.0"

[rust-dependencies]
tokio = "1.0"

[link]
libraries = ["pthread"]  # POSIX threads for async runtime
```

## Migration from Hardcoded Linking

**Before** (hardcoded in clorus-cli):
- All GUI projects automatically got all frameworks
- No flexibility for minimal builds
- Unnecessary linking overhead

**After** (manifest-based):
- Projects declare only what they need
- Cleaner builds
- Better cross-platform support
- Explicit dependencies

## Troubleshooting

### Error: Undefined symbols for architecture arm64

```
ld: symbol(s) not found for architecture arm64
clang: error: linker command failed with exit code 1
```

**Solution**: Add the missing framework/library to your `Clorus.toml`:

1. Check the error for the symbol name (e.g., `_CGLErrorString`)
2. Search online for which framework provides it (e.g., OpenGL)
3. Add it to the `frameworks` list:

```toml
[link]
frameworks = ["OpenGL"]
```

### Common Symbols and Their Frameworks

| Symbol Pattern | Framework/Library | Platform |
|----------------|-------------------|----------|
| `_CGL*` | OpenGL | macOS |
| `_NS*` | AppKit or Foundation | macOS |
| `_CG*` | CoreGraphics | macOS |
| `_CV*` | CoreVideo | macOS |
| `_TIS*` | Carbon | macOS |
| `_LM*` | Carbon | macOS |
| `X*` | X11, Xi, Xrandr | Linux |
| `gl*` | GL or opengl32 | Linux/Windows |

## Benefits

1. **Flexibility**: Projects declare exactly what they need
2. **Cross-platform**: Different platforms can specify different requirements
3. **Performance**: Smaller binaries by avoiding unnecessary linking
4. **Clarity**: Explicit dependencies are easier to understand
5. **Maintainability**: No need to modify clorus-cli for new frameworks

## Future Enhancements

- Conditional linking based on target platform
- Profile-specific linking (debug vs release)
- Link-time optimization flags
- Custom linker selection
