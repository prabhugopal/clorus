use clorus_cli::{commands, pack};

use clap::{Parser, Subcommand, ValueEnum};
use std::process::ExitCode;

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

#[derive(Parser)]
#[command(
    name = "clorus",
    version,
    about = "A Clojure-inspired systems programming language",
    after_help = "See https://github.com/yourusername/clorus for more information"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Clone, Copy, ValueEnum)]
enum TemplateArg {
    Basic,
    RustInterop,
}

#[derive(Subcommand)]
enum Command {
    /// Create a new Clorus project
    #[command(after_help = "EXAMPLES:\n    clorus new my-project\n    clorus new my-app --rust-interop\n    clorus new my-app --template rust-interop")]
    New {
        /// Project name
        name: String,
        /// Project template
        #[arg(long, value_enum)]
        template: Option<TemplateArg>,
        /// Shortcut for --template rust-interop
        #[arg(long = "rust-interop")]
        rust_interop: bool,
    },
    /// Compile the current project
    Build {
        /// Build all workspace members
        #[arg(long)]
        workspace: bool,
        /// Enable debug mode (memory tracking)
        #[arg(short, long)]
        debug: bool,
    },
    /// Run the current project (JIT by default)
    #[command(after_help = "EXAMPLES:\n    clorus run                  Run with JIT (default)\n    clorus run --legacy-run     Run with legacy compile+execute path\n    clorus run arg1 arg2        Run with arguments passed to -main\n    clorus run --debug          Run with memory tracking")]
    Run {
        /// Enable debug mode (memory tracking)
        #[arg(short, long)]
        debug: bool,
        /// Use legacy compile+run path instead of JIT
        #[arg(long = "legacy-run")]
        legacy_run: bool,
        /// Force JIT (the default; kept for backward compatibility)
        #[arg(long)]
        jit: bool,
        /// Entry file override and/or arguments passed to -main
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Check syntax without building
    Check,
    /// Remove build artifacts (target/)
    Clean {
        /// Clean all workspace members
        #[arg(long)]
        workspace: bool,
    },
    /// Start an interactive REPL
    Repl {
        /// Run on main thread (for GUI on macOS)
        #[arg(long = "main-thread")]
        main_thread: bool,
    },
    /// Start extended REPL (smart, adaptive)
    Replx {
        /// --main-thread, --warn, --silent, --no-adapt, --no-gui-detach
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Package project as a .clip library
    #[command(after_help = "WORKSPACE EXAMPLES:\n    clorus pack --workspace     Package all libraries to dist/")]
    Pack {
        /// Package all workspace libraries
        #[arg(long)]
        workspace: bool,
        /// Output directory (workspace only)
        #[arg(long)]
        output: Option<String>,
    },
    /// Install a .clip package
    Install {
        /// Path to the .clip file
        path: String,
        /// Install to project (default: global)
        #[arg(long)]
        local: bool,
    },
    /// Print version information
    Version,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let command = match cli.command {
        Some(command) => command,
        None => {
            let _ = <Cli as clap::CommandFactory>::command().print_help();
            println!();
            return ExitCode::SUCCESS;
        }
    };

    let result = match command {
        Command::New {
            name,
            template,
            rust_interop,
        } => {
            let resolved_template = match template {
                Some(TemplateArg::Basic) => commands::NewTemplate::Basic,
                Some(TemplateArg::RustInterop) => commands::NewTemplate::RustInterop,
                None if rust_interop => commands::NewTemplate::RustInterop,
                None => commands::NewTemplate::Basic,
            };
            commands::new_with_template(&name, resolved_template)
        }
        Command::Build { workspace, debug } => {
            if workspace {
                commands::build_workspace(debug)
            } else {
                commands::build()
            }
        }
        Command::Run {
            debug,
            legacy_run,
            jit: _,
            args,
        } => {
            // JIT is the default execution path; --legacy-run forces the
            // pre-JIT compile+execute path. --jit is accepted for backward
            // compatibility but is a no-op (it was always the default).
            commands::run(debug, !legacy_run, args)
        }
        Command::Check => commands::check(),
        Command::Clean { workspace } => {
            if workspace {
                commands::clean_workspace()
            } else {
                commands::clean()
            }
        }
        Command::Repl { main_thread } => commands::repl(main_thread),
        Command::Replx { args } => commands::replx(&args),
        Command::Pack { workspace, output } => {
            if workspace {
                commands::pack_workspace(output)
            } else {
                pack::pack(output)
            }
        }
        Command::Install { path, local } => pack::install(&path, local),
        Command::Version => {
            println!("clorus {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
