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
        "build" => commands::build(),
        "run" => {
            // Check for flags
            let debug = args.iter().any(|arg| arg == "--debug" || arg == "-d");
            let use_jit = args.iter().any(|arg| arg == "--jit");

            // Collect arguments after "run" (excluding flags)
            let extra_args: Vec<String> = args[2..]
                .iter()
                .filter(|arg| *arg != "--debug" && *arg != "-d" && *arg != "--jit")
                .map(|s| s.clone())
                .collect();

            commands::run(debug, use_jit, extra_args)
        }
        "check" => commands::check(),
        "clean" => commands::clean(),
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
            // Parse --output flag
            let output = if args.len() >= 4 && args[2] == "--output" {
                Some(args[3].clone())
            } else {
                None
            };
            pack::pack(output)
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
    println!("    run           Compile and run the current project (like cargo run)");
    println!("                    --jit       Use JIT mode (faster, but no .clip support)");
    println!("    check         Check syntax without building");
    println!("    pack          Package project as .clip library");
    println!("                    --output <name>  Output file name");
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
    println!();
    println!("EXAMPLES:");
    println!("    clorus new my-project       Create a new project");
    println!("    clorus run                  Compile and run the project");
    println!("    clorus run --jit            Run with JIT (faster, no .clip dependencies)");
    println!("    clorus run arg1 arg2        Run with arguments passed to -main");
    println!("    clorus run --debug          Run with memory tracking");
    println!("    clorus pack                 Package as a .clip library");
    println!("    clorus check                Check for syntax errors");
    println!("    clorus clean                Remove build artifacts (target/)");
    println!();
    println!("See https://github.com/yourusername/clorus for more information");
}
