use std::fs;
use std::path::Path;

/// Scans this crate's own source for every `#[no_mangle] pub extern "C" fn`
/// and generates a function that takes all of their addresses, in
/// `$OUT_DIR/keep_alive_generated.rs` (included by `src/keep_alive.rs`).
///
/// Why: JIT-compiled Clorus code calls these functions by symbol name at
/// runtime, but nothing in this crate's *own* Rust code calls most of them
/// (e.g. `clorus_register_protocol_method` -- only ever invoked from
/// compiled `deftype`/`extend-type` forms). A normal executable link only
/// retains symbols reachable from some Rust-level call site; the linker
/// can't see JIT-generated machine code's calls, so it silently drops the
/// rest. `-rdynamic` (Linux) / Mach-O's default (macOS) only *export*
/// symbols that survive the link in the first place -- they can't export
/// what's already been eliminated.
///
/// This generated function makes every one of them genuinely Rust-reachable
/// (via `retain_all_runtime_symbols`, called once at process start by every
/// host binary that embeds a JIT engine -- see `run_jit_internal` in
/// `clorus-cli`), which is what actually keeps them in the final binary
/// under ordinary linking. Confirmed (via `nm`) to be necessary after a
/// `--whole-archive`/`-force_load` attempt at the *linker* level caused
/// duplicate-symbol errors against the crate's own normal rlib dependency
/// link -- this crate is linked twice either way (once for the symbols its
/// own downstream Rust code needs, once implicitly for everything else),
/// and reachability-based retention is the one approach that doesn't
/// require a second, conflicting copy of the same object code.
///
/// Auto-generated (not hand-maintained) so it can never silently drift out
/// of sync with the actual function list as new runtime functions are
/// added -- the exact risk a hand-written list would carry.
fn main() {
    println!("cargo:rerun-if-changed=src");

    let mut paths = Vec::new();
    collect_no_mangle_fn_paths(Path::new("src"), "crate", &mut paths);
    paths.sort();
    paths.dedup();

    let mut out = String::new();
    out.push_str("/// Takes the address of every `#[no_mangle] pub extern \"C\" fn` in this\n");
    out.push_str("/// crate, making all of them Rust-reachable. See build.rs for why.\n");
    out.push_str("pub fn retain_all_runtime_symbols() -> usize {\n");
    out.push_str("    let mut acc: usize = 0;\n");
    for path in &paths {
        out.push_str(&format!("    acc ^= {} as *const () as usize;\n", path));
    }
    out.push_str("    acc\n");
    out.push_str("}\n");

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest = Path::new(&out_dir).join("keep_alive_generated.rs");
    fs::write(&dest, out).expect("failed to write keep_alive_generated.rs");
}

/// Walks `dir`, deriving each file's Rust module path from its location
/// under `src/` (`src/lib.rs` -> the crate root; `src/foo.rs` -> `foo`;
/// `src/foo/mod.rs` -> `foo`; `src/foo/bar.rs` -> `foo::bar`), and collects
/// the fully-qualified path of every `#[no_mangle] pub extern "C" fn` found.
fn collect_no_mangle_fn_paths(dir: &Path, module_path: &str, paths: &mut Vec<String>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();

        if path.is_dir() {
            let child_module_path = format!("{}::{}", module_path, stem);
            collect_no_mangle_fn_paths(&path, &child_module_path, paths);
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };

        let file_module_path = if stem == "lib" || stem == "mod" {
            module_path.to_string()
        } else {
            format!("{}::{}", module_path, stem)
        };

        let mut prev_was_no_mangle = false;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed == "#[no_mangle]" {
                prev_was_no_mangle = true;
                continue;
            }
            if prev_was_no_mangle {
                if let Some(name) = extract_extern_c_fn_name(trimmed) {
                    paths.push(format!("{}::{}", file_module_path, name));
                }
                prev_was_no_mangle = false;
            }
        }
    }
}

/// Extracts `foo` from a line like `pub extern "C" fn foo(...) -> ... {`.
/// Only matches crate-root-visible items (`pub`, not `pub(crate)` etc. --
/// those can't be reached via `crate::name` from the generated function if
/// they're nested in a private module, but every symbol JIT code calls by
/// name must already be `pub extern "C"` to be linkable at all).
fn extract_extern_c_fn_name(line: &str) -> Option<String> {
    let marker = "pub extern \"C\" fn ";
    let idx = line.find(marker)?;
    let rest = &line[idx + marker.len()..];
    let name_end = rest.find(['(', '<', ' '])?;
    let name = &rest[..name_end];
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}
