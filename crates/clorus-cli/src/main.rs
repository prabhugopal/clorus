mod commands;
mod manifest;
mod rust_ffi;
mod interface;
mod pack;

use std::env;

// Import Value FFI types from runtime
use clorus_runtime::value::Value;

// Force inclusion of runtime FFI symbols for JIT execution
// This prevents the linker from stripping them, making them visible to LLVM JIT
#[used]
static FORCE_LINK_AGENT: unsafe extern "C" fn(*mut Value) -> *mut Value = clorus_runtime::agent::clorus_agent;
#[used]
static FORCE_LINK_CHAN: unsafe extern "C" fn(i64) -> *mut Value = clorus_runtime::channel::clorus_chan;
#[used]
static FORCE_LINK_CHAN_PUT: unsafe extern "C" fn(*mut Value, *mut Value) -> *mut Value = clorus_runtime::channel::clorus_chan_put;
#[used]
static FORCE_LINK_CHAN_TAKE: unsafe extern "C" fn(*mut Value) -> *mut Value = clorus_runtime::channel::clorus_chan_take;
#[used]
static FORCE_LINK_CHAN_CLOSE: unsafe extern "C" fn(*mut Value) -> *mut Value = clorus_runtime::channel::clorus_chan_close;
#[used]
static FORCE_LINK_ALTS: unsafe extern "C" fn(*mut Value) -> *mut Value = clorus_runtime::channel::clorus_alts;
#[used]
static FORCE_LINK_GO: unsafe extern "C" fn(*mut Value, *mut Value) -> *mut Value = clorus_runtime::go_block::clorus_go;
#[used]
static FORCE_LINK_DERIVE: extern "C" fn(*mut Value, *mut Value) -> *mut Value = clorus_runtime::hierarchy::clorus_derive;
#[used]
static FORCE_LINK_UNDERIVE: extern "C" fn(*mut Value, *mut Value) -> *mut Value = clorus_runtime::hierarchy::clorus_underive;
#[used]
static FORCE_LINK_ISA_I32: extern "C" fn(*mut Value, *mut Value) -> i32 = clorus_runtime::hierarchy::clorus_isa_i32;
#[used]
static FORCE_LINK_PARENTS: extern "C" fn(*mut Value) -> *mut Value = clorus_runtime::hierarchy::clorus_parents;
#[used]
static FORCE_LINK_ANCESTORS: extern "C" fn(*mut Value) -> *mut Value = clorus_runtime::hierarchy::clorus_ancestors;
#[used]
static FORCE_LINK_DESCENDANTS: extern "C" fn(*mut Value) -> *mut Value = clorus_runtime::hierarchy::clorus_descendants;
#[used]
static FORCE_LINK_PROTOCOL_SATISFIES: extern "C" fn(*const std::ffi::c_char, *const std::ffi::c_char) -> i32 =
    clorus_runtime::protocols::clorus_protocol_satisfies_type_i32;
#[used]
static FORCE_LINK_GENSYM: extern "C" fn(*const std::ffi::c_char) -> *mut Value =
    clorus_runtime::value::clorus_gensym;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_help();
        return;
    }

    let result = match args[1].as_str() {
        "new" => {
            if args.len() < 3 {
                eprintln!("Error: 'clorus new' requires a project name");
                eprintln!("Usage: clorus new <name>");
                std::process::exit(1);
            }
            commands::new(&args[2])
        }
        "build" => {
            // Check for --workspace flag
            let is_workspace = args.iter().any(|arg| arg == "--workspace");
            let debug = args.iter().any(|arg| arg == "--debug" || arg == "-d");

            if is_workspace {
                commands::build_workspace(debug)
            } else {
                commands::build()
            }
        }
        "run" => {
            // Check for flags
            let debug = args.iter().any(|arg| arg == "--debug" || arg == "-d");
            // JIT is now the default execution path for `clorus run`.
            // `--legacy-run` forces the pre-JIT compile+execute path.
            let use_jit = !args.iter().any(|arg| arg == "--legacy-run");

            // Collect arguments after "run" (excluding flags)
            let extra_args: Vec<String> = args[2..]
                .iter()
                .filter(|arg| {
                    *arg != "--debug"
                        && *arg != "-d"
                        && *arg != "--jit"
                        && *arg != "--legacy-run"
                })
                .map(|s| s.clone())
                .collect();

            commands::run(debug, use_jit, extra_args)
        }
        "check" => commands::check(),
        "clean" => {
            // Check for --workspace flag
            let is_workspace = args.iter().any(|arg| arg == "--workspace");

            if is_workspace {
                commands::clean_workspace()
            } else {
                commands::clean()
            }
        }
        "repl" => {
            // Check for --main-thread flag
            let main_thread = args.iter().any(|arg| arg == "--main-thread");
            commands::repl(main_thread)
        }
        "replx" => {
            // Extended REPL with smart adaptive execution
            commands::replx(&args[2..])
        }
        "pack" => {
            // Check for --workspace flag
            let is_workspace = args.iter().any(|arg| arg == "--workspace");

            if is_workspace {
                // Parse --output flag for workspace pack
                let output = args.iter()
                    .position(|arg| arg == "--output")
                    .and_then(|i| args.get(i + 1))
                    .map(|s| s.clone());

                commands::pack_workspace(output)
            } else {
                // Parse --output flag for single pack
                let output = if args.len() >= 4 && args[2] == "--output" {
                    Some(args[3].clone())
                } else {
                    None
                };
                pack::pack(output)
            }
        }
        "install" => {
            if args.len() < 3 {
                eprintln!("Error: 'clorus install' requires a .clip file path");
                eprintln!("Usage: clorus install <path-to-clip>");
                std::process::exit(1);
            }
            let local = args.iter().any(|arg| arg == "--local");
            pack::install(&args[2], local)
        }
        "help" | "--help" | "-h" => {
            print_help();
            return;
        }
        "version" | "--version" | "-V" => {
            println!("clorus {}", env!("CARGO_PKG_VERSION"));
            return;
        }
        cmd => {
            eprintln!("Error: unknown command '{}'", cmd);
            eprintln!();
            eprintln!("Usage: clorus <command>");
            eprintln!();
            eprintln!("Run 'clorus help' for more information");
            std::process::exit(1);
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn print_help() {
    println!("Clorus {}", env!("CARGO_PKG_VERSION"));
    println!("A Clojure-inspired systems programming language");
    println!();
    println!("USAGE:");
    println!("    clorus <command> [options]");
    println!();
    println!("COMMANDS:");
    println!("    new <name>    Create a new Clorus project");
    println!("    build         Compile the current project");
    println!("                    --workspace  Build all workspace members");
    println!("    run           Run the current project (JIT by default)");
    println!("                    --legacy-run  Use legacy compile+run path");
    println!("    check         Check syntax without building");
    println!("    clean         Remove build artifacts");
    println!("                    --workspace  Clean all workspace members");
    println!("    pack          Package project as .clip library");
    println!("                    --workspace  Package all workspace libraries");
    println!("                    --output <dir>  Output directory (workspace only)");
    println!("    install       Install a .clip package");
    println!("                    --local  Install to project (default: global)");
    println!("    repl          Start an interactive REPL");
    println!("                    --main-thread  Run on main thread (for GUI on macOS)");
    println!("    replx         Start extended REPL (smart, adaptive)");
    println!("                    --main-thread  Run on main thread");
    println!("                    --warn         Warn only, don't auto-fix");
    println!("                    --silent       Auto-fix silently");
    println!("                    --no-adapt     Disable adaptive features");
    println!("    help          Print this help message");
    println!("    version       Print version information");
    println!();
    println!("OPTIONS:");
    println!("    -h, --help       Print help information");
    println!("    -V, --version    Print version information");
    println!("    -d, --debug      Enable debug mode (memory tracking)");
    println!("    --workspace      Apply command to all workspace members");
    println!();
    println!("EXAMPLES:");
    println!("    clorus new my-project       Create a new project");
    println!("    clorus run                  Run with JIT (default)");
    println!("    clorus run --legacy-run     Run with legacy compile+execute path");
    println!("    clorus run arg1 arg2        Run with arguments passed to -main");
    println!("    clorus run --debug          Run with memory tracking");
    println!("    clorus pack                 Package as a .clip library");
    println!("    clorus check                Check for syntax errors");
    println!("    clorus clean                Remove build artifacts (target/)");
    println!();
    println!("WORKSPACE EXAMPLES:");
    println!("    clorus build --workspace    Build all workspace members");
    println!("    clorus pack --workspace     Package all libraries to dist/");
    println!("    clorus clean --workspace    Clean all workspace members");
    println!();
    println!("See https://github.com/yourusername/clorus for more information");
}
