use clorus_replx::{AdaptiveConfig, AdaptationLevel};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    // Parse arguments
    let mut config = AdaptiveConfig::default();

    for arg in &args[1..] {
        match arg.as_str() {
            "--no-adapt" => config.enabled = false,
            "--warn" => config.level = AdaptationLevel::Warn,
            "--silent" => config.level = AdaptationLevel::Silent,
            "--main-thread" => config.main_thread = true,
            "--no-gui-detach" => config.auto_detach_gui = false,
            "--help" | "-h" => {
                print_help();
                return;
            }
            _ => {
                eprintln!("Unknown option: {}", arg);
                eprintln!("Run 'replx --help' for usage");
                std::process::exit(1);
            }
        }
    }

    // Run extended REPL
    if let Err(e) = clorus_replx::run_with_config(config) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn print_help() {
    println!("replx - Extended Clorus REPL");
    println!("Smart, adaptive REPL that prevents blocking and auto-detects execution contexts");
    println!();
    println!("USAGE:");
    println!("    replx [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    --main-thread     Run on main thread (for GUI on macOS)");
    println!("    --no-adapt        Disable adaptive features");
    println!("    --warn            Warn about issues but don't auto-fix");
    println!("    --silent          Auto-adapt silently (no messages)");
    println!("    --no-gui-detach   Don't auto-detach GUI functions");
    println!("    -h, --help        Print this help");
    println!();
    println!("EXAMPLES:");
    println!("    replx                     # Smart REPL with auto-adapt");
    println!("    replx --main-thread       # For GUI development on macOS");
    println!("    replx --warn              # Warnings only, no auto-fix");
    println!();
    println!("ADAPTIVE FEATURES:");
    println!("  • Auto-detects GUI functions that would block");
    println!("  • Spawns them in detached context to keep REPL responsive");
    println!("  • Suggests optimizations for long-running operations");
    println!("  • Learns from your usage patterns (with --learn)");
    println!();
    println!("See: https://github.com/clorus/clorus for more info");
}
