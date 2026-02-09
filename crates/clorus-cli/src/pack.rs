/// Packaging support for creating .clip (Clorus Library Package) files
///
/// A .clip file is a ZIP archive containing:
/// - clip.toml (package metadata)
/// - lib/*.bc (LLVM bitcode - portable)
/// - lib/*.o (native object - platform-specific)
/// - api/exports.json (public API definitions)

use std::fs::{self, File};
use std::io::{Write, Read};
use std::path::{Path, PathBuf};
use crate::manifest::{Manifest, ClorusDependency};
use serde_json::{json, Value};

/// Data extracted from a .clip package
pub struct ClipPackage {
    pub name: String,
    pub version: String,
    pub bitcode_path: Option<PathBuf>,
    pub object_path: Option<PathBuf>,
    pub exports: Value,
    pub dylib_cache_path: Option<PathBuf>,  // Cached .dylib for REPL
}

/// Build a dynamic library from .clip package for REPL use
/// Returns path to the created .dylib file
pub fn build_dylib_for_repl(package: &ClipPackage) -> Result<PathBuf, String> {
    // Determine cache directory (.repl/ in current directory)
    let cache_dir = PathBuf::from(".repl").join(&package.name);
    fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("Failed to create .repl cache directory: {}", e))?;

    // Determine dynamic library name based on platform
    #[cfg(target_os = "macos")]
    let dylib_name = format!("lib{}.dylib", package.name.replace('-', "_"));

    #[cfg(target_os = "linux")]
    let dylib_name = format!("lib{}.so", package.name.replace('-', "_"));

    #[cfg(target_os = "windows")]
    let dylib_name = format!("{}.dll", package.name.replace('-', "_"));

    let dylib_path = cache_dir.join(&dylib_name);

    // Check if already cached and fresh
    if dylib_path.exists() {
        // TODO: Check timestamp against source
        return Ok(dylib_path);
    }

    // Get object file from package
    let object_path = package.object_path.as_ref()
        .ok_or_else(|| format!("No object file in .clip package: {}", package.name))?;

    if !object_path.exists() {
        return Err(format!("Object file not found: {}", object_path.display()));
    }

    println!("      Building {} for REPL...", package.name);

    // Find runtime library
    let runtime_lib = find_runtime_library()?;

    // Link as dynamic library
    let mut link_cmd = std::process::Command::new("cc");
    link_cmd
        .arg("-shared")                    // Create dynamic library
        .arg(object_path)                  // Input object file
        .arg(runtime_lib)                  // Runtime library
        .arg("-o").arg(&dylib_path);       // Output dynamic library

    // Add C++ standard library (for LLVM runtime)
    link_cmd.arg("-lc++");

    // Platform-specific flags
    #[cfg(target_os = "macos")]
    {
        link_cmd.arg("-dynamiclib");
        link_cmd.arg("-framework").arg("CoreFoundation");
        link_cmd.arg("-framework").arg("Security");
    }

    let status = link_cmd
        .status()
        .map_err(|e| format!("Failed to run linker: {}", e))?;

    if !status.success() {
        return Err(format!("Failed to link dynamic library for {}", package.name));
    }

    println!("         ✓ Cached at {}", dylib_path.display());

    Ok(dylib_path)
}

/// Find runtime library for linking
fn find_runtime_library() -> Result<String, String> {
    // Search for libclorus_runtime.a
    let search_paths = vec![
        "target/release/libclorus_runtime.a",
        "target/debug/libclorus_runtime.a",
        "../target/release/libclorus_runtime.a",
        "../target/debug/libclorus_runtime.a",
        "../../target/release/libclorus_runtime.a",
        "../../target/debug/libclorus_runtime.a",
    ];

    for path in search_paths {
        if Path::new(path).exists() {
            return Ok(path.to_string());
        }
    }

    // Try relative to clorus executable
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let runtime_path = exe_dir.join("../lib/libclorus_runtime.a");
            if runtime_path.exists() {
                return Ok(runtime_path.to_string_lossy().to_string());
            }
        }
    }

    Err("Runtime library not found. Run: cargo build -p clorus-runtime --release".to_string())
}

/// Extract and parse a .clip package to a temporary directory
pub fn extract_clip(clip_path: &str) -> Result<ClipPackage, String> {
    let clip_path = Path::new(clip_path);
    if !clip_path.exists() {
        return Err(format!(".clip file not found: {}", clip_path.display()));
    }

    // Create temp directory for extraction
    let temp_dir = std::env::temp_dir().join(format!(
        "clorus-clip-{}",
        clip_path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
    ));

    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)
            .map_err(|e| format!("Failed to clean temp dir: {}", e))?;
    }
    fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create temp dir: {}", e))?;

    // Extract ZIP archive
    let file = File::open(clip_path)
        .map_err(|e| format!("Failed to open .clip file: {}", e))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("Failed to read .clip archive: {}", e))?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)
            .map_err(|e| format!("Failed to read archive entry: {}", e))?;
        let outpath = temp_dir.join(file.name());

        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        } else {
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create parent dir: {}", e))?;
            }
            let mut outfile = File::create(&outpath)
                .map_err(|e| format!("Failed to create file: {}", e))?;
            std::io::copy(&mut file, &mut outfile)
                .map_err(|e| format!("Failed to extract file: {}", e))?;
        }
    }

    // Read clip.toml
    let clip_toml_path = temp_dir.join("clip.toml");
    let clip_toml_content = fs::read_to_string(&clip_toml_path)
        .map_err(|e| format!("Failed to read clip.toml: {}", e))?;
    let clip_manifest: toml::Value = toml::from_str(&clip_toml_content)
        .map_err(|e| format!("Failed to parse clip.toml: {}", e))?;

    let name = clip_manifest.get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .ok_or("Missing package name in clip.toml")?
        .to_string();

    let version = clip_manifest.get("package")
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .ok_or("Missing package version in clip.toml")?
        .to_string();

    // Read api/exports.json
    let exports_path = temp_dir.join("api/exports.json");
    let exports_content = fs::read_to_string(&exports_path)
        .map_err(|e| format!("Failed to read exports.json: {}", e))?;
    let exports: Value = serde_json::from_str(&exports_content)
        .map_err(|e| format!("Failed to parse exports.json: {}", e))?;

    // Find bitcode and object files
    let lib_dir = temp_dir.join("lib");
    let mut bitcode_path = None;
    let mut object_path = None;

    if lib_dir.exists() {
        for entry in fs::read_dir(&lib_dir)
            .map_err(|e| format!("Failed to read lib directory: {}", e))? {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let path = entry.path();

            if let Some(ext) = path.extension() {
                if ext == "bc" {
                    bitcode_path = Some(path);
                } else if ext == "o" {
                    object_path = Some(path);
                }
            }
        }
    }

    Ok(ClipPackage {
        name,
        version,
        bitcode_path,
        object_path,
        exports,
        dylib_cache_path: None,  // Will be set when built for REPL
    })
}

/// Load all .clip dependencies from manifest
pub fn load_clip_dependencies(manifest: &Manifest) -> Result<Vec<ClipPackage>, String> {
    let mut packages = Vec::new();

    for (name, dep) in &manifest.dependencies {
        if let Some(path) = dep.get_path() {
            if path.ends_with(".clip") {
                println!("   📦 Loading dependency: {} from {}", name, path);
                let package = extract_clip(path)?;
                packages.push(package);
            }
        }
        // TODO: Handle Git and Simple (registry) dependencies
    }

    Ok(packages)
}

/// Create a .clip package from the current project
pub fn pack(output: Option<String>) -> Result<(), String> {
    println!("📦 Packaging Clorus library...");

    // 1. Load Clorus.toml
    let manifest = Manifest::load("Clorus.toml")
        .map_err(|e| format!("Failed to load Clorus.toml: {}", e))?;

    let package_name = manifest.package.name.clone();
    let package_version = manifest.package.version.clone();

    // 2. Determine output filename
    let output_filename = output.unwrap_or_else(|| {
        format!("{}-{}.clip", package_name, package_version)
    });

    println!("   📝 Package: {} v{}", package_name, package_version);
    println!("   📄 Output: {}", output_filename);

    // 3. Build the project first to get compiled artifacts (in library mode)
    println!("   🔨 Building project...");
    crate::commands::build_lib()?;

    // 4. Create temporary directory for .clip contents
    let temp_dir = PathBuf::from(format!(".clip-build-{}", package_name));
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)
            .map_err(|e| format!("Failed to clean temp dir: {}", e))?;
    }
    fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create temp dir: {}", e))?;

    fs::create_dir_all(temp_dir.join("lib"))
        .map_err(|e| format!("Failed to create lib dir: {}", e))?;
    fs::create_dir_all(temp_dir.join("api"))
        .map_err(|e| format!("Failed to create api dir: {}", e))?;

    // 5. Create clip.toml metadata
    println!("   📋 Creating metadata...");
    create_clip_toml(&manifest, &temp_dir)?;

    // 6. Copy compiled artifacts (.o and .bc files if they exist)
    println!("   📦 Packaging compiled artifacts...");
    copy_artifacts(&package_name, &temp_dir)?;

    // 7. Extract API exports
    println!("   🔍 Extracting API exports...");
    create_exports_json(&package_name, &temp_dir)?;

    // 8. Create ZIP archive
    println!("   🗜️  Creating .clip archive...");
    create_zip_archive(&temp_dir, &output_filename)?;

    // 9. Cleanup temp directory
    fs::remove_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to cleanup temp dir: {}", e))?;

    println!("✅ Successfully created {}", output_filename);
    println!();
    println!("You can now distribute this .clip file and install it with:");
    println!("    clorus install {}", output_filename);

    Ok(())
}

/// Create clip.toml metadata file
fn create_clip_toml(manifest: &Manifest, temp_dir: &Path) -> Result<(), String> {
    let content = format!(
        r#"[package]
name = "{}"
version = "{}"
authors = {:?}
description = "{}"

[build]
{}

[api]
# See api/exports.json for detailed function signatures
"#,
        manifest.package.name,
        manifest.package.version,
        manifest.package.authors,
        manifest.package.description.as_deref().unwrap_or(""),
        // Only include entry line if it's specified
        manifest.build.entry.as_ref()
            .map(|e| format!("entry = \"{}\"", e))
            .unwrap_or_else(|| "# No entry - this is a library".to_string())
    );

    fs::write(temp_dir.join("clip.toml"), content)
        .map_err(|e| format!("Failed to write clip.toml: {}", e))
}

/// Copy compiled artifacts (.o and .bc files) to temp directory
fn copy_artifacts(package_name: &str, temp_dir: &Path) -> Result<(), String> {
    // Look for compiled .o file in target directory
    let target_output_path = PathBuf::from("target").join(format!("{}.o", package_name));
    let output_path = Path::new("output.o");

    let source_path = if target_output_path.exists() {
        target_output_path
    } else if output_path.exists() {
        output_path.to_path_buf()
    } else {
        return Err(format!(
            "No compiled object file found. Expected {} or output.o\n\
             Run 'clorus build' first.",
            target_output_path.display()
        ));
    };

    let dest_path = temp_dir.join("lib").join(format!("{}.o", package_name));
    fs::copy(&source_path, &dest_path)
        .map_err(|e| format!("Failed to copy .o file: {}", e))?;
    println!("      ✓ Packaged {}.o", package_name);

    // Look for .bc (bitcode) file if it exists
    let target_bc_path = PathBuf::from("target").join(format!("{}.bc", package_name));
    let bc_path = Path::new("output.bc");

    if target_bc_path.exists() {
        let dest_path = temp_dir.join("lib").join(format!("{}.bc", package_name));
        fs::copy(&target_bc_path, &dest_path)
            .map_err(|e| format!("Failed to copy .bc file: {}", e))?;
        println!("      ✓ Packaged {}.bc (portable bitcode)", package_name);
    } else if bc_path.exists() {
        let dest_path = temp_dir.join("lib").join(format!("{}.bc", package_name));
        fs::copy(bc_path, &dest_path)
            .map_err(|e| format!("Failed to copy .bc file: {}", e))?;
        println!("      ✓ Packaged {}.bc (portable bitcode)", package_name);
    }

    Ok(())
}

/// Extract API exports from compiled code
/// For now, create a basic exports.json structure
/// TODO: Parse actual symbols from .o file
fn create_exports_json(package_name: &str, temp_dir: &Path) -> Result<(), String> {
    // TODO: In a complete implementation, we would:
    // 1. Parse the .o file to extract symbol names
    // 2. Demangle Clorus function names
    // 3. Extract arity information
    //
    // For now, create a minimal valid exports.json
    let exports = json!({
        "version": "1.0.0",
        "exports": {},
        "note": "API exports will be automatically extracted in future versions"
    });

    let exports_path = temp_dir.join("api").join("exports.json");
    let exports_json = serde_json::to_string_pretty(&exports)
        .map_err(|e| format!("Failed to serialize exports: {}", e))?;

    fs::write(&exports_path, exports_json)
        .map_err(|e| format!("Failed to write exports.json: {}", e))?;

    Ok(())
}

/// Create ZIP archive from temp directory
fn create_zip_archive(temp_dir: &Path, output_filename: &str) -> Result<(), String> {
    let file = File::create(output_filename)
        .map_err(|e| format!("Failed to create output file: {}", e))?;

    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // Add all files from temp directory
    add_directory_to_zip(&mut zip, temp_dir, temp_dir, &options)?;

    zip.finish()
        .map_err(|e| format!("Failed to finalize ZIP: {}", e))?;

    Ok(())
}

/// Recursively add directory contents to ZIP archive
fn add_directory_to_zip<W: Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    base_dir: &Path,
    current_dir: &Path,
    options: &zip::write::FileOptions,
) -> Result<(), String> {
    let entries = fs::read_dir(current_dir)
        .map_err(|e| format!("Failed to read directory: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();
        let relative_path = path.strip_prefix(base_dir)
            .map_err(|e| format!("Failed to get relative path: {}", e))?;

        if path.is_dir() {
            // Add directory
            let dir_name = relative_path.to_str()
                .ok_or_else(|| "Invalid path".to_string())?;
            zip.add_directory(format!("{}/", dir_name), *options)
                .map_err(|e| format!("Failed to add directory to ZIP: {}", e))?;

            // Recurse
            add_directory_to_zip(zip, base_dir, &path, options)?;
        } else {
            // Add file
            let file_name = relative_path.to_str()
                .ok_or_else(|| "Invalid path".to_string())?;

            zip.start_file(file_name, *options)
                .map_err(|e| format!("Failed to start file in ZIP: {}", e))?;

            let mut file = File::open(&path)
                .map_err(|e| format!("Failed to open file: {}", e))?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)
                .map_err(|e| format!("Failed to read file: {}", e))?;

            zip.write_all(&buffer)
                .map_err(|e| format!("Failed to write file to ZIP: {}", e))?;
        }
    }

    Ok(())
}

/// Install a .clip package
pub fn install(clip_path: &str, local: bool) -> Result<(), String> {
    println!("📥 Installing .clip package...");
    println!("   📦 Source: {}", clip_path);

    let clip_path = Path::new(clip_path);
    if !clip_path.exists() {
        return Err(format!("File not found: {}", clip_path.display()));
    }

    // Determine installation directory
    let install_dir = if local {
        PathBuf::from("libs")
    } else {
        // Global installation (in user's home directory)
        let home = std::env::var("HOME")
            .map_err(|_| "HOME environment variable not set".to_string())?;
        PathBuf::from(home).join(".clorus").join("packages")
    };

    println!("   📁 Installing to: {}", install_dir.display());

    // Create installation directory
    fs::create_dir_all(&install_dir)
        .map_err(|e| format!("Failed to create installation directory: {}", e))?;

    // Copy .clip file to installation directory
    let filename = clip_path.file_name()
        .ok_or_else(|| "Invalid filename".to_string())?;
    let dest_path = install_dir.join(filename);

    fs::copy(clip_path, &dest_path)
        .map_err(|e| format!("Failed to copy .clip file: {}", e))?;

    println!("✅ Successfully installed {}", filename.to_string_lossy());
    println!();
    println!("Add to your Clorus.toml dependencies:");
    println!("    [dependencies]");
    println!("    package-name = {{ path = \"{}\" }}", dest_path.display());

    Ok(())
}
